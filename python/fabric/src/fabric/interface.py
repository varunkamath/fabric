from abc import ABC, abstractmethod
from typing import Any, Dict, Optional
from pydantic import BaseModel, Field

class NodeConfig(BaseModel):
    node_id: str
    config: Dict[str, Any]

class NodeData(BaseModel):
    node_id: str
    node_type: str
    timestamp: int
    metadata: Optional[Dict[str, Any]] = None
    status: str = Field(default="online")

class NodeInterface(ABC):
    @abstractmethod
    async def get_config(self) -> NodeConfig:
        pass

    @abstractmethod
    async def set_config(self, config: NodeConfig):
        pass

    @abstractmethod
    def get_type(self) -> str:
        pass

    @abstractmethod
    async def create_publisher(self, topic: str) -> None:
        pass

    @abstractmethod
    async def create_subscriber(self, topic: str, callback: Any) -> None:
        pass

    @abstractmethod
    async def publish(self, topic: str, data: Any) -> None:
        pass

    @abstractmethod
    async def update_config(self, config: NodeConfig) -> None:
        pass

    @abstractmethod
    async def get_state(self) -> NodeData:
        pass

    @abstractmethod
    async def handle_event(self, event: str, payload: str) -> None:
        pass