import asyncio
import json
import yaml
from dataclasses import dataclass
from typing import Dict, Any
import zenoh
import time


@dataclass
class SensorData:
    sensor_id: str
    value: float


@dataclass
class SensorConfig:
    sampling_rate: int
    threshold: float


class SensorState:
    def __init__(self, value: float):
        self.last_value = value
        self.last_update = time.time()


class Orchestrator:
    def __init__(self):
        self.session = None
        self.sensors = {}
        self.callbacks = {}

    async def initialize(self):
        try:
            conf = zenoh.Config()
            conf.insert_json5("listen", json.dumps({"endpoints": ["tcp/0.0.0.0:7447"]}))
            self.session = await zenoh.open(conf)
        except Exception as e:
            print(f"Failed to initialize Zenoh session: {e}")
            raise

    async def run(self, cancel_event: asyncio.Event):
        subscriber = await self.session.declare_subscriber("sensor/#")
        async for sample in subscriber.receiver():
            if cancel_event.is_set():
                break
            try:
                payload = await sample.payload.decode("utf-8")
                data = json.loads(payload)
                sensor_data = SensorData(**data)
                print(
                    f"Control node received data from sensor {sensor_data.sensor_id}: {sensor_data.value:.2f}"
                )  # Add this line
                await self.update_sensor_state(sensor_data)
                await self.trigger_callbacks(sensor_data)
            except json.JSONDecodeError:
                print(f"Failed to parse sensor data: {payload}")

    async def update_sensor_state(self, data: SensorData):
        self.sensors[data.sensor_id] = SensorState(value=data.value)
        print(f"Updated sensor {data.sensor_id}: {data.value:.2f}")

    async def trigger_callbacks(self, data: SensorData):
        callback = self.callbacks.get(data.sensor_id)
        if callback:
            await callback(data)

    def subscribe_to_sensor(self, sensor_id: str, callback):
        self.callbacks[sensor_id] = callback

    async def monitor_sensors(self, cancel_event: asyncio.Event):
        while not cancel_event.is_set():
            print("Current sensor states:")
            for id, state in self.sensors.items():
                print(
                    f"  Sensor {id}: {state.last_value:.2f} (last update: {time.time() - state.last_update:.2f}s ago)"
                )
            await asyncio.sleep(10)

    @staticmethod
    async def load_config(path: str) -> Dict[str, Any]:
        with open(path, "r") as f:
            return yaml.safe_load(f)

    async def publish_sensor_config(self, sensor_id: str, sensor_config: SensorConfig):
        key = f"sensor/{sensor_id}/config"
        config_json = json.dumps(dataclass_to_dict(sensor_config))
        await self.session.put(key, config_json)
        print(f"Published configuration for sensor {sensor_id}")


def dataclass_to_dict(obj):
    return {
        field.name: getattr(obj, field.name)
        for field in obj.__dataclass_fields__.values()
    }


async def main():
    print("Starting control node...")
    orchestrator = Orchestrator()
    await orchestrator.initialize()

    cancel_event = asyncio.Event()

    # Load configuration
    config = await Orchestrator.load_config("config.yaml")

    # Publish configurations to sensors
    for sensor_id, sensor_config in config["sensors"].items():
        await orchestrator.publish_sensor_config(
            sensor_id, SensorConfig(**sensor_config)
        )

    # Subscribe to all sensors
    orchestrator.subscribe_to_sensor(
        "sensor/**",
        lambda data: print(
            f"Received data from sensor {data.sensor_id}: {data.value:.2f}"
        ),
    )

    run_task = asyncio.create_task(orchestrator.run(cancel_event))
    monitor_task = asyncio.create_task(orchestrator.monitor_sensors(cancel_event))

    try:
        await asyncio.gather(run_task, monitor_task)
    except asyncio.CancelledError:
        print("Shutting down...")
    finally:
        cancel_event.set()
        await orchestrator.session.close()


if __name__ == "__main__":
    asyncio.run(main())
