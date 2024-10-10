import asyncio
import pytest
import zenoh
from fabric.orchestrator.orchestrator import Orchestrator
from fabric.node.interface import NodeConfig, NodeData
from fabric.error import FabricError

@pytest.fixture
def zenoh_session():
    config = zenoh.Config()
    session = zenoh.open(config)
    yield session
    session.close()

@pytest.fixture
def orchestrator(zenoh_session):
    orchestrator = Orchestrator("test_orchestrator", zenoh_session)
    yield orchestrator
    orchestrator.cleanup()  # This is now synchronous

@pytest.mark.asyncio
async def test_orchestrator_initialization(orchestrator):
    assert orchestrator.id == "test_orchestrator"
    assert isinstance(orchestrator.nodes, dict)
    assert isinstance(orchestrator.publishers, dict)
    assert isinstance(orchestrator.subscribers, dict)
    assert isinstance(orchestrator.callbacks, dict)

@pytest.mark.asyncio
async def test_publish_node_config(orchestrator):
    node_id = "test_node"
    config = NodeConfig(node_id=node_id, config={"key": "value"})
    await orchestrator.publish_node_config(node_id, config)
    assert f"node/{node_id}/config" in orchestrator.publishers

@pytest.mark.asyncio
async def test_update_node_state(orchestrator):
    node_id = "test_node"
    node_data = NodeData(node_id=node_id, node_type="test", timestamp=0, metadata=None, status="online")
    await orchestrator.update_node_state(node_data)
    assert node_id in orchestrator.nodes

@pytest.mark.asyncio
async def test_get_node_state(orchestrator):
    node_id = "test_node"
    node_data = NodeData(node_id=node_id, node_type="test", timestamp=0, metadata=None, status="online")
    await orchestrator.update_node_state(node_data)
    state = await orchestrator.get_node_state(node_id)
    assert state.last_value == node_data

@pytest.mark.asyncio
async def test_get_node_state_not_found(orchestrator):
    with pytest.raises(FabricError):
        await orchestrator.get_node_state("non_existent_node")

@pytest.mark.asyncio
async def test_register_callback(orchestrator):
    node_id = "test_node"
    callback = lambda data: None
    await orchestrator.register_callback(node_id, callback)
    assert node_id in orchestrator.callbacks

@pytest.mark.asyncio
async def test_publish_with_retry(orchestrator):
    topic = "test_topic"
    data = "test_data"
    orchestrator.publish_with_retry(topic, data)
    # This test just checks if the method runs without errors
    # In a real scenario, you'd want to verify the data was actually published

@pytest.mark.asyncio
async def test_remove_node(orchestrator):
    node_id = "test_node"
    node_data = NodeData(node_id=node_id, node_type="test", timestamp=0, metadata=None, status="online")
    await orchestrator.update_node_state(node_data)
    await orchestrator.remove_node(node_id)
    assert node_id not in orchestrator.nodes

@pytest.mark.asyncio
async def test_send_event_to_node(orchestrator):
    node_id = "test_node"
    event = "test_event"
    payload = {"key": "value"}
    await orchestrator.send_event_to_node(node_id, event, payload)
    assert f"node/{node_id}/events" in orchestrator.publishers

@pytest.mark.asyncio
async def test_get_node_data(orchestrator):
    node_id = "test_node"
    node_data = NodeData(node_id=node_id, node_type="test", timestamp=0, metadata=None, status="online")
    await orchestrator.update_node_state(node_data)
    retrieved_data = await orchestrator.get_node_data(node_id)
    assert retrieved_data == node_data

@pytest.mark.asyncio
async def test_process_node_updates(orchestrator):
    node_id = "test_node"
    node_data = NodeData(node_id=node_id, node_type="test", timestamp=0, metadata=None, status="online")
    await orchestrator.update_node_state(node_data)
    # Set the last_update to a time more than 5 seconds ago
    orchestrator.nodes[node_id].last_update = 0
    await orchestrator.process_node_updates()
    # This test just checks if the method runs without errors
    # In a real scenario, you'd want to verify that an update was requested

@pytest.mark.asyncio
async def test_run(orchestrator):
    cancel_token = asyncio.Event()
    run_task = asyncio.create_task(orchestrator.run(cancel_token))
    await asyncio.sleep(0.1)  # Let the run method start
    cancel_token.set()
    await run_task
    # This test just checks if the run method starts and stops without errors