use chrono::Utc;
use fabric::node::interface::NodeData;
use fabric::orchestrator::Orchestrator;
use log::{error, info, warn};
use serde_json::Value;
use std::error::Error;
use std::time::Duration;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::init();

    info!("Starting example orchestrator...");

    let config = zenoh::config::Config::default();
    let session = zenoh::open(config).res().await.unwrap().into_arc();

    let orchestrator =
        Orchestrator::new("example_orchestrator".to_string(), session.clone()).await?;

    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();

    let orchestrator_clone = orchestrator.clone();
    let orchestrator_handle = tokio::spawn(async move {
        if let Err(e) = orchestrator_clone.run(cancel_token_clone).await {
            error!("Orchestrator error: {:?}", e);
        }
    });

    info!("Subscribing to node data...");
    let subscriber = session.declare_subscriber("node/data").res().await.unwrap();

    let orchestrator_clone = orchestrator.clone();
    let subscriber_handle = tokio::spawn(async move {
        while let Ok(sample) = subscriber.recv_async().await {
            info!("Received sample on topic: {}", sample.key_expr);
            if let Ok(data) = serde_json::from_slice::<Value>(&sample.value.payload.contiguous()) {
                info!("Received data: {:?}", data);
                if let Some(node_id) = data["node_id"].as_str() {
                    let node_type = data["node_type"].as_str().unwrap_or("unknown");
                    info!("Updating state for node: {}", node_id);
                    orchestrator_clone
                        .update_node_state(NodeData {
                            node_id: node_id.to_string(),
                            node_type: node_type.to_string(),
                            status: "online".to_string(),
                            timestamp: Utc::now().timestamp() as u64,
                            metadata: Some(data.clone()),
                        })
                        .await;
                } else {
                    warn!("Received data without node_id: {:?}", data);
                }
            } else {
                warn!("Failed to parse received data as JSON");
            }
        }
    });

    // Spawn a task to log connected nodes every 5 seconds
    let orchestrator_clone = orchestrator.clone();
    let log_nodes_handle = tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            let nodes = orchestrator_clone.get_nodes().await;
            info!("Connected nodes: {:?}", nodes);
            for (node_id, node_state) in nodes.iter() {
                info!("Node {}: {:?}", node_id, node_state);
            }
        }
    });

    // Wait for a shutdown signal (e.g., Ctrl+C)
    tokio::signal::ctrl_c().await?;

    info!("Shutting down...");
    cancel_token.cancel();

    // Wait for all tasks to finish
    let _ = tokio::join!(orchestrator_handle, subscriber_handle, log_nodes_handle);

    Ok(())
}
