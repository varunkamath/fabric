use fabric::error::Result;
use fabric::node::interface::NodeConfig;
use fabric::orchestrator::{Orchestrator, OrchestratorConfig};
use std::env;
use std::sync::Arc;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

#[tokio::main]
async fn main() -> Result<()> {
    let orchestrator_id =
        env::var("ORCHESTRATOR_ID").unwrap_or_else(|_| "example_orchestrator".to_string());

    println!("Starting example orchestrator: {}", orchestrator_id);

    let config = Config::default();
    let session = Arc::new(zenoh::open(config).res().await?);

    let orchestrator = Arc::new(Orchestrator::new(orchestrator_id.clone(), session.clone()).await?);

    let cancel = CancellationToken::new();
    let orchestrator_clone = orchestrator.clone();
    tokio::spawn(async move {
        orchestrator_clone.run(cancel).await.unwrap();
    });

    println!("Orchestrator {} is running...", orchestrator_id);

    // Example: Subscribe to all node data
    orchestrator
        .subscribe_to_node("*", |data| {
            println!("Received data from node {}: {:?}", data.node_id, data);
        })
        .await?;

    // Example: Publish initial node configurations
    let initial_configs = OrchestratorConfig {
        nodes: vec![
            NodeConfig {
                node_id: "node1".to_string(),
                sampling_rate: 5,
                threshold: 50.0,
                custom_config: serde_json::json!({"radio_config": {"frequency": 915.0}}),
            },
            NodeConfig {
                node_id: "node2".to_string(),
                sampling_rate: 10,
                threshold: 75.0,
                custom_config: serde_json::json!({"radio_config": {"frequency": 2400.0}}),
            },
        ],
    };

    orchestrator.publish_node_configs(&initial_configs).await?;

    // Example: Periodically update node configurations
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        let new_configs = vec![
            NodeConfig {
                node_id: "node1".to_string(),
                sampling_rate: 7,
                threshold: 60.0,
                custom_config: serde_json::json!({"radio_config": {"frequency": 915.0}}),
            },
            NodeConfig {
                node_id: "node2".to_string(),
                sampling_rate: 12,
                threshold: 80.0,
                custom_config: serde_json::json!({"radio_config": {"frequency": 2400.0}}),
            },
        ];

        for new_config in &new_configs {
            orchestrator
                .publish_node_config(&new_config.node_id, new_config)
                .await?;
        }
    }
}
