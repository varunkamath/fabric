import pytest
import asyncio
import time
from unittest.mock import AsyncMock, patch, mock_open
from python.control_node.main import Orchestrator, SensorData, SensorConfig, SensorState


@pytest.fixture
def orchestrator():
    return Orchestrator()


@pytest.mark.asyncio
async def test_initialize(orchestrator):
    with patch("zenoh.open") as mock_zenoh_open:
        mock_session = AsyncMock()
        mock_zenoh_open.return_value = asyncio.Future()
        mock_zenoh_open.return_value.set_result(mock_session)
        await orchestrator.initialize()
        mock_zenoh_open.assert_called_once()
        assert orchestrator.session == mock_session


@pytest.mark.asyncio
async def test_update_sensor_state(orchestrator):
    data = SensorData(sensor_id="test-sensor", value=42.0)
    await orchestrator.update_sensor_state(data)
    assert "test-sensor" in orchestrator.sensors
    assert orchestrator.sensors["test-sensor"].last_value == 42.0


@pytest.mark.asyncio
async def test_trigger_callbacks(orchestrator):
    callback = AsyncMock()
    orchestrator.callbacks["test-sensor"] = callback
    data = SensorData(sensor_id="test-sensor", value=42.0)
    await orchestrator.trigger_callbacks(data)
    callback.assert_awaited_once_with(data)


@pytest.mark.asyncio
async def test_subscribe_to_sensor(orchestrator):
    def callback(x):
        pass

    orchestrator.subscribe_to_sensor("test-sensor", callback)
    assert "test-sensor" in orchestrator.callbacks


@pytest.mark.asyncio
async def test_monitor_sensors(orchestrator):
    from python.control_node.main import SensorState
    import time

    orchestrator.sensors["test-sensor"] = SensorState(value=42.0)
    cancel_event = asyncio.Event()
    monitor_task = asyncio.create_task(orchestrator.monitor_sensors(cancel_event))
    await asyncio.sleep(0.1)  # Give some time for the monitor to run
    cancel_event.set()
    await monitor_task


@pytest.mark.asyncio
async def test_load_config():
    mock_config = """
    sensors:
      sensor1:
        sampling_rate: 5
        threshold: 50.0
    """
    with patch("builtins.open", mock_open(read_data=mock_config)):
        config = await Orchestrator.load_config("dummy_path")
        assert "sensors" in config
        assert "sensor1" in config["sensors"]
        assert config["sensors"]["sensor1"]["sampling_rate"] == 5
        assert config["sensors"]["sensor1"]["threshold"] == 50.0


@pytest.mark.asyncio
async def test_publish_sensor_config(orchestrator):
    orchestrator.session = AsyncMock()
    sensor_id = "test-sensor"
    sensor_config = SensorConfig(sampling_rate=10, threshold=75.0)
    await orchestrator.publish_sensor_config(sensor_id, sensor_config)
    orchestrator.session.put.assert_called_once()


@pytest.mark.asyncio
async def test_run(orchestrator):
    orchestrator.session = AsyncMock()
    mock_subscriber = AsyncMock()
    orchestrator.session.declare_subscriber.return_value = mock_subscriber

    mock_sample = AsyncMock()
    mock_sample.payload.decode.return_value = '{"sensor_id": "test-sensor", "value": 42.0}'

    async def mock_receiver():
        yield mock_sample

    mock_subscriber.receiver = mock_receiver

    cancel_event = asyncio.Event()
    run_task = asyncio.create_task(orchestrator.run(cancel_event))
    await asyncio.sleep(0.1)  # Give some time for the run method to start
    cancel_event.set()  # Signal the run method to stop
    await run_task

    orchestrator.session.declare_subscriber.assert_called_once_with("sensor/#")
    assert "test-sensor" in orchestrator.sensors
    assert orchestrator.sensors["test-sensor"].last_value == 42.0