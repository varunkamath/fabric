from .interface import NodeInterface, NodeConfig
from typing import Any, Dict

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
        # Implement generic event handling logic here
        pass

    async def update_config(self, config: NodeConfig) -> None:
        self.config = config

    def as_any(self) -> Any:
        return self

    @classmethod
    def new(cls, config: NodeConfig) -> 'GenericNode':
        return cls(config)