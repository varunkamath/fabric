from abc import ABC, abstractmethod
from typing import Any, Dict, Optional
from dataclasses import dataclass, asdict

@dataclass
class NodeConfig:
    node_id: str
    config: Dict[str, Any]

@dataclass
class NodeData:
    node_id: str
    node_type: str
    timestamp: int
    metadata: Optional[Dict[str, Any]] = None
    status: str = "online"

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "NodeData":
        return cls(**data)

    @classmethod
    def new(cls, node_id: str) -> "NodeData":
        return cls(
            node_id=node_id,
            node_type="",
            timestamp=0,
            metadata=None,
            status="online"
        )

    def get(self, key: str) -> str:
        if self.metadata and key in self.metadata:
            return str(self.metadata[key])
        raise KeyError(f"Key '{key}' not found in metadata")

    def set_status(self, status: str) -> None:
        self.status = status

class NodeInterface(ABC):
    @abstractmethod
    def get_config(self) -> NodeConfig:
        pass

    @abstractmethod
    async def set_config(self, config: NodeConfig) -> None:
        pass

    @abstractmethod
    def get_type(self) -> str:
        pass

    @abstractmethod
    async def handle_event(self, event: str, payload: str) -> None:
        pass

    @abstractmethod
    async def update_config(self, config: NodeConfig) -> None:
        pass

    @abstractmethod
    def as_any(self) -> Any:
        pass

class NodeFactory(ABC):
    @abstractmethod
    def create(self, config: NodeConfig) -> NodeInterface:
        pass