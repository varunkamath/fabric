from typing import Dict, Type
from fabric.node.interface import NodeInterface, NodeConfig


class NodeRegistry:
    _interfaces: Dict[str, Type[NodeInterface]] = {}

    @classmethod
    def register_interface(
        cls, interface_type: str, interface_class: Type[NodeInterface]
    ):
        cls._interfaces[interface_type] = interface_class

    @classmethod
    def create_interface(cls, interface_type: str, config: NodeConfig) -> NodeInterface:
        interface_class = cls._interfaces.get(interface_type)
        if interface_class:
            return interface_class(config)
        raise ValueError(f"Unknown interface type: {interface_type}")
