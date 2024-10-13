import asyncio
import pytest
from zenoh import Session, Config
from fabric import Node
from fabric.node.interface import NodeConfig
from examples.example_random_int_node import RandomIntNode, main


@pytest.fixture
def node_id():
    return "test_random_int_node"


@pytest.fixture
def initial_config():
    return {"publish_rate": 0.1}  # Fast publish rate for testing


@pytest.fixture
def node_config(node_id, initial_config):
    return NodeConfig(node_id=node_id, config=initial_config)


@pytest.fixture
async def zenoh_session():
    config = Config()
    session = Session(config)
    yield session
    if hasattr(session, "close") and callable(session.close):
        session.close()


@pytest.fixture
async def random_int_node(node_id, initial_config):
    return RandomIntNode(node_id, initial_config)


@pytest.fixture
async def fabric_node(node_id, node_config, zenoh_session, random_int_node):
    node = Node(node_id, "random_int", node_config, zenoh_session)
    node.interface = random_int_node
    return node


@pytest.mark.asyncio
async def test_random_int_node_initialization(random_int_node, node_id, initial_config):
    assert random_int_node.node_id == node_id
    assert random_int_node.config.node_id == node_id
    assert random_int_node.config.config == initial_config
    assert random_int_node.publish_rate == initial_config["publish_rate"]


@pytest.mark.asyncio
async def test_random_int_node_get_config(random_int_node):
    config = await random_int_node.get_config()
    assert isinstance(config, NodeConfig)
    assert config.node_id == random_int_node.node_id


@pytest.mark.asyncio
async def test_random_int_node_set_config(random_int_node):
    new_config = NodeConfig(
        node_id=random_int_node.node_id, config={"publish_rate": 2.0}
    )
    await random_int_node.set_config(new_config)
    assert random_int_node.publish_rate == 2.0


@pytest.mark.asyncio
async def test_random_int_node_get_type(random_int_node):
    assert random_int_node.get_type() == "random_int"


@pytest.mark.asyncio
async def test_random_int_node_run(fabric_node, random_int_node):
    published_data = []

    async def mock_publish(topic, data):
        published_data.append(data)

    fabric_node.publish = mock_publish

    cancel_token = asyncio.Event()
    run_task = asyncio.create_task(random_int_node.run(fabric_node, cancel_token))
    await asyncio.sleep(0.3)
    cancel_token.set()
    await run_task

    assert len(published_data) > 0
    for data in published_data:
        node_data = eval(data)
        assert node_data["node_id"] == random_int_node.node_id
        assert node_data["node_type"] == "random_int"
        assert "timestamp" in node_data
        assert "value" in node_data["metadata"]
        assert 0 <= node_data["metadata"]["value"] <= 100


@pytest.mark.asyncio
async def test_random_int_node_handle_event(random_int_node):
    # Test that handle_event doesn't raise an exception
    await random_int_node.handle_event("test_event", {"test": "payload"})


@pytest.mark.asyncio
async def test_random_int_node_update_config(random_int_node):
    new_config = NodeConfig(
        node_id=random_int_node.node_id, config={"publish_rate": 3.0}
    )
    await random_int_node.update_config(new_config)
    assert random_int_node.publish_rate == 3.0


@pytest.mark.asyncio
async def test_random_int_node_run_cancellation(fabric_node, random_int_node):
    cancel_token = asyncio.Event()
    run_task = asyncio.create_task(random_int_node.run(fabric_node, cancel_token))
    cancel_token.set()
    await run_task
    # Test passes if no exception is raised


@pytest.mark.asyncio
async def test_main_function(monkeypatch):
    class MockSession:
        def __init__(self, config=None):
            pass

        def close(self):
            pass

        def declare_publisher(self, topic):
            return MockPublisher()

        def declare_subscriber(self, topic, callback):
            return MockSubscriber()

    class MockPublisher:
        async def put(self, data):
            pass

        def undeclare(self):
            pass

    class MockSubscriber:
        def undeclare(self):
            pass

    class MockNode:
        def __init__(self, *args, **kwargs):
            self.interface = None

        async def run(self, cancel_token):
            pass

        async def cleanup(self):
            pass

    monkeypatch.setattr(Session, "__init__", MockSession.__init__)
    monkeypatch.setattr(Session, "close", MockSession.close)
    monkeypatch.setattr(Session, "declare_publisher", MockSession.declare_publisher)
    monkeypatch.setattr(Session, "declare_subscriber", MockSession.declare_subscriber)
    monkeypatch.setattr(Node, "__init__", MockNode.__init__)
    monkeypatch.setattr(Node, "run", MockNode.run)
    monkeypatch.setattr(Node, "cleanup", MockNode.cleanup)

    main_task = asyncio.create_task(main())
    await asyncio.sleep(0.1)
    main_task.cancel()

    try:
        await main_task
    except asyncio.CancelledError:
        pass

    assert True


@pytest.mark.asyncio
async def test_main_function_keyboard_interrupt(monkeypatch):
    class MockSession:
        def __init__(self, config=None):
            pass

        def close(self):
            pass

    class MockNode:
        def __init__(self, *args, **kwargs):
            self.interface = None

        async def run(self, cancel_token):
            raise KeyboardInterrupt

        async def cleanup(self):
            pass

    monkeypatch.setattr(Session, "__init__", MockSession.__init__)
    monkeypatch.setattr(Session, "close", MockSession.close)
    monkeypatch.setattr(Node, "__init__", MockNode.__init__)
    monkeypatch.setattr(Node, "run", MockNode.run)
    monkeypatch.setattr(Node, "cleanup", MockNode.cleanup)

    await main()
    assert True  # If we get here without an exception, the test passes
