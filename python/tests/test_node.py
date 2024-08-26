import pytest
import asyncio
import json
import logging
from unittest.mock import MagicMock, patch, AsyncMock
from fabric.node import Node
from fabric.node.interface import NodeConfig, NodeInterface
from fabric.plugins import NodeRegistry

logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger(__name__)


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


NodeRegistry.register_interface("mock", MockInterface)


@pytest.fixture
def mock_zenoh_session():
    session = MagicMock()
    session.declare_publisher = AsyncMock(return_value=AsyncMock())
    session.declare_subscriber = AsyncMock(return_value=AsyncMock())
    return session


@pytest.mark.asyncio
async def test_node_run(mock_zenoh_session, caplog):
    caplog.set_level(logging.DEBUG)
    config = NodeConfig(
        node_id="test_node", sampling_rate=5, threshold=50.0, custom_config={}
    )
    node = await Node.create("test_node", "mock", config, mock_zenoh_session)

    cancel_event = asyncio.Event()

    # Ensure that the mock publisher's put method returns a coroutine
    mock_publisher = mock_zenoh_session.declare_publisher.return_value
    mock_publisher.put.return_value = asyncio.Future()
    mock_publisher.put.return_value.set_result(None)

    # Mock the config subscriber
    config_subscriber = AsyncMock()
    config_subscriber.recv.return_value = MagicMock(
        payload=json.dumps(config.to_dict()).encode()
    )

    # Mock the event subscriber
    event_subscriber = AsyncMock()
    event_subscriber.recv.return_value = MagicMock(
        key_expr=MagicMock(as_string=lambda: "node/test_node/event/test_event"),
        payload="test_payload".encode(),
    )

    # Set up the mock session to return our mocked subscribers
    mock_zenoh_session.declare_subscriber.side_effect = [
        config_subscriber,
        event_subscriber,
    ]

    async def run_node():
        task = asyncio.create_task(node.run(cancel_event))
        await asyncio.sleep(0.2)  # Increased sleep time to allow for one loop iteration
        cancel_event.set()
        await task

    await asyncio.wait_for(run_node(), timeout=1.0)

    mock_zenoh_session.declare_publisher.assert_called_once_with("node/data")
    mock_zenoh_session.declare_subscriber.assert_any_call("node/test_node/config")
    mock_zenoh_session.declare_subscriber.assert_any_call("node/test_node/event/*")

    # Assert that the publisher's put method was called with valid JSON
    mock_publisher.put.assert_called_once()
    args, _ = mock_publisher.put.call_args
    assert isinstance(args[0], str), f"Expected str, got {type(args[0])}"
    assert json.loads(args[0])  # Ensure the argument is valid JSON

    # Print captured logs
    print("Captured logs:")
    for record in caplog.records:
        print(f"{record.levelname}: {record.message}")

    # Assert that no errors were logged
    error_logs = [
        record for record in caplog.records if record.levelno == logging.ERROR
    ]
    assert not error_logs, f"Errors were logged during the test: {error_logs}"


@pytest.mark.asyncio
async def test_node_config_update(mock_zenoh_session):
    config = NodeConfig(
        node_id="test_node", sampling_rate=5, threshold=50.0, custom_config={}
    )
    node = await Node.create("test_node", "mock", config, mock_zenoh_session)

    new_config = NodeConfig(
        node_id="test_node", sampling_rate=10, threshold=75.0, custom_config={}
    )

    config_subscriber = AsyncMock()
    config_subscriber.recv.return_value = MagicMock(
        payload=json.dumps(new_config.to_dict()).encode()
    )
    mock_zenoh_session.declare_subscriber.return_value = config_subscriber

    cancel_event = asyncio.Event()

    async def run_node():
        task = asyncio.create_task(node.run(cancel_event))
        await asyncio.sleep(0.2)  # Increased sleep time to allow for config update
        cancel_event.set()
        await task

    await asyncio.wait_for(run_node(), timeout=1.0)

    assert node.get_config() == new_config
    assert node.interface.get_config() == new_config


@pytest.mark.asyncio
async def test_node_event_handling(mock_zenoh_session):
    config = NodeConfig(
        node_id="test_node", sampling_rate=5, threshold=50.0, custom_config={}
    )
    node = await Node.create("test_node", "mock", config, mock_zenoh_session)

    event_subscriber = AsyncMock()
    event_subscriber.recv.return_value = MagicMock(
        key_expr=MagicMock(as_string=lambda: "node/test_node/event/test_event"),
        payload="test_payload".encode(),
    )
    mock_zenoh_session.declare_subscriber.return_value = event_subscriber

    cancel_event = asyncio.Event()

    async def run_node():
        with patch.object(
            node, "handle_event", new_callable=AsyncMock
        ) as mock_handle_event:
            task = asyncio.create_task(node.run(cancel_event))
            await asyncio.sleep(0.5)  # Increased sleep time to allow for event handling
            cancel_event.set()
            await task
            mock_handle_event.assert_called_once_with("test_event", "test_payload")

    await asyncio.wait_for(run_node(), timeout=2.0)

    # Add assertions to check if the event was processed
    assert event_subscriber.recv.called, "Event subscriber's recv method was not called"
    mock_zenoh_session.declare_subscriber.assert_any_call("node/test_node/event/*")

    # Add assertion to check if the run method exited after handling the event
    assert cancel_event.is_set(), "Cancel event should be set after handling the event"
