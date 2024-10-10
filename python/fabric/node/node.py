import asyncio
import logging
from typing import Dict, Optional, Callable, Any
from zenoh import Session, Subscriber, Publisher, Sample
from .interface import NodeInterface, NodeConfig, NodeData
import time
import json

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
        try:
            await self.initialize()
            while not cancel_token.is_set():
                await self.process_messages()
                await self.update_state()
                await asyncio.sleep(0.1)  # Adjust sleep time as needed
        except Exception as e:
            logger.error(f"Error in node {self.id}: {e}", exc_info=True)
        finally:
            self.cleanup()  # Note: This is now synchronous

    async def initialize(self) -> None:
        try:
            await self.create_publisher(f"node/{self.id}/data")
            await self.create_subscriber(
                f"node/{self.id}/config", self.handle_config_update
            )
            logger.info(f"Node {self.id} initialized successfully")
        except Exception as e:
            logger.error(f"Error initializing node {self.id}: {e}", exc_info=True)
            raise

    async def process_messages(self) -> None:
        # This method will be called in the run loop to process any pending messages
        pass

    async def update_state(self) -> None:
        try:
            if self.interface:
                node_data = NodeData(
                    node_id=self.id,
                    node_type=self.node_type,
                    timestamp=int(time.time()),
                    metadata=None,
                    status="online",
                )
                await self.publish(f"node/{self.id}/data", node_data.to_json())
        except Exception as e:
            logger.error(f"Error updating state for node {self.id}: {e}", exc_info=True)

    def cleanup(self) -> None:
        try:
            for publisher in self.publishers.values():
                publisher.undeclare()
            for subscriber in self.subscribers.values():
                subscriber.undeclare()
            logger.info(f"Node {self.id} cleaned up successfully")
        except Exception as e:
            logger.error(f"Error during cleanup for node {self.id}: {e}", exc_info=True)

    def create_publisher(self, topic: str) -> None:
        self.publishers[topic] = self.session.declare_publisher(topic)

    def create_subscriber(self, topic: str, callback: Callable[[Sample], None]) -> None:
        self.subscribers[topic] = self.session.declare_subscriber(topic, callback)

    def publish(self, topic: str, data: Any) -> None:
        if topic not in self.publishers:
            self.create_publisher(topic)
        try:
            self.publishers[topic].put(data)
        except Exception as e:
            logger.error(f"Error publishing to topic {topic}: {e}", exc_info=True)
            raise

    async def get_config(self) -> NodeConfig:
        return self.config

    async def set_config(self, config: NodeConfig) -> None:
        self.config = config
        if self.interface:
            try:
                await self.interface.set_config(config)
                logger.info(f"Config updated for node {self.id}")
            except Exception as e:
                logger.error(
                    f"Error setting config for node {self.id}: {e}", exc_info=True
                )
                raise

    def get_type(self) -> str:
        return self.node_type

    async def handle_event(self, event: str, payload: str) -> None:
        if self.interface:
            try:
                await self.interface.handle_event(event, payload)
                logger.debug(f"Handled event {event} for node {self.id}")
            except Exception as e:
                logger.error(
                    f"Error handling event {event} for node {self.id}: {e}",
                    exc_info=True,
                )
                raise

    async def update_config(self, config: NodeConfig) -> None:
        await self.set_config(config)

    async def handle_config_update(self, sample: Sample) -> None:
        try:
            config_data = sample.payload.decode("utf-8")
            new_config = NodeConfig(**json.loads(config_data))
            await self.update_config(new_config)
            logger.info(f"Config updated for node {self.id}")
        except Exception as e:
            logger.error(
                f"Error handling config update for node {self.id}: {e}", exc_info=True
            )
            raise
