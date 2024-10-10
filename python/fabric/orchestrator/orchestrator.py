import asyncio
import logging
from typing import Dict, Callable, Any
from zenoh import Session, Subscriber, Publisher, Sample
from ..node.interface import NodeConfig, NodeData
from ..error import FabricError
import time
import json
import backoff
from datetime import datetime, timedelta
from collections import deque

logger = logging.getLogger(__name__)


class NodeState:
    def __init__(self, node_data: NodeData):
        self.last_value = node_data
        self.last_update = time.time()


class Orchestrator:
    def __init__(self, orchestrator_id: str, session: Session):
        self.id = orchestrator_id
        self.session = session
        self.nodes: Dict[str, NodeState] = {}
        self.publishers: Dict[str, Publisher] = {}
        self.subscribers: Dict[str, Subscriber] = {}
        self.callbacks: Dict[str, Callable[[NodeData], None]] = {}
        self.update_queue = deque()
        self.event_queue = asyncio.Queue()

    async def run(self, cancel_token: asyncio.Event) -> None:
        try:
            await self.initialize()
            while not cancel_token.is_set():
                await self.process_node_updates()
                await self.process_events()
                await self.check_node_health()
                await asyncio.sleep(0.1)  # Adjust sleep time as needed
        except Exception as e:
            logger.error(f"Error in orchestrator {self.id}: {e}", exc_info=True)
        finally:
            await self.cleanup()  # Change this to await

    async def initialize(self) -> None:
        try:
            for node_id in self.nodes.keys():
                await self.create_subscriber(
                    f"node/{node_id}/data", self.handle_node_data
                )
            logger.info(f"Orchestrator {self.id} initialized successfully")
        except Exception as e:
            logger.error(
                f"Error initializing orchestrator {self.id}: {e}", exc_info=True
            )
            raise

    async def process_node_updates(self) -> None:
        while self.update_queue:
            node_data = self.update_queue.popleft()
            self.nodes[node_data.node_id] = NodeState(node_data)
            if node_data.node_id in self.callbacks:
                try:
                    await self.callbacks[node_data.node_id](node_data)
                    logger.debug(f"Updated state for node {node_data.node_id}")
                except Exception as e:
                    logger.error(
                        f"Error in callback for node {node_data.node_id}: {e}",
                        exc_info=True,
                    )

    async def check_node_health(self) -> None:
        current_time = datetime.now()
        for node_id, state in self.nodes.items():
            last_update = datetime.fromtimestamp(state.last_update)
            if current_time - last_update > timedelta(seconds=10):
                logger.warning(f"Node {node_id} appears to be offline")
                state.last_value.set_status("offline")
                await self.update_node_state(state.last_value)
                await self.request_node_update(node_id)

    async def cleanup(self):  # Change this to async
        try:
            for publisher in self.publishers.values():
                publisher.undeclare()
            for subscriber in self.subscribers.values():
                subscriber.undeclare()
            logger.info(f"Orchestrator {self.id} cleaned up successfully")
        except Exception as e:
            logger.error(
                f"Error during cleanup for orchestrator {self.id}: {e}", exc_info=True
            )

    def create_publisher(self, topic: str) -> None:
        if topic not in self.publishers:
            self.publishers[topic] = self.session.declare_publisher(topic)

    async def create_subscriber(
        self, topic: str, callback: Callable[[Sample], None]
    ) -> None:
        try:
            self.subscribers[topic] = await self.session.declare_subscriber(
                topic, callback
            )
            logger.debug(f"Created subscriber for topic: {topic}")
        except Exception as e:
            logger.error(
                f"Error creating subscriber for topic {topic}: {e}", exc_info=True
            )
            raise

    async def publish_node_config(self, node_id: str, config: NodeConfig) -> None:
        topic = f"node/{node_id}/config"
        try:
            if topic not in self.publishers:
                self.create_publisher(topic)
            await self.publish_with_retry(
                topic, json.dumps(config.__dict__)
            )  # Add await here
            logger.info(f"Published config for node {node_id}")
        except Exception as e:
            logger.error(
                f"Error publishing config for node {node_id}: {e}", exc_info=True
            )
            raise

    async def update_node_state(self, node_data: NodeData) -> None:
        self.update_queue.append(node_data)
        self.nodes[node_data.node_id] = NodeState(node_data)
        if node_data.node_id in self.callbacks:
            try:
                self.callbacks[node_data.node_id](node_data)
                logger.debug(f"Updated state for node {node_data.node_id}")
            except Exception as e:
                logger.error(
                    f"Error in callback for node {node_data.node_id}: {e}",
                    exc_info=True,
                )

    async def get_node_state(self, node_id: str) -> NodeState:
        if node_id not in self.nodes:
            logger.error(f"Node {node_id} not found")
            raise FabricError(f"Node {node_id} not found")
        return self.nodes[node_id]

    async def handle_node_data(self, sample: Sample) -> None:
        try:
            node_data = NodeData.from_json(sample.payload.decode("utf-8"))
            await self.update_node_state(node_data)
        except Exception as e:
            logger.error(f"Error handling node data: {e}", exc_info=True)

    async def register_callback(
        self, node_id: str, callback: Callable[[NodeData], None]
    ) -> None:
        self.callbacks[node_id] = callback
        logger.debug(f"Registered callback for node {node_id}")

    @backoff.on_exception(backoff.expo, Exception, max_tries=3)
    async def publish_with_backoff(self, topic: str, data: Any) -> None:
        await self.publishers[topic].put(data)
        logger.debug(f"Published data to topic: {topic}")

    async def get_all_node_states(self) -> Dict[str, NodeState]:
        return self.nodes

    async def remove_node(self, node_id: str) -> None:
        if node_id in self.nodes:
            del self.nodes[node_id]
            logger.info(f"Removed node {node_id} from orchestrator {self.id}")
        else:
            logger.warning(
                f"Attempted to remove non-existent node {node_id} from orchestrator {self.id}"
            )

    async def send_event_to_node(
        self, node_id: str, event: str, payload: Dict[str, Any]
    ) -> None:
        topic = f"node/{node_id}/events"
        if topic not in self.publishers:
            self.create_publisher(topic)
        await self.publish_with_retry(
            topic, json.dumps({"event": event, "payload": payload})
        )  # Add await here
        logger.info(f"Sent event '{event}' to node {node_id}")

    async def process_events(self) -> None:
        while not self.event_queue.empty():
            node_id, event, payload = await self.event_queue.get()
            topic = f"node/{node_id}/events"
            if topic not in self.publishers:
                await self.create_publisher(topic)

            event_data = json.dumps({"event": event, "payload": payload})
            await self.publish_with_retry(topic, event_data)
            logger.info(f"Sent event '{event}' to node {node_id}")

    async def get_node_data(self, node_id: str) -> NodeData:
        if node_id not in self.nodes:
            raise FabricError(f"Node {node_id} not found")
        return self.nodes[node_id].last_value

    async def request_node_update(self, node_id: str) -> None:
        await self.send_event_to_node(node_id, "request_update", {})
        logger.debug(f"Requested update from node {node_id}")

    @backoff.on_exception(backoff.expo, Exception, max_tries=3)
    async def publish_with_retry(self, topic: str, data: Any) -> None:
        if topic not in self.publishers:
            self.create_publisher(topic)
        self.publishers[topic].put(data)
