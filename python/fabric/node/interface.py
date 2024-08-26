from abc import ABC, abstractmethod
from dataclasses import dataclass, asdict
from typing import Any, Dict, Optional


@dataclass
class NodeConfig:
    node_id: str
    sampling_rate: int
    threshold: float
    custom_config: Dict[str, Any]

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "NodeConfig":
        return cls(**data)


@dataclass
class NodeData:
    node_id: str
    interface_type: str
    value: float
    timestamp: int
    metadata: Optional[Dict[str, Any]] = None

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "NodeData":
        return cls(**data)


class NodeInterface(ABC):
    @abstractmethod
    async def read(self) -> float:
        pass

    @abstractmethod
    def get_type(self) -> str:
        pass

    @abstractmethod
    def set_config(self, config: NodeConfig):
        pass

    @abstractmethod
    def get_config(self) -> NodeConfig:
        pass

    @abstractmethod
    async def handle_event(self, event: str, payload: str):
        pass
