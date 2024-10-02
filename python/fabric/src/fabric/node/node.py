import asyncio
from typing import Dict, Any, Optional
from .interface import NodeInterface, NodeConfig, NodeData
from .generic import GenericNode
import zenoh
import json
import logging
import time

class Node:
    def __init__(self, id: str, node_type: str, config: NodeConfig, session: zenoh.Session, interface: Optional[NodeInterface] = None):
        self.id = id
        self.node_type = node_type
        self.config = config
        self.session = session
        self.interface = interface if interface else GenericNode(config)
        self.publishers: Dict[str, zenoh.Publisher] = {}
        self.subscribers: Dict[str, zenoh.Subscriber] = {}
        self.subscriber_queue = asyncio.Queue()

    async def run(self, cancel_event: asyncio.Event) -> None:
        logging.info(f"Starting node {self.id}")
        await self.update_status("online")

        while not cancel_event.is_set():
            try:
                sample = await asyncio.wait_for(self.subscriber_queue.get(), timeout=1.0)
                await self.handle_sample(sample)
            except asyncio.TimeoutError:
                pass

        logging.info(f"Node {self.id} stopped")

    async def update_status(self, status: str) -> None:
        node_data = NodeData(
            node_id=self.id,
            node_type=self.node_type,
            status=status,
            timestamp=int(time.time()),
            metadata=None
        )
        await self.publish_node_status(node_data)

    async def publish_node_status(self, node_data: NodeData) -> None:
        key_expr = f"fabric/{self.id}/status"
        payload = json.dumps(node_data.to_dict()).encode('utf-8')
        await self.session.put(key_expr, payload)

    async def create_publisher(self, topic: str) -> None:
        self.publishers[topic] = await self.session.declare_publisher(topic)

    async def publish(self, topic: str, data: bytes) -> None:
        if topic in self.publishers:
            await self.publishers[topic].put(data)
        else:
            raise ValueError(f"Publisher not found for topic: {topic}")

    async def create_subscriber(self, topic: str, callback) -> None:
        self.subscribers[topic] = await self.session.declare_subscriber(topic, callback)

    async def handle_sample(self, sample: zenoh.Sample) -> None:
        for subscriber in self.subscribers.values():
            if subscriber.key_expr.intersects(sample.key_expr):
                await subscriber.callback(sample)

    async def update_config(self, new_config: NodeConfig) -> None:
        await self.interface.update_config(new_config)
        self.config = new_config

    def get_config(self) -> NodeConfig:
        return self.config

    def get_id(self) -> str:
        return self.id

    def get_type(self) -> str:
        return self.node_type

    async def get_interface(self) -> NodeInterface:
        return self.interface

    async def set_interface(self, interface: NodeInterface) -> None:
        self.interface = interface

    @classmethod
    async def create(cls, id: str, node_type: str, config: NodeConfig, session: zenoh.Session, interface: Optional[NodeInterface] = None):
        return cls(id, node_type, config, session, interface)