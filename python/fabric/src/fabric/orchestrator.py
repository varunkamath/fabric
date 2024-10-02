import asyncio
import json
import time
from typing import Dict, Any, Callable
import zenoh
from .error import FabricError
from .node.interface import NodeConfig, NodeData

class NodeState:
    def __init__(self, node_data: NodeData):
        self.last_value = node_data
        self.last_update = time.time()

class Orchestrator:
    def __init__(self, orchestrator_id: str, session: zenoh.Session):
        self.orchestrator_id = orchestrator_id
        self.session = session
        self.node_states: Dict[str, NodeState] = {}
        self.callbacks: Dict[str, Callable[[NodeData], None]] = {}
        self.subscribers: Dict[str, zenoh.Subscriber] = {}

    async def run(self, cancellation_token: asyncio.Event):
        try:
            self.subscribe_to_node_statuses()
            
            async def check_offline_nodes():
                while not cancellation_token.is_set():
                    self.check_offline_nodes()
                    await asyncio.sleep(1)

            offline_check_task = asyncio.create_task(check_offline_nodes())

            await cancellation_token.wait()
        except asyncio.CancelledError:
            # Handle cancellation
            pass
        finally:
            offline_check_task.cancel()
            await self.unsubscribe_from_node_statuses()

    def subscribe_to_node_statuses(self):
        self.subscribers['status'] = self.session.declare_subscriber("fabric/*/status", self.update_node_health)

    async def unsubscribe_from_node_statuses(self):
        if 'status' in self.subscribers:
            await self.subscribers['status'].undeclare()

    def update_node_health(self, sample: zenoh.Sample):
        key_expr = str(sample.key_expr)
        node_id = key_expr.split('/')[1]
        try:
            node_data = NodeData.from_dict(json.loads(sample.payload.decode()))
            self.node_states[node_id] = NodeState(node_data)
            if node_data.status != "online":
                print(f"Node {node_id} is {node_data.status}")
            if node_id in self.callbacks:
                self.callbacks[node_id](node_data)
        except Exception as e:
            print(f"Failed to parse NodeData from JSON for node {node_id}: {e}")

    async def publish_node_config(self, node_id: str, config: NodeConfig):
        topic = f"node/{node_id}/config"
        payload = json.dumps(config.config)
        await self.session.put(topic, payload)

    def update_node_state(self, node_data: NodeData):
        self.node_states[node_data.node_id] = NodeState(node_data)
        if node_data.node_id in self.callbacks:
            self.callbacks[node_data.node_id](node_data)

    def check_offline_nodes(self):
        current_time = time.time()
        for node_id, node_state in self.node_states.items():
            if node_state.last_value.status == "online":
                if current_time - node_state.last_update > 10:
                    print(f"Node {node_id} has not sent a status update in 10 seconds, marking as offline")
                    node_state.last_value.set_status("offline")
                    if node_id in self.callbacks:
                        self.callbacks[node_id](node_state.last_value)

    def get_id(self) -> str:
        return self.orchestrator_id

    def register_callback(self, node_id: str, callback: Callable[[NodeData], None]):
        self.callbacks[node_id] = callback

    async def send_event_to_node(self, node_id: str, event: str, payload: str):
        topic = f"node/{node_id}/event/{event}"
        await self.session.put(topic, payload)

    @classmethod
    async def create(cls, orchestrator_id: str, session: zenoh.Session):
        return cls(orchestrator_id, session)