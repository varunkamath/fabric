import asyncio
import logging
from typing import Dict, Optional, Callable, Any
from zenoh import Session, Subscriber, Publisher, Sample
from .interface import NodeInterface, NodeConfig
import json

# Set up logging
logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)


class Node:
    def __init__(
        self, node_id: str, node_type: str, config: NodeConfig, session: Session
    ):
        self.id = node_id
        self.node_type = node_type
        self.config = config
        self.session = session
        self.interface: Optional[NodeInterface] = None
        self.publishers: Dict[str, Publisher] = {}
        self.subscribers: Dict[str, Subscriber] = {}

    async def run(self, cancel_token: asyncio.Event) -> None:
        logger.info(f"Node {self.id} starting...")
        try:
            await self.initialize()
            if self.interface:
                await self.interface.run(self, cancel_token)
        except Exception as e:
            logger.error(f"Error in node {self.id}: {str(e)}", exc_info=True)
        finally:
            logger.info(f"Node {self.id} shutting down...")
            await self.cleanup()
            logger.info(f"Node {self.id} cleaned up successfully")

    async def initialize(self) -> None:
        logger.info(f"Node {self.id} initialized with type {self.node_type}")
        try:
            await self.create_publisher(f"node/{self.id}/data")
            await self.create_subscriber(
                f"node/{self.id}/config", self.handle_config_update
            )
        except Exception as e:
            logger.error(f"Error initializing node {self.id}: {str(e)}", exc_info=True)
            raise

    async def cleanup(self) -> None:
        for publisher in self.publishers.values():
            publisher.undeclare()
        for subscriber in self.subscribers.values():
            subscriber.undeclare()

    async def create_publisher(self, topic: str) -> None:
        self.publishers[topic] = self.session.declare_publisher(topic)

    async def create_subscriber(
        self, topic: str, callback: Callable[[Sample], None]
    ) -> None:
        self.subscribers[topic] = self.session.declare_subscriber(topic, callback)

    async def publish(self, topic: str, data: Any) -> None:
        if topic in self.publishers:
            await self.publishers[topic].put(data)
        else:
            logger.warning(f"Attempted to publish to non-existent topic: {topic}")

    async def handle_config_update(self, sample: Sample) -> None:
        if self.interface:
            config_data = json.loads(sample.payload.decode())
            new_config = NodeConfig(**config_data)
            await self.interface.update_config(new_config)

    async def get_config(self) -> NodeConfig:
        if self.interface:
            return await self.interface.get_config()
        return self.config

    async def set_config(self, config: NodeConfig) -> None:
        self.config = config
        if self.interface:
            await self.interface.set_config(config)

    def get_type(self) -> str:
        return self.node_type

    async def handle_event(self, event: str, payload: dict) -> None:
        if self.interface:
            await self.interface.handle_event(event, payload)
