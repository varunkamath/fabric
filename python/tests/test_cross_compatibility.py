import asyncio
import pytest
import zenoh
from fabric import Orchestrator
from fabric.node.interface import NodeConfig, NodeData
import json

@pytest.mark.asyncio
async def test_python_orchestrator_rust_node():
    config = zenoh.Config()
    session = zenoh.open(config)

    try:
        # Create a mock Rust node
        rust_node = MockRustNode("rust_node", session)

        # Start the Python orchestrator and Rust node
        cancel_token = asyncio.Event()
        orchestrator = Orchestrator("test_orchestrator", session)
        orchestrator_task = asyncio.create_task(orchestrator.run(cancel_token))
        node_task = asyncio.create_task(rust_node.run(cancel_token))

        # Wait for initialization
        await asyncio.sleep(1)

        # Test communication between Python orchestrator and Rust node
        test_config = NodeConfig(node_id="rust_node", config={"key": "value"})
        await orchestrator.publish_node_config("rust_node", test_config)

        # Wait for the node to process the new config
        await asyncio.sleep(1)

        # Verify that the Rust node received and applied the new config
        updated_config = await asyncio.wait_for(rust_node.get_config(), timeout=5.0)
        assert updated_config.config.get("key") == "value"

    finally:
        # Clean up
        cancel_token.set()
        await asyncio.gather(orchestrator_task, node_task, return_exceptions=True)
        if hasattr(rust_node, 'subscriber') and rust_node.subscriber:
            rust_node.subscriber.undeclare()
        session.close()

class MockRustNode:
    def __init__(self, node_id: str, session: zenoh.Session):
        self.node_id = node_id
        self.session = session
        self.config = NodeConfig(node_id=node_id, config={})
        self.subscriber = None

    async def run(self, cancel_token: asyncio.Event) -> None:
        self.subscriber = self.session.declare_subscriber(f"node/{self.node_id}/config", self.handle_config_update)
        try:
            while not cancel_token.is_set():
                await asyncio.sleep(0.1)
        finally:
            if self.subscriber:
                self.subscriber.undeclare()

    async def get_config(self) -> NodeConfig:
        return self.config

    def handle_config_update(self, sample):
        new_config = json.loads(sample.payload.decode())
        self.config = NodeConfig(node_id=self.node_id, config=new_config['config'])