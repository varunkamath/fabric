import asyncio
import pytest
from fabric import Node
from fabric.node.interface import NodeConfig
from examples.example_quadcopter_node import (
    QuadcopterNode,
    QuadcopterConfig,
    QuadcopterCommand,
    create_zenoh_session,
)


@pytest.fixture
def node_id():
    return "test_quadcopter_node"


@pytest.fixture
def initial_config():
    return {
        "quadcopter_config": {
            "max_altitude": 100.0,
            "max_speed": 10.0,
            "home_position": [0.0, 0.0, 0.0],
            "battery_threshold": 20.0,
        }
    }


@pytest.fixture
def node_config(node_id, initial_config):
    return NodeConfig(node_id=node_id, config=initial_config)


@pytest.fixture
async def zenoh_session():
    session = await create_zenoh_session()
    yield session
    session.close()


@pytest.fixture
async def quadcopter_node(node_id, initial_config):
    return QuadcopterNode(node_id, initial_config)


@pytest.fixture
async def fabric_node(node_id, node_config, zenoh_session, quadcopter_node):
    node = Node(node_id, "quadcopter", node_config, zenoh_session)
    node.interface = quadcopter_node
    return node


@pytest.mark.asyncio
async def test_quadcopter_node_initialization(quadcopter_node, node_id, initial_config):
    assert quadcopter_node.node_id == node_id
    assert quadcopter_node.config.node_id == node_id
    assert quadcopter_node.config.config == initial_config
    assert isinstance(quadcopter_node.quadcopter_config, QuadcopterConfig)
    assert quadcopter_node.altitude == 0.0
    assert quadcopter_node.battery_level == 100.0
    assert quadcopter_node.command_mode == "idle"


@pytest.mark.asyncio
async def test_quadcopter_node_get_config(quadcopter_node):
    config = await quadcopter_node.get_config()
    assert isinstance(config, NodeConfig)
    assert config.node_id == quadcopter_node.node_id


@pytest.mark.asyncio
async def test_quadcopter_node_set_config(quadcopter_node):
    new_config = NodeConfig(
        node_id=quadcopter_node.node_id,
        config={
            "quadcopter_config": {
                "max_altitude": 150.0,
                "max_speed": 15.0,
                "home_position": [1.0, 1.0, 1.0],
                "battery_threshold": 25.0,
            }
        },
    )
    await quadcopter_node.set_config(new_config)
    assert quadcopter_node.quadcopter_config.max_altitude == 150.0
    assert quadcopter_node.quadcopter_config.max_speed == 15.0
    assert quadcopter_node.quadcopter_config.home_position == [1.0, 1.0, 1.0]
    assert quadcopter_node.quadcopter_config.battery_threshold == 25.0


@pytest.mark.asyncio
async def test_quadcopter_node_get_type(quadcopter_node):
    assert quadcopter_node.get_type() == "quadcopter"


@pytest.mark.asyncio
async def test_quadcopter_node_handle_event(quadcopter_node):
    await quadcopter_node.handle_event(QuadcopterCommand.MOVE_TO, [10.0, 20.0, 30.0])
    assert quadcopter_node.command_mode == "moving"

    await quadcopter_node.handle_event(QuadcopterCommand.LAND, None)
    assert quadcopter_node.command_mode == "landing"

    await quadcopter_node.handle_event(QuadcopterCommand.TAKE_OFF, None)
    assert quadcopter_node.command_mode == "taking_off"

    await quadcopter_node.handle_event("unknown_command", None)
    assert (
        quadcopter_node.command_mode == "taking_off"
    )  # Should not change for unknown command


@pytest.mark.asyncio
async def test_quadcopter_node_update_config(quadcopter_node):
    new_config = NodeConfig(
        node_id=quadcopter_node.node_id,
        config={
            "quadcopter_config": {
                "max_altitude": 200.0,
                "max_speed": 20.0,
                "home_position": [2.0, 2.0, 2.0],
                "battery_threshold": 30.0,
            }
        },
    )
    await quadcopter_node.update_config(new_config)
    assert quadcopter_node.quadcopter_config.max_altitude == 200.0
    assert quadcopter_node.quadcopter_config.max_speed == 20.0
    assert quadcopter_node.quadcopter_config.home_position == [2.0, 2.0, 2.0]
    assert quadcopter_node.quadcopter_config.battery_threshold == 30.0


@pytest.mark.asyncio
async def test_quadcopter_node_run(fabric_node, quadcopter_node):
    published_data = []

    async def mock_publish(topic, data):
        published_data.append(data)

    fabric_node.publish = mock_publish

    cancel_token = asyncio.Event()
    run_task = asyncio.create_task(quadcopter_node.run(fabric_node, cancel_token))
    await asyncio.sleep(3)  # Run for 3 seconds
    cancel_token.set()
    await run_task

    assert len(published_data) > 0
    for data in published_data:
        node_data = eval(data)
        assert node_data["node_id"] == quadcopter_node.node_id
        assert node_data["node_type"] == "quadcopter"
        assert "timestamp" in node_data
        assert "altitude" in node_data["metadata"]
        assert "battery_level" in node_data["metadata"]
        assert "command_mode" in node_data["metadata"]


@pytest.mark.asyncio
async def test_quadcopter_node_low_battery(fabric_node, quadcopter_node):
    published_data = []

    async def mock_publish(topic, data):
        published_data.append(data)

    fabric_node.publish = mock_publish

    # Set initial battery level close to the threshold
    quadcopter_node.battery_level = 21.0

    cancel_token = asyncio.Event()
    run_task = asyncio.create_task(quadcopter_node.run(fabric_node, cancel_token))
    await asyncio.sleep(5)  # Run for 5 seconds to ensure battery drops below threshold
    cancel_token.set()
    await run_task

    assert any(
        eval(data)["metadata"]["command_mode"] == "returning_home"
        for data in published_data
    )


@pytest.mark.asyncio
async def test_create_zenoh_session():
    session = await create_zenoh_session()
    assert session is not None
    info = session.info()
    assert info.zid() is not None
    session.close()


if __name__ == "__main__":
    pytest.main([__file__])
