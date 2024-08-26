use async_trait::async_trait;
use fabric::error::Result;
use fabric::node::interface::{NodeConfig, NodeData, NodeFactory, NodeInterface};
use fabric::node::Node;
use fabric::orchestrator::{Orchestrator, OrchestratorConfig};
use fabric::plugins::register_node_type;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

struct MockNodeFactory;

impl NodeFactory for MockNodeFactory {
    fn create(&self, config: NodeConfig) -> Box<dyn NodeInterface> {
        Box::new(MockNode {
            config,
            value: Arc::new(Mutex::new(0.0)),
        })
    }
}

struct MockNode {
    config: NodeConfig,
    value: Arc<Mutex<f64>>,
}

#[async_trait]
impl NodeInterface for MockNode {
    async fn read(&self) -> Result<f64> {
        Ok(*self.value.lock().await)
    }

    fn get_config(&self) -> NodeConfig {
        self.config.clone()
    }

    fn set_config(&mut self, config: NodeConfig) {
        self.config = config;
    }

    fn get_type(&self) -> String {
        "mock".to_string()
    }

    async fn handle_event(&mut self, _event: &str, _payload: &str) -> Result<()> {
        Ok(())
    }
}

fn setup_mock_node() {
    register_node_type("mock", MockNodeFactory);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_node_creation_and_run() {
    setup_mock_node();
    let config = zenoh::config::Config::default();
    let session = zenoh::open(config).res().await.unwrap();
    let node_config = NodeConfig {
        node_id: "test_node".to_string(),
        sampling_rate: 5,
        threshold: 50.0,
        custom_config: serde_json::json!({"mock_config": {"param1": 100, "param2": "test"}}),
    };

    let node = Node::new(
        "test_node".to_string(),
        "mock".to_string(),
        node_config,
        Arc::new(session),
    )
    .await
    .unwrap();

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let handle = tokio::spawn(async move {
        node.run(cancel_clone).await.unwrap();
    });

    // Allow some time for the node to start
    tokio::time::sleep(Duration::from_secs(1)).await;

    cancel.cancel();
    handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_orchestrator_creation_and_run() {
    let config = zenoh::config::Config::default();
    let session = zenoh::open(config).res().await.unwrap();

    let orchestrator = Orchestrator::new("test_orchestrator".to_string(), Arc::new(session))
        .await
        .unwrap();

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let handle = tokio::spawn(async move {
        orchestrator.run(cancel_clone).await.unwrap();
    });

    // Allow some time for the orchestrator to start
    tokio::time::sleep(Duration::from_secs(1)).await;

    cancel.cancel();
    handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_node_config_publication() {
    setup_mock_node();
    let config = zenoh::config::Config::default();
    let session = Arc::new(zenoh::open(config).res().await.unwrap());

    let orchestrator = Orchestrator::new("test_orchestrator".to_string(), session.clone())
        .await
        .unwrap();

    let node_configs = vec![
        NodeConfig {
            node_id: "node1".to_string(),
            sampling_rate: 5,
            threshold: 50.0,
            custom_config: serde_json::json!({"mock_config": {"param1": 100}}),
        },
        NodeConfig {
            node_id: "node2".to_string(),
            sampling_rate: 10,
            threshold: 75.0,
            custom_config: serde_json::json!({"mock_config": {"param1": 200}}),
        },
    ];

    for config in &node_configs {
        orchestrator
            .publish_node_config(&config.node_id, config)
            .await
            .unwrap();
    }

    let received_data = Arc::new(Mutex::new(Vec::new()));

    for config in &node_configs {
        let node = Node::new(
            config.node_id.clone(),
            "mock".to_string(),
            config.clone(),
            session.clone(),
        )
        .await
        .unwrap();

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            node.run(cancel_clone).await.unwrap();
        });

        let received_data_clone = received_data.clone();
        let orchestrator_clone = orchestrator.clone();
        orchestrator_clone
            .subscribe_to_node(&config.node_id, move |data| {
                let received_data_clone = received_data_clone.clone();
                tokio::spawn(async move {
                    received_data_clone.lock().await.push(data);
                });
            })
            .await
            .unwrap();
    }

    // Allow some time for the nodes to publish data
    tokio::time::sleep(Duration::from_secs(5)).await;

    let received = received_data.lock().await;
    assert!(
        received.len() >= 2,
        "Should receive data from at least 2 nodes"
    );
    assert!(
        received.iter().any(|data| data.node_id == "node1"),
        "Should receive data from node1"
    );
    assert!(
        received.iter().any(|data| data.node_id == "node2"),
        "Should receive data from node2"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_node_config_update() {
    setup_mock_node();
    let config = zenoh::config::Config::default();
    let session = Arc::new(zenoh::open(config).res().await.unwrap());

    let initial_config = NodeConfig {
        node_id: "test_node".to_string(),
        sampling_rate: 5,
        threshold: 50.0,
        custom_config: serde_json::json!({"mock_config": {"param1": 100}}),
    };

    let node = Arc::new(
        Node::new(
            initial_config.node_id.clone(),
            "mock".to_string(),
            initial_config.clone(),
            session.clone(),
        )
        .await
        .unwrap(),
    );

    let orchestrator = Orchestrator::new("test_orchestrator".to_string(), session.clone())
        .await
        .unwrap();

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let node_clone = node.clone();
    tokio::spawn(async move {
        node_clone.run(cancel_clone).await.unwrap();
    });

    // Allow some time for the node to start
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Update node config
    let updated_config = NodeConfig {
        node_id: "test_node".to_string(),
        sampling_rate: 10,
        threshold: 75.0,
        custom_config: serde_json::json!({"mock_config": {"param1": 200}}),
    };

    orchestrator
        .publish_node_config(&updated_config.node_id, &updated_config)
        .await
        .unwrap();

    // Allow some time for the config update to be processed
    tokio::time::sleep(Duration::from_secs(2)).await;

    let updated_config = node.get_config().await;
    assert_eq!(updated_config.sampling_rate, 10);
    assert_eq!(updated_config.threshold, 75.0);

    cancel.cancel();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_concurrent_operations() {
    setup_mock_node();
    let config = zenoh::config::Config::default();
    let session = Arc::new(zenoh::open(config).res().await.unwrap());

    let orchestrator = Arc::new(
        Orchestrator::new("test_orchestrator".to_string(), session.clone())
            .await
            .unwrap(),
    );

    let node_configs = vec![
        NodeConfig {
            node_id: "node1".to_string(),
            sampling_rate: 5,
            threshold: 50.0,
            custom_config: serde_json::json!({"mock_config": {"param1": 100}}),
        },
        NodeConfig {
            node_id: "node2".to_string(),
            sampling_rate: 10,
            threshold: 75.0,
            custom_config: serde_json::json!({"mock_config": {"param1": 200}}),
        },
    ];

    let mut handles = vec![];

    for config in node_configs {
        let session_clone = session.clone();
        let orchestrator_clone = orchestrator.clone();
        handles.push(tokio::spawn(async move {
            let node = Node::new(
                config.node_id.clone(),
                "mock".to_string(),
                config.clone(),
                session_clone.clone(),
            )
            .await
            .unwrap();

            let cancel = CancellationToken::new();
            let cancel_clone = cancel.clone();
            tokio::spawn(async move {
                node.run(cancel_clone).await.unwrap();
            });

            // Publish some data
            for _ in 0..5 {
                let data = NodeData {
                    node_id: config.node_id.clone(),
                    node_type: "mock".to_string(),
                    value: rand::random::<f64>() * 100.0,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    metadata: None,
                };
                session_clone
                    .put("node/data", serde_json::to_string(&data).unwrap())
                    .res()
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            // Update node config
            let updated_config = NodeConfig {
                node_id: config.node_id.clone(),
                sampling_rate: config.sampling_rate * 2,
                threshold: config.threshold * 1.5,
                custom_config: config.custom_config.clone(),
            };
            orchestrator_clone
                .publish_node_config(&updated_config.node_id, &updated_config)
                .await
                .unwrap();

            cancel.cancel();
        }));
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // The test passes if no panics occur during concurrent operations
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_orchestrator_node_interaction() {
    setup_mock_node();
    let config = zenoh::config::Config::default();
    let session = Arc::new(zenoh::open(config).res().await.unwrap());

    let orchestrator = Arc::new(
        Orchestrator::new("test_orchestrator".to_string(), session.clone())
            .await
            .unwrap(),
    );

    let node_config = NodeConfig {
        node_id: "test_node".to_string(),
        sampling_rate: 5,
        threshold: 50.0,
        custom_config: serde_json::json!({"mock_config": {"param1": 100}}),
    };

    let node = Arc::new(
        Node::new(
            node_config.node_id.clone(),
            "mock".to_string(),
            node_config.clone(),
            session.clone(),
        )
        .await
        .unwrap(),
    );

    let cancel = CancellationToken::new();
    let cancel_clone1 = cancel.clone();
    let cancel_clone2 = cancel.clone();
    let node_clone = node.clone();
    tokio::spawn(async move {
        node_clone.run(cancel_clone1).await.unwrap();
    });

    let orchestrator_clone = orchestrator.clone();
    let handle = tokio::spawn(async move {
        orchestrator_clone.run(cancel_clone2).await.unwrap();
    });

    // Allow some time for the node and orchestrator to start
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Test node config update
    let updated_config = NodeConfig {
        node_id: "test_node".to_string(),
        sampling_rate: 10,
        threshold: 75.0,
        custom_config: serde_json::json!({"mock_config": {"param1": 200}}),
    };

    orchestrator
        .publish_node_config(&updated_config.node_id, &updated_config)
        .await
        .unwrap();

    // Allow some time for the config update to be processed
    tokio::time::sleep(Duration::from_secs(2)).await;

    let received_config = node.get_config().await;
    assert_eq!(received_config.sampling_rate, 10);
    assert_eq!(received_config.threshold, 75.0);

    // Test node data publication and orchestrator subscription
    let received_data = Arc::new(Mutex::new(Vec::new()));
    let received_data_clone = received_data.clone();

    orchestrator
        .subscribe_to_node(&node_config.node_id, move |data| {
            let received_data_clone = received_data_clone.clone();
            tokio::spawn(async move {
                received_data_clone.lock().await.push(data);
            });
        })
        .await
        .unwrap();

    // Simulate node publishing data
    for _ in 0..5 {
        let data = NodeData {
            node_id: node_config.node_id.clone(),
            node_type: "mock".to_string(),
            value: rand::random::<f64>() * 100.0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
        };
        session
            .put("node/data", serde_json::to_string(&data).unwrap())
            .res()
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Allow some time for the data to be processed
    tokio::time::sleep(Duration::from_secs(2)).await;

    let received = received_data.lock().await;
    assert!(received.len() >= 5, "Should receive at least 5 data points");

    cancel.cancel();
    handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_orchestrator_config() {
    setup_mock_node();
    let config = zenoh::config::Config::default();
    let session = Arc::new(zenoh::open(config).res().await.unwrap());

    let orchestrator = Arc::new(
        Orchestrator::new("test_orchestrator".to_string(), session.clone())
            .await
            .unwrap(),
    );

    let node_configs = vec![
        NodeConfig {
            node_id: "node1".to_string(),
            sampling_rate: 5,
            threshold: 50.0,
            custom_config: serde_json::json!({"mock_config": {"param1": 100}}),
        },
        NodeConfig {
            node_id: "node2".to_string(),
            sampling_rate: 10,
            threshold: 75.0,
            custom_config: serde_json::json!({"mock_config": {"param1": 200}}),
        },
    ];

    let orchestrator_config = OrchestratorConfig {
        nodes: node_configs.clone(),
    };

    orchestrator
        .publish_node_configs(&orchestrator_config)
        .await
        .unwrap();

    // Create nodes and verify their configs
    for config in node_configs {
        let node = Node::new(
            config.node_id.clone(),
            "mock".to_string(),
            config.clone(),
            session.clone(),
        )
        .await
        .unwrap();

        // Allow some time for the node to receive the config
        tokio::time::sleep(Duration::from_secs(1)).await;

        let received_config = node.get_config().await;
        assert_eq!(received_config.sampling_rate, config.sampling_rate);
        assert_eq!(received_config.threshold, config.threshold);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_node_event_handling() {
    setup_mock_node();
    let config = zenoh::config::Config::default();
    let session = Arc::new(zenoh::open(config).res().await.unwrap());

    let node_config = NodeConfig {
        node_id: "test_node".to_string(),
        sampling_rate: 5,
        threshold: 50.0,
        custom_config: serde_json::json!({"mock_config": {"param1": 100}}),
    };

    let node = Arc::new(
        Node::new(
            node_config.node_id.clone(),
            "mock".to_string(),
            node_config.clone(),
            session.clone(),
        )
        .await
        .unwrap(),
    );

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let node_clone = node.clone();
    tokio::spawn(async move {
        node_clone.run(cancel_clone).await.unwrap();
    });

    // Allow some time for the node to start
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Send an event to the node
    let event = "test_event";
    let payload = "test_payload";
    session
        .put(
            format!("node/{}/event/{}", node_config.node_id, event),
            payload,
        )
        .res()
        .await
        .unwrap();

    // Allow some time for the event to be processed
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Note: In the current implementation, we can't easily verify if the event was handled.
    // You might want to add a mechanism to check if events were received and handled correctly.

    cancel.cancel();
}
