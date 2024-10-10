import asyncio
import pytest
import zenoh
from fabric.node.node import Node
from fabric.node.interface import NodeConfig


@pytest.fixture
def zenoh_session():
    config = zenoh.Config()
    session = zenoh.open(config)
    yield session
    session.close()


@pytest.fixture
def node(zenoh_session):
    node_config = NodeConfig(node_id="test_node", config={"key": "value"})
    node = Node("test_node", "test", node_config, zenoh_session)
    yield node
    node.cleanup()


@pytest.mark.asyncio
async def test_node_initialization(node):
    assert node.id == "test_node"
    assert node.node_type == "test"
    assert isinstance(node.config, NodeConfig)
    assert node.config.node_id == "test_node"
    assert node.config.config == {"key": "value"}


@pytest.mark.asyncio
async def test_node_get_config(node):
    config = await node.get_config()
    assert isinstance(config, NodeConfig)
    assert config.node_id == "test_node"
    assert config.config == {"key": "value"}


@pytest.mark.asyncio
async def test_node_set_config(node):
    new_config = NodeConfig(node_id="test_node", config={"new_key": "new_value"})
    await node.set_config(new_config)
    config = await node.get_config()
    assert config.config == {"new_key": "new_value"}


@pytest.mark.asyncio
async def test_node_get_type(node):
    assert node.get_type() == "test"


@pytest.mark.asyncio
async def test_node_handle_event(node):
    # This test assumes that the node's interface is implemented
    # and has a handle_event method. If not, you may need to create a mock interface.
    node.interface = MockNodeInterface()
    await node.handle_event("test_event", "test_payload")
    assert node.interface.last_event == "test_event"
    assert node.interface.last_payload == "test_payload"


@pytest.mark.asyncio
async def test_node_update_config(node):
    new_config = NodeConfig(
        node_id="test_node", config={"updated_key": "updated_value"}
    )
    await node.update_config(new_config)
    config = await node.get_config()
    assert config.config == {"updated_key": "updated_value"}


@pytest.mark.asyncio
async def test_node_handle_config_update(node):
    new_config = NodeConfig(
        node_id="test_node", config={"updated_key": "updated_value"}
    )
    await node.handle_config_update(MockSample(new_config.to_json()))
    config = await node.get_config()
    assert config.config == {"updated_key": "updated_value"}


@pytest.mark.asyncio
async def test_node_publish(node):
    node.publish("test_topic", "test_data")
    # This test just checks if the publish method runs without errors
    # In a real scenario, you'd want to verify the data was actually published


@pytest.mark.asyncio
async def test_node_run(node):
    cancel_token = asyncio.Event()
    run_task = asyncio.create_task(node.run(cancel_token))
    await asyncio.sleep(0.1)  # Let the run method start
    cancel_token.set()
    await run_task
    # This test just checks if the run method starts and stops without errors


class MockNodeInterface:
    def __init__(self):
        self.last_event = None
        self.last_payload = None

    async def handle_event(self, event: str, payload: str):
        self.last_event = event
        self.last_payload = payload


class MockSample:
    def __init__(self, payload):
        self.payload = payload.encode("utf-8")


if __name__ == "__main__":
    pytest.main([__file__])
