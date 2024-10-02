import asyncio
from typing import Dict, Callable, Any
from zenoh import Session, Subscriber, Publisher, Sample
from ..node.interface import NodeConfig, NodeData
from ..error import FabricError

class NodeState:
    def __init__(self, node_data: NodeData):
        self.last_value = node_data
        self.last_update = asyncio.get_event_loop().time()

class Orchestrator:
    def __init__(self, orchestrator_id: str, session: Session):
        self.id = orchestrator_id
        self.session = session
        self.nodes: Dict[str, NodeState] = {}
        self.callbacks: Dict[str, Callable[[NodeData], None]] = {}
        self.subscribers: Dict[str, Subscriber] = {}
        self.publishers: Dict[str, Publisher] = {}

    async def run(self, cancel_token: asyncio.Event) -> None:
        self.subscribe_to_node_statuses()  # Remove await

        async def check_offline_nodes():
            while not cancel_token.is_set():
                self.check_offline_nodes()  # Remove await
                await asyncio.sleep(1)

        offline_check_task = asyncio.create_task(check_offline_nodes())

        try:
            await cancel_token.wait()
        finally:
            offline_check_task.cancel()
            await self.unsubscribe_from_node_statuses()

    def subscribe_to_node_statuses(self) -> None:
        self.status_subscriber = self.session.declare_subscriber("fabric/*/status", self.update_node_health)

    async def unsubscribe_from_node_statuses(self) -> None:
        if hasattr(self, 'status_subscriber'):
            await self.status_subscriber.undeclare()

    def update_node_health(self, sample: Sample) -> None:
        key_expr = str(sample.key_expr)
        node_id = key_expr.split('/')[1]
        try:
            node_data = NodeData.from_json(sample.payload.decode())
            self.nodes[node_id] = NodeState(node_data)
            if node_data.status != "online":
                print(f"Node {node_id} is {node_data.status}")
            if node_id in self.callbacks:
                self.callbacks[node_id](node_data)
        except Exception as e:
            print(f"Failed to parse NodeData from JSON for node {node_id}: {e}")

    async def publish_node_config(self, node_id: str, config: NodeConfig) -> None:
        key = f"node/{node_id}/config"
        config_json = config.config  # Use the config attribute directly
        await self.session.put(key, config_json)

    def update_node_state(self, node_data: NodeData) -> None:
        self.nodes[node_data.node_id] = NodeState(node_data)
        if node_data.node_id in self.callbacks:
            self.callbacks[node_data.node_id](node_data)

    def check_offline_nodes(self) -> None:
        current_time = asyncio.get_event_loop().time()
        for node_id, node_state in self.nodes.items():
            if node_state.last_value.status == "online":
                if current_time - node_state.last_update > 10:
                    print(f"Node {node_id} has not sent a status update in 10 seconds, marking as offline")
                    node_state.last_value.status = "offline"
                    if node_id in self.callbacks:
                        self.callbacks[node_id](node_state.last_value)

    def get_id(self) -> str:
        return self.id

    def register_callback(self, node_id: str, callback: Callable[[NodeData], None]) -> None:
        self.callbacks[node_id] = callback

    def create_publisher(self, topic: str) -> None:
        self.publishers[topic] = self.session.declare_publisher(topic)

    def publish(self, topic: str, data: bytes) -> None:
        if topic not in self.publishers:
            raise FabricError(f"Publisher not found for topic: {topic}")
        self.publishers[topic].put(data)

    def create_subscriber(self, topic: str, callback: Callable[[Sample], None]) -> None:
        self.subscribers[topic] = self.session.declare_subscriber(topic, callback)