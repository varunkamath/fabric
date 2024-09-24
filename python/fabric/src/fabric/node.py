import asyncio
from asyncio import CancelledError
import json
from typing import Any, Callable, Dict
import time
import zenoh
from .error import FabricError
from .interface import NodeConfig, NodeData, NodeInterface

class Node:
    def __init__(self, node_id: str, node_type: str, config: NodeConfig, session: zenoh.Session, interface: NodeInterface):
        self.node_id = node_id
        self.node_type = node_type
        self.config = config
        self.session = session
        self.interface = interface
        self.publishers: Dict[str, zenoh.Publisher] = {}
        self.subscribers: Dict[str, zenoh.Subscriber] = {}

    async def run(self, cancellation_token: asyncio.Event):
        try:
            while not cancellation_token.is_set():
                # Implement the main node logic here
                await asyncio.sleep(1)
        except CancelledError:
            # Handle cancellation
            pass
        finally:
            # Cleanup resources
            for publisher in self.publishers.values():
                await publisher.close()
            for subscriber in self.subscribers.values():
                await subscriber.close()

    async def get_config(self) -> NodeConfig:
        return await self.interface.get_config()

    async def set_config(self, config: NodeConfig):
        await self.interface.set_config(config)

    def get_type(self) -> str:
        return self.interface.get_type()

    async def create_publisher(self, topic: str) -> None:
        if topic not in self.publishers:
            self.publishers[topic] = await self.session.declare_publisher(topic)

    async def create_subscriber(self, topic: str, callback: Callable[[zenoh.Sample], None]) -> None:
        if topic not in self.subscribers:
            self.subscribers[topic] = await self.session.declare_subscriber(topic, callback)

    async def publish(self, topic: str, data: Any) -> None:
        if topic not in self.publishers:
            await self.create_publisher(topic)
        await self.publishers[topic].put(json.dumps(data))

    async def update_config(self, config: NodeConfig) -> None:
        await self.interface.update_config(config)

    async def get_state(self) -> NodeData:
        return await self.interface.get_state()

    async def handle_event(self, event: str, payload: str) -> None:
        await self.interface.handle_event(event, payload)

    @classmethod
    async def create(cls, node_id: str, node_type: str, config: NodeConfig, session: zenoh.Session, interface: NodeInterface):
        return cls(node_id, node_type, config, session, interface)