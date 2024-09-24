import asyncio
from asyncio import CancelledError
import json
from typing import Any, Callable, Dict
import time
import zenoh
from .error import FabricError
from .interface import NodeConfig, NodeData

class NodeState:
    def __init__(self, node_data: NodeData):
        self.last_value = node_data
        self.last_update = time.time()

class Orchestrator:
    def __init__(self, orchestrator_id: str, session: zenoh.Session):
        self.orchestrator_id = orchestrator_id
        self.session = session
        self.node_states: Dict[str, NodeState] = {}

    async def run(self, cancellation_token: asyncio.Event):
        try:
            while not cancellation_token.is_set():
                # Implement the main orchestrator logic here
                await asyncio.sleep(1)
        except CancelledError:
            # Handle cancellation
            pass

    async def update_node_state(self, node_data: NodeData):
        self.node_states[node_data.node_id] = NodeState(node_data)

    async def publish_node_config(self, node_id: str, config: NodeConfig):
        topic = f"node/{node_id}/config"
        publisher = await self.session.declare_publisher(topic)
        await publisher.put(json.dumps(config.dict()))
        await publisher.close()

    async def get_node_state(self, node_id: str) -> NodeState:
        return self.node_states.get(node_id)

    async def get_all_node_states(self) -> Dict[str, NodeState]:
        return self.node_states

    async def send_event_to_node(self, node_id: str, event: str, payload: str):
        topic = f"node/{node_id}/event/{event}"
        publisher = await self.session.declare_publisher(topic)
        await publisher.put(payload)
        await publisher.close()

    @classmethod
    async def create(cls, orchestrator_id: str, session: zenoh.Session):
        return cls(orchestrator_id, session)