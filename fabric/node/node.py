import asyncio
from typing import Dict, Optional, Callable, Any
from zenoh import Session, Subscriber, Publisher, Sample
from .interface import NodeInterface, NodeConfig, NodeData
from ..error import FabricError, PublisherNotFoundError

class Node:
    def __init__(self, node_id: str, node_type: str, config: NodeConfig, session: Session):
        self.id = node_id
        self.node_type = node_type
        self.config = config
        self.session = session
        self.interface: Optional[NodeInterface] = None
        self.publishers: Dict[str, Publisher] = {}
        self.subscribers: Dict[str, Subscriber] = {}

    async def run(self, cancel_token: asyncio.Event) -> None:
        while not cancel_token.is_set():
            # Implement node logic here
            await asyncio.sleep(1)

    async def create_publisher(self, topic: str) -> None:
        self.publishers[topic] = await self.session.declare_publisher(topic)

    async def create_subscriber(self, topic: str, callback: Callable[[Sample], None]) -> None:
        self.subscribers[topic] = await self.session.declare_subscriber(topic, callback)

    async def publish(self, topic: str, data: Any) -> None:
        if topic not in self.publishers:
            raise PublisherNotFoundError(f"Publisher for topic '{topic}' not found")
        await self.publishers[topic].put(data)

    async def get_config(self) -> NodeConfig:
        return self.config

    async def set_config(self, config: NodeConfig) -> None:
        self.config = config
        if self.interface:
            await self.interface.set_config(config)

    def get_type(self) -> str:
        return self.node_type

    async def handle_event(self, event: str, payload: str) -> None:
        if self.interface:
            await self.interface.handle_event(event, payload)

    async def update_config(self, config: NodeConfig) -> None:
        await self.set_config(config)