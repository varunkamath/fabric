from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any, Dict, Optional
import json


@dataclass
class NodeConfig:
    node_id: str
    config: Dict[str, Any]

    def to_json(self) -> str:
        return json.dumps({"node_id": self.node_id, "config": self.config})


@dataclass
class NodeData:
    node_id: str
    node_type: str
    timestamp: int
    metadata: Optional[Dict[str, Any]]
    status: str = "online"

    @classmethod
    def from_json(cls, json_str: str) -> "NodeData":
        data = json.loads(json_str)
        return cls(**data)

    def to_json(self) -> str:
        return json.dumps(self.__dict__)

    def get(self, key: str) -> str:
        if self.metadata and key in self.metadata:
            return str(self.metadata[key])
        raise KeyError(f"Key '{key}' not found in metadata")

    def set_status(self, status: str) -> None:
        self.status = status


class NodeInterface(ABC):
    @abstractmethod
    async def run(self, node: Any, cancel_token: Any) -> None:
        pass

    @abstractmethod
    async def get_config(self) -> Any:
        pass

    @abstractmethod
    async def set_config(self, config: NodeConfig) -> None:
        pass

    @abstractmethod
    def get_type(self) -> str:
        pass

    @abstractmethod
    async def handle_event(self, event: str, payload: Any) -> None:
        pass

    @abstractmethod
    async def update_config(self, config: NodeConfig) -> None:
        pass
