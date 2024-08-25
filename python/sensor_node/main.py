import asyncio
import json
import os
import random
import zenoh
from dataclasses import dataclass


@dataclass
class SensorData:
    sensor_id: str
    value: float


@dataclass
class SensorConfig:
    sampling_rate: int
    threshold: float


class SensorNode:
    def __init__(self, sensor_id, zenoh_peer):
        self.sensor_id = sensor_id
        self.zenoh_peer = zenoh_peer
        self.config = SensorConfig(sampling_rate=5, threshold=50.0)  # Default config
        self.cancel_event = asyncio.Event()

    def apply_config(self, new_config):
        self.config = new_config
        print(f"Applying new configuration: {self.config}")
        # Add logic to apply configuration to sensor hardware or behavior

    async def read_sensor(self) -> SensorData:
        await asyncio.sleep(1)  # Simulate sensor read time
        return SensorData(sensor_id=self.sensor_id, value=random.uniform(0, 100))

    async def publish_sensor_data(self, session: zenoh.Session):
        pub = session.declare_publisher("sensor/data")
        while not self.cancel_event.is_set():
            data = await self.read_sensor()
            payload = json.dumps(dataclass_to_dict(data))
            print(
                f"Sensor {self.sensor_id} publishing data: {data.value:.2f}"
            )  # Add this line
            pub.put(payload)
            await asyncio.sleep(self.config.sampling_rate)

    async def subscribe_to_config(self, session: zenoh.Session):
        key = f"sensor/{self.sensor_id}/config"
        sub = session.declare_subscriber(key)
        async for change in sub.receiver():
            try:
                if isinstance(change.payload, (str, bytes, bytearray)):
                    config_dict = json.loads(
                        change.payload.decode("utf-8")
                        if isinstance(change.payload, (bytes, bytearray))
                        else change.payload
                    )
                    new_config = SensorConfig(**config_dict)
                    print(f"Received new configuration: {new_config}")
                    self.apply_config(new_config)
                else:
                    print(f"Unexpected payload type: {type(change.payload)}")
            except json.JSONDecodeError:
                print(f"Failed to parse configuration: {change.payload}")
            except Exception as e:
                print(f"Error processing configuration: {e}")

    async def run(self):
        conf = zenoh.Config()
        conf.insert_json5("connect", json.dumps({"endpoints": [self.zenoh_peer]}))

        try:
            session = zenoh.open(conf)
            publish_task = asyncio.create_task(self.publish_sensor_data(session))
            config_task = asyncio.create_task(self.subscribe_to_config(session))

            try:
                await asyncio.gather(publish_task, config_task)
            except asyncio.CancelledError:
                self.cancel_event.set()
            finally:
                await publish_task
                await config_task
        except Exception as e:
            print(f"Error opening Zenoh session: {e}")
        finally:
            if "session" in locals():
                session.close()


def dataclass_to_dict(obj):
    return {
        field.name: getattr(obj, field.name)
        for field in obj.__dataclass_fields__.values()
    }


async def main():
    sensor_id = os.environ.get("SENSOR_ID", "unknown")
    zenoh_peer = os.environ.get("ZENOH_PEER", "tcp/localhost:7447")

    print(f"Starting sensor node with ID: {sensor_id}")
    print(f"Connecting to Zenoh peer: {zenoh_peer}")

    sensor_node = SensorNode(sensor_id, zenoh_peer)
    await sensor_node.run()


if __name__ == "__main__":
    asyncio.run(main())
