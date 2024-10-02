import asyncio
import pytest
from zenoh import Config, Session
from fabric import Node, Orchestrator
from fabric.node.interface import NodeConfig, NodeData

@pytest.mark.asyncio
async def test_python_node_rust_orchestrator():
    # Create a Zenoh session
    config = Config()
    session = await Session.open(config)

    # Create a Python node
    node_config = NodeConfig(node_id="python_node", config={"key": "value"})
    python_node = Node("python_node", "test", node_config, session)

    # Create a mock Rust orchestrator (you'll need to implement this)
    rust_orchestrator = MockRustOrchestrator()

    # Start the Python node and Rust orchestrator
    cancel_token = asyncio.Event()
    node_task = asyncio.create_task(python_node.run(cancel_token))
    orchestrator_task = asyncio.create_task(rust_orchestrator.run(cancel_token))

    # Wait for initialization
    await asyncio.sleep(1)

    # Test communication between Python node and Rust orchestrator
    await rust_orchestrator.publish_node_config("python_node", NodeConfig(node_id="python_node", config={"updated_key": "updated_value"}))

    # Wait for the node to process the new config
    await asyncio.sleep(1)

    # Verify that the Python node received and applied the new config
    updated_config = await python_node.get_config()
    assert updated_config.config["updated_key"] == "updated_value"

    # Clean up
    cancel_token.set()
    await asyncio.gather(node_task, orchestrator_task)
    await session.close()

# You'll need to implement a MockRustOrchestrator class that simulates the behavior of a Rust orchestrator
class MockRustOrchestrator:
    async def run(self, cancel_token: asyncio.Event) -> None:
        # Implement mock run method
        pass

    async def publish_node_config(self, node_id: str, config: NodeConfig) -> None:
        # Implement mock publish_node_config method
        pass