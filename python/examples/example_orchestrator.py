import asyncio
import logging
from zenoh import Config, Session
from fabric import Orchestrator
from fabric.node.interface import NodeConfig, NodeData

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

async def node_data_callback(node_data: NodeData):
    logger.info(f"Received data from node {node_data.node_id}: {node_data.metadata}")

async def main():
    config = Config()
    session = await Session.open(config)

    orchestrator = Orchestrator("main_orchestrator", session)

    cancel_token = asyncio.Event()
    orchestrator_task = asyncio.create_task(orchestrator.run(cancel_token))

    # Wait for the orchestrator to initialize
    await asyncio.sleep(1)

    # Register a callback for the quadcopter node
    await orchestrator.register_callback("quadcopter_1", node_data_callback)

    # Simulate sending commands to the quadcopter
    commands = [
        ("take_off", {}),
        ("move_to", {"position": [10.0, 20.0, 30.0]}),
        ("land", {})
    ]

    for command, payload in commands:
        await orchestrator.send_event_to_node("quadcopter_1", command, payload)
        await asyncio.sleep(5)

    # Run for a while to observe the quadcopter's behavior
    await asyncio.sleep(30)

    try:
        # Cancel the orchestrator task
        cancel_token.set()
        await orchestrator_task
    finally:
        await session.close()

if __name__ == "__main__":
    asyncio.run(main())