import asyncio
import random
import time
import logging
from zenoh import Config, Session
from fabric import Node
from fabric.node.interface import NodeInterface, NodeConfig, NodeData


class RandomIntNode(NodeInterface):
    def __init__(self, node_id: str, initial_config: dict):
        self.node_id = node_id
        self.config = NodeConfig(node_id=node_id, config=initial_config)
        self.publish_rate = initial_config.get(
            "publish_rate", 1.0
        )  # Default to 1 second

    async def get_config(self) -> NodeConfig:
        return self.config

    async def set_config(self, config: NodeConfig) -> None:
        self.config = config
        self.publish_rate = config.config.get("publish_rate", self.publish_rate)

    def get_type(self) -> str:
        return "random_int"

    async def run(self, node: Node, cancel_token: asyncio.Event) -> None:
        while not cancel_token.is_set():
            random_int = random.randint(0, 100)
            node_data = NodeData(
                node_id=self.node_id,
                node_type=self.get_type(),
                timestamp=int(time.time()),
                metadata={"value": random_int},
                status="online",
            )
            topic = f"node/{self.node_id}/random_int/data"
            await node.publish(topic, node_data.to_json())
            print(
                f"Published to {topic}: {node_data.to_json()}"
            )  # Add this line for debugging
            await asyncio.sleep(self.publish_rate)

    async def handle_event(self, event: str, payload: dict) -> None:
        # Implement event handling logic here
        pass

    async def update_config(self, config: NodeConfig) -> None:
        await self.set_config(config)


async def create_zenoh_session() -> Session:
    config = Config()
    session = Session(config)
    info = session.info()
    logging.info(f"Zenoh session created with ZID: {info.zid()}")
    return session


async def main():
    session = await create_zenoh_session()

    initial_config = {"publish_rate": random.uniform(0.5, 2.0)}

    node_config = NodeConfig(
        node_id=f"random_int_node_{random.randint(1, 100)}", config=initial_config
    )
    random_int_node = RandomIntNode(node_config.node_id, initial_config)
    node = Node(node_config.node_id, "random_int", node_config, session)
    node.interface = random_int_node

    topic = f"node/{node_config.node_id}/random_int/data"
    await node.create_publisher(topic)
    print(f"Created publisher for topic: {topic}")  # Add this line for debugging

    cancel_token = asyncio.Event()
    try:
        await node.run(cancel_token)
    except KeyboardInterrupt:
        print("Stopping random int node...")
    finally:
        cancel_token.set()
        await node.cleanup()
        await session.close()
        logging.debug(f"Zenoh session closed: {session}")


if __name__ == "__main__":
    logging.basicConfig(level=logging.DEBUG)
    logger = logging.getLogger(__name__)
    asyncio.run(main())
