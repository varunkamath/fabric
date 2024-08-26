import asyncio
import json
from typing import Dict, Any, Callable
import zenoh
from fabric.node.interface import NodeConfig, NodeData


class Orchestrator:
    def __init__(self, id: str, session: zenoh.Session):
        self.id = id
        self.session = session
        self.nodes: Dict[str, Dict[str, Any]] = {}
        self.callbacks: Dict[str, Callable[[NodeData], None]] = {}

    @classmethod
    async def create(cls, id: str, session: zenoh.Session):
        return cls(id, session)

    async def run(self, cancel_event: asyncio.Event):
        subscriber = await self.session.declare_subscriber("node/data")

        while not cancel_event.is_set():
            try:
                sample = await asyncio.wait_for(subscriber.recv(), timeout=1.0)
                if sample:
                    try:
                        data = NodeData(**json.loads(sample.payload.decode()))
                        print(
                            f"Orchestrator {self.id} received data from node {data.node_id}: {data.value:.2f}"
                        )
                        await self.update_node_state(data)
                        await self.trigger_callbacks(data)
                    except json.JSONDecodeError:
                        print("Invalid node data received")
            except asyncio.TimeoutError:
                pass

    async def update_node_state(self, data: NodeData):
        self.nodes[data.node_id] = {
            "last_value": data.value,
            "last_update": data.timestamp,
        }

    async def trigger_callbacks(self, data: NodeData):
        for callback in self.callbacks.values():
            callback(data)

    async def subscribe_to_node(
        self, node_id: str, callback: Callable[[NodeData], None]
    ):
        self.callbacks[node_id] = callback

    async def publish_node_config(self, node_id: str, config: NodeConfig):
        key = f"node/{node_id}/config"
        config_json = json.dumps(config.to_dict())
        await self.session.put(key, config_json)
        print(f"Published configuration for node {node_id}")

    async def publish_node_configs(self, configs: Dict[str, NodeConfig]):
        for node_id, config in configs.items():
            await self.publish_node_config(node_id, config)

    async def send_event_to_node(self, node_id: str, event: str, payload: str):
        key = f"node/{node_id}/event/{event}"
        await self.session.put(key, payload)
        print(f"Sent event {event} to node {node_id}")
