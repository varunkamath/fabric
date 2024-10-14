import asyncio
import logging
import random
import time
from typing import Dict, Any
from zenoh import Config, Session
from fabric import Node
from fabric.node.interface import NodeInterface, NodeConfig, NodeData

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class QuadcopterConfig:
    def __init__(
        self,
        max_altitude: float,
        max_speed: float,
        home_position: list[float],
        battery_threshold: float,
    ):
        self.max_altitude = max_altitude
        self.max_speed = max_speed
        self.home_position = home_position
        self.battery_threshold = battery_threshold


class QuadcopterCommand:
    MOVE_TO = "move_to"
    LAND = "land"
    TAKE_OFF = "take_off"


class QuadcopterNode(NodeInterface):
    def __init__(self, node_id: str, initial_config: Dict[str, Any]):
        self.node_id = node_id
        self.config = NodeConfig(node_id=node_id, config=initial_config)
        self.quadcopter_config = QuadcopterConfig(**initial_config["quadcopter_config"])
        self.altitude = 0.0
        self.battery_level = 100.0
        self.command_mode = "idle"

    async def get_config(self) -> NodeConfig:
        return self.config

    async def set_config(self, config: NodeConfig) -> None:
        self.config = config
        self.quadcopter_config = QuadcopterConfig(**config.config["quadcopter_config"])
        logger.info(f"Updated quadcopter config: {self.quadcopter_config.__dict__}")

    def get_type(self) -> str:
        return "quadcopter"

    async def handle_event(self, event: str, payload: Any) -> None:
        if event == QuadcopterCommand.MOVE_TO:
            self.command_mode = "moving"
            logger.info(f"Moving to position: {payload}")
        elif event == QuadcopterCommand.LAND:
            self.command_mode = "landing"
            logger.info("Landing quadcopter")
        elif event == QuadcopterCommand.TAKE_OFF:
            self.command_mode = "taking_off"
            logger.info("Taking off")
        else:
            logger.warning(f"Unknown event: {event}")

    async def update_config(self, config: NodeConfig) -> None:
        await self.set_config(config)

    async def run(self, node: Node, cancel_token: asyncio.Event) -> None:
        telemetry_topic = f"node/{self.node_id}/quadcopter/telemetry"
        await node.create_publisher(telemetry_topic)
        logger.info(f"Created publisher for topic: {telemetry_topic}")

        while not cancel_token.is_set():
            self.altitude += random.uniform(-0.1, 0.1)
            self.battery_level -= random.uniform(0.1, 0.5)

            if self.battery_level < self.quadcopter_config.battery_threshold:
                logger.warning("Low battery! Returning to home position.")
                self.command_mode = "returning_home"

            telemetry_data = {
                "altitude": self.altitude,
                "battery_level": self.battery_level,
                "command_mode": self.command_mode,
            }

            node_data = NodeData(
                node_id=self.node_id,
                node_type=self.get_type(),
                timestamp=int(time.time()),
                metadata=telemetry_data,
                status="online",
            )

            await node.publish(telemetry_topic, node_data.to_json())
            logger.info(
                f"Published telemetry data to {telemetry_topic}: {node_data.to_json()}"
            )
            await asyncio.sleep(1)


async def create_zenoh_session() -> Session:
    config = Config()
    session = Session(config)
    info = session.info()
    logger.info(f"Zenoh session created with ZID: {info.zid()}")
    return session


async def main():
    session = await create_zenoh_session()

    initial_config = {
        "quadcopter_config": {
            "max_altitude": 100.0,
            "max_speed": 10.0,
            "home_position": [0.0, 0.0, 0.0],
            "battery_threshold": 20.0,
        }
    }

    node_config = NodeConfig(node_id="quadcopter_1", config=initial_config)
    quadcopter_node = QuadcopterNode("quadcopter_1", initial_config)
    node = Node("quadcopter_1", "quadcopter", node_config, session)
    node.interface = quadcopter_node

    cancel_token = asyncio.Event()
    try:
        await node.run(cancel_token)
    except KeyboardInterrupt:
        logger.info("Stopping quadcopter node...")
    finally:
        cancel_token.set()
        await node.cleanup()
        await session.close()


if __name__ == "__main__":
    asyncio.run(main())
