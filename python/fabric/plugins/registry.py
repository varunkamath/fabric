from fabric.sensor.interface import SensorInterface, SensorConfig
from fabric.plugins import SensorRegistry
import random


class RadioSensor(SensorInterface):
    def __init__(self, config: SensorConfig):
        self.config = config
        self.type = "radio"

    async def read(self) -> float:
        # Simulate reading from a radio sensor
        return random.uniform(0, 100)

    def get_type(self) -> str:
        return self.type

    def set_config(self, config: SensorConfig):
        self.config = config

    def get_config(self) -> SensorConfig:
        return self.config

    async def handle_event(self, event: str, payload: str):
        print(f"Radio sensor handling event: {event} with payload: {payload}")


class TemperatureSensor(SensorInterface):
    def __init__(self, config: SensorConfig):
        self.config = config
        self.type = "temperature"

    async def read(self) -> float:
        # Simulate reading from a temperature sensor
        return random.uniform(20, 30)

    def get_type(self) -> str:
        return self.type

    def set_config(self, config: SensorConfig):
        self.config = config

    def get_config(self) -> SensorConfig:
        return self.config

    async def handle_event(self, event: str, payload: str):
        print(f"Temperature sensor handling event: {event} with payload: {payload}")


# Register the sensors
registry = SensorRegistry()
registry.register_sensor("radio", RadioSensor)
registry.register_sensor("temperature", TemperatureSensor)
