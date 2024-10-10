import asyncio
from typing import Any, Dict
from .interface import NodeInterface, NodeConfig
from ..error import FabricError

class GenericNode(NodeInterface):
    def __init__(self, node_id: str, initial_config: Dict[str, Any]):
        self.node_id = node_id
        self.config = NodeConfig(node_id=node_id, config=initial_config)
        self.node_type = "generic"

    def get_config(self) -> NodeConfig:
        return self.config

    async def set_config(self, config: NodeConfig) -> None:
        self.config = config

    def get_type(self) -> str:
        return self.node_type

    async def handle_event(self, event: str, payload: str) -> None:
        print(f"Handling event: {event} with payload: {payload}")

    async def update_config(self, config: NodeConfig) -> None:
        await self.set_config(config)

    async def run(self, cancel_token: asyncio.Event) -> None:
        while not cancel_token.is_set():
            print(f"Generic node {self.node_id} running with config: {self.config}")
            await asyncio.sleep(1)