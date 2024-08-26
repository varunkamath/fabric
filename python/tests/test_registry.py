import pytest
from fabric.plugins import NodeRegistry
from fabric.node.interface import NodeConfig, NodeInterface


class MockInterface(NodeInterface):
    def __init__(self, config: NodeConfig):
        self.config = config
        self.type = "mock"

    async def read(self) -> float:
        return 42.0

    def get_type(self) -> str:
        return self.type

    def set_config(self, config: NodeConfig):
        self.config = config

    def get_config(self) -> NodeConfig:
        return self.config

    async def handle_event(self, event: str, payload: str):
        pass


def test_node_registry():
    NodeRegistry.register_interface("mock", MockInterface)

    config = NodeConfig(
        node_id="test_node", sampling_rate=5, threshold=50.0, custom_config={}
    )

    interface = NodeRegistry.create_interface("mock", config)
    assert isinstance(interface, MockInterface)
    assert interface.get_type() == "mock"
    assert interface.get_config() == config

    with pytest.raises(ValueError):
        NodeRegistry.create_interface("unknown", config)
