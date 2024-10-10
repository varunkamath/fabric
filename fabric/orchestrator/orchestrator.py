import asyncio
from typing import Dict, Callable
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
        self.publishers: Dict[str, Publisher] = {}
        self.subscribers: Dict[str, Subscriber] = {}

    async def run(self, cancel_token: asyncio.Event) -> None:
        while not cancel_token.is_set():
            # Implement orchestrator logic here
            await asyncio.sleep(1)

    async def create_publisher(self, topic: str) -> None:
        self.publishers[topic] = await self.session.declare_publisher(topic)

    async def create_subscriber(
        self, topic: str, callback: Callable[[Sample], None]
    ) -> None:
        self.subscribers[topic] = await self.session.declare_subscriber(topic, callback)

    async def publish_node_config(self, node_id: str, config: NodeConfig) -> None:
        topic = f"node/{node_id}/config"
        if topic not in self.publishers:
            await self.create_publisher(topic)
        await self.publishers[topic].put(config.config)

    async def update_node_state(self, node_data: NodeData) -> None:
        self.nodes[node_data.node_id] = NodeState(node_data)

    async def get_node_state(self, node_id: str) -> NodeState:
        if node_id not in self.nodes:
            raise FabricError(f"Node {node_id} not found")
        return self.nodes[node_id]
