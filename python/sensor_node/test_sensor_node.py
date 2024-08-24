import pytest
import asyncio
from unittest.mock import AsyncMock, patch
from python.sensor_node.main import SensorNode, SensorData, SensorConfig


@pytest.fixture
def sensor_node():
    return SensorNode("test-sensor", "tcp/localhost:7447")


@pytest.mark.asyncio
async def test_read_sensor(sensor_node):
    data = await sensor_node.read_sensor()
    assert isinstance(data, SensorData)
    assert data.sensor_id == "test-sensor"
    assert 0 <= data.value <= 100


@pytest.mark.asyncio
async def test_apply_config(sensor_node):
    initial_config = sensor_node.config
    new_config = SensorConfig(sampling_rate=10, threshold=75.0)
    sensor_node.apply_config(new_config)
    assert sensor_node.config != initial_config
    assert sensor_node.config.sampling_rate == 10
    assert sensor_node.config.threshold == 75.0


@pytest.mark.asyncio
async def test_publish_sensor_data(sensor_node):
    mock_session = AsyncMock()
    mock_publisher = AsyncMock()
    mock_session.declare_publisher.return_value = mock_publisher

    async def delayed_cancel():
        await asyncio.sleep(0.1)
        sensor_node.cancel_event.set()

    cancel_task = asyncio.create_task(delayed_cancel())
    await sensor_node.publish_sensor_data(mock_session)
    await cancel_task

    mock_session.declare_publisher.assert_called_once_with("sensor/data")
    mock_publisher.put.assert_called()


@pytest.mark.asyncio
async def test_subscribe_to_config(sensor_node):
    mock_session = AsyncMock()
    mock_subscriber = AsyncMock()
    mock_session.declare_subscriber.return_value = mock_subscriber

    mock_change = AsyncMock()
    mock_change.value.payload = b'{"sampling_rate": 15, "threshold": 80.0}'
    
    async def mock_receiver():
        yield mock_change

    mock_subscriber.receiver = mock_receiver

    await sensor_node.subscribe_to_config(mock_session)

    assert sensor_node.config.sampling_rate == 15
    assert sensor_node.config.threshold == 80.0


@pytest.mark.asyncio
async def test_run(sensor_node):
    with patch("zenoh.open") as mock_zenoh_open:
        mock_session = AsyncMock()
        mock_zenoh_open.return_value.__aenter__.return_value = mock_session

        run_task = asyncio.create_task(sensor_node.run())
        await asyncio.sleep(0.1)  # Give some time for the run method to start
        sensor_node.cancel_event.set()  # Signal the run method to stop
        await run_task

        mock_zenoh_open.assert_called_once()
        assert mock_zenoh_open.call_args[0][0].config["connect"]["endpoints"] == [sensor_node.zenoh_peer]