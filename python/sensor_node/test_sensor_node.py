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
    mock_publisher.put.assert_awaited()


@pytest.mark.asyncio
async def test_subscribe_to_config(sensor_node):
    mock_session = AsyncMock()
    mock_subscriber = AsyncMock()
    mock_session.declare_subscriber.return_value = mock_subscriber

    mock_change = AsyncMock()
    mock_change.payload = '{"sampling_rate": 15, "threshold": 80.0}'

    async def mock_receiver():
        yield mock_change

    mock_subscriber.receiver = mock_receiver()

    task = asyncio.create_task(sensor_node.subscribe_to_config(mock_session))
    await asyncio.sleep(0.1)
    task.cancel()
    try:
        await task
    except asyncio.CancelledError:
        pass

    assert sensor_node.config.sampling_rate == 15
    assert sensor_node.config.threshold == 80.0


@pytest.mark.asyncio
async def test_run(sensor_node):
    with patch("zenoh.open") as mock_zenoh_open, patch("json.dumps") as mock_json_dumps:
        mock_session = AsyncMock()
        mock_zenoh_open.return_value = mock_session
        mock_json_dumps.return_value = '{"endpoints": ["tcp/localhost:7447"]}'

        mock_subscriber = AsyncMock()
        mock_session.declare_subscriber.return_value = mock_subscriber

        mock_change = AsyncMock()
        mock_change.payload = AsyncMock()
        mock_change.payload.decode.return_value = (
            '{"sampling_rate": 10, "threshold": 75.0}'
        )

        async def mock_receiver():
            yield mock_change

        mock_subscriber.receiver = mock_receiver()

        run_task = asyncio.create_task(sensor_node.run())
        await asyncio.sleep(0.1)
        sensor_node.cancel_event.set()
        await run_task

        mock_zenoh_open.assert_called_once()
        mock_json_dumps.assert_called_once_with({"endpoints": [sensor_node.zenoh_peer]})
