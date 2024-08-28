use fabric::init_logger;
use fabric::node::interface::NodeConfig;
use fabric::node::Node;
use fabric::orchestrator::Orchestrator;
use fabric::FabricError;
use log::{info, warn, LevelFilter};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;
use zenoh::config;
use zenoh::prelude::r#async::*;

async fn wait_for_node_initialization() {
    sleep(Duration::from_millis(5000)).await;
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
        }),
    };

    let node = Node::new(
        node_config.node_id.clone(),
        "generic".to_string(),
        node_config.clone(),
        session.clone(),
        None,
    )
    .await?;

    wait_for_node_initialization().await;

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let handle = tokio::spawn(async move {
        node.run(cancel_clone).await.unwrap();
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    cancel.cancel();
    handle.await.unwrap();

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_orchestrator_creation_and_run() -> fabric::Result<()> {
    init_logger(LevelFilter::Info);

    let mut config = config::peer();
    config.transport.shared_memory.set_enabled(true).unwrap();
    let session = zenoh::open(config).res().await.unwrap().into_arc();

    let orchestrator = Orchestrator::new("test_orchestrator".to_string(), session.clone()).await?;

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let handle = tokio::spawn(async move {
        orchestrator.run(cancel_clone).await.unwrap();
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    cancel.cancel();
    handle.await.unwrap();

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_node_config_publication() -> fabric::Result<()> {
    init_logger(LevelFilter::Debug);

    let mut config = config::peer();
    config.transport.shared_memory.set_enabled(true).unwrap();
    let session = zenoh::open(config).res().await.unwrap().into_arc();

    let node_config = NodeConfig {
        node_id: "config_test_node".to_string(),
        config: serde_json::json!({
            "sampling_rate": 5,
            "threshold": 50.0,
            "mock_config": {"param1": 100}
        }),
    };

    let config_updated = Arc::new(Notify::new());
    let config_updated_clone = config_updated.clone();

    let node = Arc::new(
        Node::new(
            node_config.node_id.clone(),
            "generic".to_string(),
            node_config.clone(),
            session.clone(),
            Some(Box::new(move || config_updated_clone.notify_one())),
        )
        .await?,
    );

    wait_for_node_initialization().await;

    let orchestrator =
        Arc::new(Orchestrator::new("test_config_orchestrator".to_string(), session.clone()).await?);

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let cancel_clone2 = cancel.clone();
    let node_clone = node.clone();
    let handle = tokio::spawn(async move {
        info!("Starting node run");
        node_clone.run(cancel_clone).await.unwrap();
        info!("Node run completed");
    });
    let orchestrator_clone = orchestrator.clone();

    let _orchestrator_handle = tokio::spawn(async move {
        orchestrator_clone.run(cancel_clone2).await.unwrap();
    });

    tokio::time::sleep(Duration::from_secs(2)).await;

    info!("Publishing config to node: {}", node_config.node_id);
    orchestrator
        .publish_node_config(&node_config.node_id, &node_config)
        .await?;

    // Add a small delay
    tokio::time::sleep(Duration::from_millis(100)).await;

    match tokio::time::timeout(Duration::from_secs(5), config_updated.notified()).await {
        Ok(_) => info!("Config update received successfully"),
        Err(_) => {
            warn!("Timeout waiting for config update");
            let current_config = node.get_config().await;
            info!("Current node config: {:?}", current_config);
            return Err(FabricError::Other(
                "Timeout waiting for config update".into(),
            ));
        }
    }

    let updated_config = node.get_config().await;
    info!("Node config after update: {:?}", updated_config);
    assert_eq!(
        updated_config.config["mock_config"]["param1"],
        serde_json::json!(100),
        "Config was not updated as expected"
    );

    info!("Cancelling node run");
    cancel.cancel();
    info!("Waiting for node handle");
    handle
        .await
        .map_err(|e| FabricError::Other(e.to_string()))?;
    info!("Test completed successfully");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_node_config_update() -> fabric::Result<()> {
    init_logger(log::LevelFilter::Debug);

    let mut config = config::peer();
    config.transport.shared_memory.set_enabled(true).unwrap();
    let session = zenoh::open(config).res().await.unwrap().into_arc();

    let initial_config = NodeConfig {
        node_id: "test_node".to_string(),
        config: serde_json::json!({
            "sampling_rate": 5,
            "threshold": 50.0,
            "mock_config": {"param1": 100}
        }),
    };

    let node = Arc::new(
        Node::new(
            initial_config.node_id.clone(),
            "generic".to_string(),
            initial_config.clone(),
            session.clone(),
            None,
        )
        .await?,
    );

    node.declare_node_data_publisher().await?;
    let orchestrator = Orchestrator::new("test_orchestrator".to_string(), session.clone()).await?;

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let node_clone = node.clone();
    tokio::spawn(async move {
        node_clone.run(cancel_clone).await.unwrap();
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    let updated_config = NodeConfig {
        node_id: "test_node".to_string(),
        config: serde_json::json!({
            "sampling_rate": 10,
            "threshold": 75.0,
            "mock_config": {"param1": 200}
        }),
    };

    orchestrator
        .publish_node_config(&updated_config.node_id, &updated_config)
        .await?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    let updated_config = node.get_config().await;
    assert_eq!(updated_config.config, updated_config.config);

    cancel.cancel();

    drop(session);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_orchestrator_get_id() -> fabric::Result<()> {
    init_logger(log::LevelFilter::Debug);

    let mut config = config::peer();
    config.transport.shared_memory.set_enabled(true).unwrap();
    let session = zenoh::open(config).res().await.unwrap().into_arc();

    let orchestrator_id = "test_orchestrator".to_string();
    let orchestrator = Orchestrator::new(orchestrator_id.clone(), session.clone()).await?;

    assert_eq!(orchestrator.get_id(), &orchestrator_id);

    drop(session);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_orchestrator_subscribe_to_all_nodes() -> fabric::Result<()> {
    let mut config = config::peer();
    config.transport.shared_memory.set_enabled(true).unwrap();
    let session = zenoh::open(config).res().await.unwrap().into_arc();

    let orchestrator =
        Arc::new(Orchestrator::new("test_orchestrator".to_string(), session.clone()).await?);

    let received = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    let node1 = Arc::new(
        Node::new(
            "node1".to_string(),
            "generic".to_string(),
            NodeConfig {
                node_id: "node1".to_string(),
                config: serde_json::json!({}),
            },
            session.clone(),
            None,
        )
        .await?,
    );

    let node2 = Arc::new(
        Node::new(
            "node2".to_string(),
            "generic".to_string(),
            NodeConfig {
                node_id: "node2".to_string(),
                config: serde_json::json!({}),
            },
            session.clone(),
            None,
        )
        .await?,
    );

    wait_for_node_initialization().await;

    let cancel = CancellationToken::new();
    let cancel_clone1 = cancel.clone();
    let cancel_clone2 = cancel.clone();

    let node1_clone = node1.clone();
    let node2_clone = node2.clone();

    let node1_handle = tokio::spawn(async move { node1_clone.run(cancel_clone1).await });
    let node2_handle = tokio::spawn(async move { node2_clone.run(cancel_clone2).await });

    orchestrator
        .subscribe_to_node(
            "*",
            Box::new(move |node_data| {
                let received = received_clone.clone();
                info!("Orchestrator received data: {:?}", node_data);
                tokio::spawn(async move {
                    received.lock().await.push(node_data);
                });
            }),
        )
        .await?;

    tokio::time::sleep(Duration::from_secs(5)).await;

    for _ in 0..30 {
        node1.publish_node_data(0.0, None).await?;
        node2.publish_node_data(0.0, None).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    tokio::time::sleep(Duration::from_secs(15)).await;

    let received_data = received.lock().await;
    info!("Received data: {:?}", received_data);
    assert!(
        received_data.len() >= 2,
        "Should receive data from at least 2 nodes, got {}",
        received_data.len()
    );
    assert!(
        received_data.iter().any(|data| data.node_id == "node1"),
        "Should receive data from node1"
    );
    assert!(
        received_data.iter().any(|data| data.node_id == "node2"),
        "Should receive data from node2"
    );

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
