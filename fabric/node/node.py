import asyncio
from typing import Dict, Optional, Callable, Any
from zenoh import Session, Subscriber, Publisher, Sample
from .interface import NodeInterface, NodeConfig, NodeData
from ..error import FabricError, PublisherNotFoundError

class Node:
    def __init__(self, node_id: str, node_type: str, config: NodeConfig, session: Session, interface: Optional[NodeInterface] = None):
        self.id = node_id
        self.node_type = node_type
        self.config = config
        self.session = session
        self.interface = interface or GenericNode(config)
        self.publishers: Dict[str, Publisher] = {}
        self.subscribers: Dict[str, Subscriber] = {}

    async def run(self, cancel_token: asyncio.Event) -> None:
        config_subscriber = self.session.declare_subscriber(f"node/{self.id}/config", lambda sample: self.handle_config_update(sample))
        
        # Initial status update
        await self.update_status("online")

        async def status_update_task():
            while not cancel_token.is_set():
                await self.update_status("online")
                await asyncio.sleep(1)

        status_task = asyncio.create_task(status_update_task())

        try:
            async for sample in config_subscriber.receiver():
                if cancel_token.is_set():
                    break
                new_config = NodeConfig(**sample.payload)
                await self.update_config(new_config)
        finally:
            status_task.cancel()
            await status_task

    async def update_config(self, new_config: NodeConfig) -> None:
        await self.interface.update_config(new_config)
        self.config = new_config

    async def get_config(self) -> NodeConfig:
        return self.config

    def get_id(self) -> str:
        return self.id

    def get_type(self) -> str:
        return self.node_type

    async def update_status(self, status: str) -> None:
        node_data = NodeData(
            node_id=self.id,
            node_type=self.node_type,
            status=status,
            timestamp=int(asyncio.get_event_loop().time()),
            metadata=None
        )
        await self.publish_node_status(node_data)

    async def publish_node_status(self, node_data: NodeData) -> None:
        key_expr = f"fabric/{self.id}/status"
        payload = node_data.to_json()
        await self.session.put(key_expr, payload)

    def create_publisher(self, topic: str) -> None:
        self.publishers[topic] = self.session.declare_publisher(topic)

    def publish(self, topic: str, data: bytes) -> None:
        if topic not in self.publishers:
            raise PublisherNotFoundError(f"Publisher not found for topic: {topic}")
        self.publishers[topic].put(data)

    def create_subscriber(self, topic: str, callback: Callable[[Sample], None]) -> None:
        self.subscribers[topic] = self.session.declare_subscriber(topic, callback)

    def handle_config_update(self, sample: Sample):
        new_config = NodeConfig(**sample.payload)
        asyncio.create_task(self.update_config(new_config))

class GenericNode(NodeInterface):
    def __init__(self, config: NodeConfig):
        self.config = config

    def get_config(self) -> NodeConfig:
        return self.config

    async def set_config(self, config: NodeConfig) -> None:
        self.config = config

    def get_type(self) -> str:
        return "generic"

    async def handle_event(self, event: str, payload: str) -> None:
        pass

    async def update_config(self, config: NodeConfig) -> None:
        self.config = config