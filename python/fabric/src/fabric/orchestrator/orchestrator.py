import asyncio
from typing import Dict, Any, Optional
from zenoh import Zenoh, Config, Sample
import json
import logging
from datetime import datetime, timedelta

class Orchestrator:
    def __init__(self, id: str, session: Zenoh):
        self.id = id
        self.session = session
        self.nodes = {}
        self.callbacks = {}
        self.subscribers = {}
        self.publishers = {}
        self.status_subscriber = None
        self.subscriber_queue = asyncio.Queue()

    async def run(self, cancel_event: asyncio.Event) -> None:
        logging.info(f"Starting orchestrator: {self.id}")
        await self.subscribe_to_node_statuses()

        while not cancel_event.is_set():
            try:
                sample = await asyncio.wait_for(self.subscriber_queue.get(), timeout=1.0)
                await self.update_node_health(sample)
            except asyncio.TimeoutError:
                pass

        logging.info(f"Orchestrator {self.id} shutting down")
        await self.unsubscribe_from_node_statuses()

    async def subscribe_to_node_statuses(self) -> None:
        self.status_subscriber = await self.session.declare_subscriber("fabric/*/status", self.subscriber_queue.put)

    async def unsubscribe_from_node_statuses(self) -> None:
        if self.status_subscriber:
            await self.status_subscriber.undeclare()

    async def update_node_health(self, sample: Sample) -> None:
        node_id = sample.key_expr.split('/')[1]
        payload = sample.payload.decode('utf-8')
        node_data = json.loads(payload)
        self.nodes[node_id] = {
            "last_value": node_data,
            "last_update": datetime.now()
        }
        if node_data["status"] != "online":
            logging.warning(f"Node {node_id} is {node_data['status']}")
        if node_id in self.callbacks:
            await self.callbacks[node_id](node_data)

    async def publish_node_config(self, node_id: str, config: Dict[str, Any]) -> None:
        key = f"node/{node_id}/config"
        payload = json.dumps(config).encode('utf-8')
        await self.session.put(key, payload)

    async def register_callback(self, node_id: str, callback) -> None:
        self.callbacks[node_id] = callback

    async def check_offline_nodes(self) -> None:
        now = datetime.now()
        for node_id, node_state in self.nodes.items():
            if node_state["last_value"]["status"] == "online" and now - node_state["last_update"] > timedelta(seconds=10):
                logging.warning(f"Node {node_id} has not sent a status update in 10 seconds, marking as offline")
                node_state["last_value"]["status"] = "offline"
                if node_id in self.callbacks:
                    await self.callbacks[node_id](node_state["last_value"])