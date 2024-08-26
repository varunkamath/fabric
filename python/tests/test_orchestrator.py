import pytest
import json
from unittest.mock import MagicMock, AsyncMock
from fabric.orchestrator import Orchestrator
from fabric.node.interface import NodeConfig, NodeData


@pytest.fixture
def mock_zenoh_session():
    session = MagicMock()
    session.put = AsyncMock()
    session.declare_subscriber = AsyncMock(return_value=MagicMock())
    return session


@pytest.mark.asyncio
async def test_orchestrator_creation(mock_zenoh_session):
    orchestrator = await Orchestrator.create("test_orchestrator", mock_zenoh_session)
    assert orchestrator.id == "test_orchestrator"
    assert orchestrator.session == mock_zenoh_session


@pytest.mark.asyncio
async def test_publish_node_config(mock_zenoh_session):
    orchestrator = await Orchestrator.create("test_orchestrator", mock_zenoh_session)
    config = NodeConfig(
        node_id="test_node", sampling_rate=5, threshold=50.0, custom_config={}
    )
    await orchestrator.publish_node_config("test_node", config)
    mock_zenoh_session.put.assert_called_once()
    args, _ = mock_zenoh_session.put.call_args
    assert args[0] == "node/test_node/config"
    assert json.loads(args[1]) == config.to_dict()


@pytest.mark.asyncio
async def test_send_event_to_node(mock_zenoh_session):
    orchestrator = await Orchestrator.create("test_orchestrator", mock_zenoh_session)
    await orchestrator.send_event_to_node("test_node", "test_event", "test_payload")
    mock_zenoh_session.put.assert_called_once()
    args, _ = mock_zenoh_session.put.call_args
    assert args[0] == "node/test_node/event/test_event"
    assert args[1] == "test_payload"


@pytest.mark.asyncio
async def test_update_node_state(mock_zenoh_session):
    orchestrator = await Orchestrator.create("test_orchestrator", mock_zenoh_session)
    data = NodeData(
        node_id="test_node",
        interface_type="mock",
        value=42.0,
        timestamp=1234567890,
        metadata=None,
    )
    await orchestrator.update_node_state(data)
    assert "test_node" in orchestrator.nodes
    assert orchestrator.nodes["test_node"]["last_value"] == 42.0
