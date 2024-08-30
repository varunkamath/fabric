use fabric::error::FabricError;
use fabric::init_logger;
use fabric::node::interface::{NodeConfig, NodeData};
use fabric::node::Node;
use fabric::orchestrator::Orchestrator;
use log::{info, LevelFilter};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;
use zenoh::config;
use zenoh::prelude::r#async::*;
use zenoh::Session;

async fn wait_for_node_initialization() {
    sleep(Duration::from_millis(500)).await;
}

async fn create_zenoh_session() -> Arc<Session> {
    let mut config = config::peer();
    config.transport.shared_memory.set_enabled(true).unwrap();
    zenoh::open(config).res().await.unwrap().into_arc()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_node_creation_and_run() -> fabric::Result<()> {
    init_logger(LevelFilter::Info);

    let mut config = config::peer();
    config.transport.shared_memory.set_enabled(true).unwrap();
    let session = zenoh::open(config).res().await.unwrap().into_arc();

    let node_config = NodeConfig {
        node_id: "test_node".to_string(),
        config: serde_json::json!({
            "sampling_rate": 5,
            "threshold": 50.0,
            "mock_config": {"param1": 100}
        }),
    };

    let node = Arc::new(
        Node::new(
            node_config.node_id.clone(),
            "generic".to_string(),
            node_config.clone(),
            session.clone(),
            None,
        )
        .await?,
    );

    node.declare_node_data_publisher().await?;

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let node_clone = node.clone();
    tokio::spawn(async move {
        node_clone.run(cancel_clone).await.unwrap();
    });

    wait_for_node_initialization().await;

    let data = node.read().await?;
    info!("Read data from node: {}", data);

    let updated_config = NodeConfig {
        node_id: "test_node".to_string(),
        config: serde_json::json!({
            "sampling_rate": 10,
            "threshold": 75.0,
            "mock_config": {"param1": 200}
        }),
    };

    node.update_config(updated_config.clone()).await?;

    wait_for_node_initialization().await;

    let updated_data = node.read().await?;
    info!("Read updated data from node: {}", updated_data);

    cancel.cancel();

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_orchestrator_node_communication() -> fabric::Result<()> {
    init_logger(LevelFilter::Info);

    let mut config = config::peer();
    config.transport.shared_memory.set_enabled(true).unwrap();
    let session = zenoh::open(config).res().await.unwrap().into_arc();

    let orchestrator = Orchestrator::new("test_orchestrator".to_string(), session.clone()).await?;

    let node1_config = NodeConfig {
        node_id: "node1".to_string(),
        config: serde_json::json!({
            "sampling_rate": 5,
            "threshold": 50.0,
            "mock_config": {"param1": 100}
        }),
    };

    let node2_config = NodeConfig {
        node_id: "node2".to_string(),
        config: serde_json::json!({
            "sampling_rate": 10,
            "threshold": 75.0,
            "mock_config": {"param1": 200}
        }),
    };

    let node1 = Arc::new(
        Node::new(
            node1_config.node_id.clone(),
            "generic".to_string(),
            node1_config.clone(),
            session.clone(),
            None,
        )
        .await?,
    );

    let node2 = Arc::new(
        Node::new(
            node2_config.node_id.clone(),
            "generic".to_string(),
            node2_config.clone(),
            session.clone(),
            None,
        )
        .await?,
    );

    let cancel = CancellationToken::new();
    let cancel_clone1 = cancel.clone();
    let cancel_clone2 = cancel.clone();

    let node1_clone = node1.clone();
    let node2_clone = node2.clone();

    let node1_handle = tokio::spawn(async move { node1_clone.run(cancel_clone1).await });
    let node2_handle = tokio::spawn(async move { node2_clone.run(cancel_clone2).await });

    wait_for_node_initialization().await;

    orchestrator
        .publish_node_config(&node1_config.node_id, &node1_config)
        .await?;
    orchestrator
        .publish_node_config(&node2_config.node_id, &node2_config)
        .await?;

    wait_for_node_initialization().await;

    let node1_data = node1.read().await?;
    let node2_data = node2.read().await?;

    info!("Node 1 data: {}", node1_data);
    info!("Node 2 data: {}", node2_data);

    cancel.cancel();

    let _ = tokio::time::timeout(Duration::from_secs(5), node1_handle)
        .await
        .map_err(|_| FabricError::Other("Timeout waiting for node1 to finish".into()))?
        .map_err(|e| FabricError::Other(format!("Node1 join error: {}", e)))?;

    let _ = tokio::time::timeout(Duration::from_secs(5), node2_handle)
        .await
        .map_err(|_| FabricError::Other("Timeout waiting for node2 to finish".into()))?
        .map_err(|e| FabricError::Other(format!("Node2 join error: {}", e)))?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_node_failure_and_recovery() -> fabric::Result<()> {
    init_logger(LevelFilter::Info);

    let session = create_zenoh_session().await;
    let orchestrator = Arc::new(
        Orchestrator::new("test_failure_orchestrator".to_string(), session.clone()).await?,
    );

    let node_config = NodeConfig {
        node_id: "failing_node".to_string(),
        config: serde_json::json!({
            "sampling_rate": 5,
            "threshold": 50.0,
            "mock_config": {"param1": 100}
        }),
    };

    let node = Arc::new(
        Node::new(
            node_config.node_id.clone(),
            "generic".to_string(),
            node_config.clone(),
            session.clone(),
            None,
        )
        .await?,
    );

    let orchestrator_cancel = CancellationToken::new();
    let orchestrator_cancel_clone = orchestrator_cancel.clone();
    let orchestrator_clone = orchestrator.clone();
    let orchestrator_handle = tokio::spawn(async move {
        orchestrator_clone
            .run(orchestrator_cancel_clone)
            .await
            .unwrap();
    });

    wait_for_node_initialization().await;

    let node_cancel = CancellationToken::new();
    let node_cancel_clone = node_cancel.clone();
    let node_clone = node.clone();
    let node_handle = tokio::spawn(async move {
        node_clone.run(node_cancel_clone).await.unwrap();
    });

    wait_for_node_initialization().await;

    orchestrator
        .publish_node_config(&node_config.node_id, &node_config)
        .await?;

    // Stop node
    node_cancel.cancel();

    // Wait for the orchestrator to detect the failure
    sleep(Duration::from_millis(10050)).await;

    // Check if the orchestrator detected the failure
    {
        let nodes = orchestrator.nodes.lock().await;
        let node_state = nodes.get("failing_node").unwrap();
        assert_eq!(node_state.last_value.status, "offline");
    }

    // Start node again
    let node_clone = node.clone();
    let node_cancel_clone = node_cancel.clone();
    let node_handle = tokio::spawn(async move {
        node_clone.run(node_cancel_clone).await.unwrap();
    });

    // Wait for the orchestrator to detect the recovery
    sleep(Duration::from_millis(100)).await;

    // Check if the orchestrator detected the recovery
    {
        let nodes = orchestrator.nodes.lock().await;
        let node_state = nodes.get("failing_node").unwrap();
        assert_eq!(node_state.last_value.status, "online");
    }

    // Cancel orchestrator and node
    orchestrator_cancel.cancel();
    node_cancel.cancel();

    // Wait for tasks to complete with a timeout
    let _ = tokio::time::timeout(Duration::from_secs(5), orchestrator_handle).await;
    let _ = tokio::time::timeout(Duration::from_secs(5), node_handle).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_orchestrator_callback_functionality() -> Result<(), FabricError> {
    init_logger(LevelFilter::Info);

    let session = create_zenoh_session().await;
    let orchestrator = Orchestrator::new("test_orchestrator".to_string(), session.clone()).await?;
    let (tx, mut rx) = mpsc::channel(100);

    let callback = Arc::new(Mutex::new(move |node_data: NodeData| {
        let tx = tx.clone();
        tokio::spawn(async move {
            tx.send(node_data).await.unwrap();
        });
    }));

    orchestrator
        .register_callback("test_node", callback)
        .await?;

    // Simulate node data update
    let node_data = NodeData {
        node_id: "test_node".to_string(),
        status: "active".to_string(),
        node_type: "radio".to_string(),
        timestamp: 1234567890,
        metadata: None,
    };
    orchestrator.update_node_state(node_data.clone()).await;

    // Check if the callback was triggered
    let received_data = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .map_err(|_| FabricError::Other("Timeout waiting for callback".into()))?
        .ok_or_else(|| FabricError::Other("Channel closed".into()))?;

    assert_eq!(received_data.node_id, node_data.node_id);
    assert_eq!(received_data.status, node_data.status);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multi_node_config_application() -> fabric::Result<()> {
    init_logger(LevelFilter::Info);

    let session = create_zenoh_session().await;
    let orchestrator = Arc::new(
        Orchestrator::new("test_multi_node_orchestrator".to_string(), session.clone()).await?,
    );

    // Create 4 nodes with initial configurations
    let node_configs = vec![
        ("complex_node1", 5, 50.0, 100),
        ("complex_node2", 10, 75.0, 200),
        ("complex_node3", 15, 100.0, 300),
        ("complex_node4", 20, 125.0, 400),
    ];

    let mut nodes = Vec::new();
    for (id, sampling_rate, threshold, param1) in node_configs {
        let config = NodeConfig {
            node_id: id.to_string(),
            config: serde_json::json!({
                "sampling_rate": sampling_rate,
                "threshold": threshold,
                "mock_config": {"param1": param1}
            }),
        };

        let node = Arc::new(
            Node::new(
                config.node_id.clone(),
                "generic".to_string(),
                config.clone(),
                session.clone(),
                None,
            )
            .await?,
        );

        nodes.push((node, config));
    }

    // Start the orchestrator
    let orchestrator_cancel = CancellationToken::new();
    let orchestrator_cancel_clone = orchestrator_cancel.clone();
    let orchestrator_clone = orchestrator.clone();
    let orchestrator_handle = tokio::spawn(async move {
        orchestrator_clone
            .run(orchestrator_cancel_clone)
            .await
            .unwrap();
    });

    // Start all nodes
    let node_cancel = CancellationToken::new();
    let node_handles: Vec<_> = nodes
        .iter()
        .map(|(node, _)| {
            let node_clone = node.clone();
            let cancel_clone = node_cancel.clone();
            tokio::spawn(async move {
                node_clone.run(cancel_clone).await.unwrap();
            })
        })
        .collect();

    // Wait for initialization
    wait_for_node_initialization().await;

    // Publish initial configurations
    for (_, config) in &nodes {
        orchestrator
            .publish_node_config(&config.node_id, config)
            .await?;
    }

    // Wait for config application
    wait_for_node_initialization().await;

    // Verify initial configurations
    for (node, config) in &nodes {
        let node_config = node.get_config().await;
        assert_eq!(node_config.node_id, config.node_id);
        assert_eq!(node_config.config, config.config);
    }

    // Update configurations
    for (node, config) in &mut nodes {
        let mut new_config = config.clone();
        let mut json_config: serde_json::Value =
            serde_json::from_value(config.config.clone()).unwrap();
        json_config["sampling_rate"] =
            serde_json::json!(json_config["sampling_rate"].as_i64().unwrap() * 2);
        json_config["threshold"] =
            serde_json::json!(json_config["threshold"].as_f64().unwrap() * 1.5);
        json_config["mock_config"]["param1"] =
            serde_json::json!(json_config["mock_config"]["param1"].as_i64().unwrap() + 50);
        new_config.config = json_config;

        orchestrator
            .publish_node_config(&new_config.node_id, &new_config)
            .await?;

        *config = new_config;
    }

    // Wait for config application
    wait_for_node_initialization().await;

    // Verify updated configurations
    for (node, config) in &nodes {
        let node_config = node.get_config().await;
        println!("Node config: {:?}", node_config);
        println!("Config: {:?}", config);
        assert_eq!(node_config.node_id, config.node_id);
        assert_eq!(node_config.config, config.config);

        // Additional assertions for specific config values
        let json_config: serde_json::Value = serde_json::from_value(config.config.clone()).unwrap();
        assert!(json_config["sampling_rate"].as_i64().unwrap() > 5);
        assert!(json_config["threshold"].as_f64().unwrap() > 50.0);
        assert!(json_config["mock_config"]["param1"].as_i64().unwrap() > 100);
    }

    // Cleanup
    orchestrator_cancel.cancel();
    node_cancel.cancel();

    // Wait for tasks to complete with a timeout
    let _ = tokio::time::timeout(Duration::from_secs(5), orchestrator_handle).await;
    for handle in node_handles {
        let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
    }

    Ok(())
}
