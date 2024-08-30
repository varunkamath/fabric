use fabric::error::Result;
use fabric::node::interface::NodeConfig;
use fabric::node::Node;
use fabric::orchestrator::Orchestrator;
use serde_json::json;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

async fn create_test_session() -> Arc<Session> {
    Arc::new(
        zenoh::open(zenoh::config::Config::default())
            .res()
            .await
            .unwrap(),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_orchestrator_creation() -> Result<()> {
    let session = create_test_session().await;
    let orchestrator = Orchestrator::new("test_orchestrator".to_string(), session).await?;
    assert_eq!(orchestrator.get_id(), "test_orchestrator");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_temperature_node_config() -> Result<()> {
    let session = create_test_session().await;
    let orchestrator = Orchestrator::new("test_orchestrator".to_string(), session.clone()).await?;

    let node_config = NodeConfig {
        node_id: "temp_sensor_1".to_string(),
        config: json!({
            "sampling_rate": 5,
            "threshold": 30.0
        }),
    };

    let _node = Node::new(
        "temp_sensor_1".to_string(),
        "temperature".to_string(),
        node_config.clone(),
        session.clone(),
        None,
    )
    .await?;

    orchestrator
        .subscribe_to_node(
            "temp_sensor_1",
            Box::new(|data| {
                println!("Received data from temperature node: {:?}", data);
            }),
        )
        .await?;

    // Publish a test configuration update
    orchestrator
        .publish_node_config("temp_sensor_1", &node_config)
        .await?;

    // In a real scenario, you'd wait for the node to process the config update
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_humidity_node_config() -> Result<()> {
    let session = create_test_session().await;
    let orchestrator = Orchestrator::new("test_orchestrator".to_string(), session.clone()).await?;

    let node_config = NodeConfig {
        node_id: "humidity_sensor_1".to_string(),
        config: json!({
            "sampling_rate": 15,
            "threshold": 60.0
        }),
    };

    let _node = Node::new(
        "humidity_sensor_1".to_string(),
        "humidity".to_string(),
        node_config.clone(),
        session.clone(),
        None,
    )
    .await?;

    orchestrator
        .subscribe_to_node(
            "humidity_sensor_1",
            Box::new(|data| {
                println!("Received data from humidity node: {:?}", data);
            }),
        )
        .await?;

    // Publish a test configuration update
    orchestrator
        .publish_node_config("humidity_sensor_1", &node_config)
        .await?;

    // In a real scenario, you'd wait for the node to process the config update
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_radio_node_config() -> Result<()> {
    let session = create_test_session().await;
    let orchestrator = Orchestrator::new("test_orchestrator".to_string(), session.clone()).await?;

    let node_config = NodeConfig {
        node_id: "radio_sensor_1".to_string(),
        config: json!({
            "sampling_rate": 1,
            "threshold": -80.0,
            "radio_config": {
                "frequency": 433.0,
                "modulation": "FSK",
                "bandwidth": 125.0,
                "tx_power": 14
            },
            "mode": "receive"
        }),
    };

    let _node = Node::new(
        "radio_sensor_1".to_string(),
        "radio".to_string(),
        node_config.clone(),
        session.clone(),
        None,
    )
    .await?;

    orchestrator
        .subscribe_to_node(
            "radio_sensor_1",
            Box::new(|data| {
                println!("Received data from radio node: {:?}", data);
            }),
        )
        .await?;

    // Publish a test configuration update
    orchestrator
        .publish_node_config("radio_sensor_1", &node_config)
        .await?;

    // In a real scenario, you'd wait for the node to process the config update
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_orchestrator_run() -> Result<()> {
    let session = create_test_session().await;
    let orchestrator = Orchestrator::new("test_orchestrator".to_string(), session.clone()).await?;

    // Create nodes for each type
    let node_configs = vec![
        (
            "temp_sensor_1",
            "temperature",
            json!({"sampling_rate": 5, "threshold": 30.0}),
        ),
        (
            "humidity_sensor_1",
            "humidity",
            json!({"sampling_rate": 15, "threshold": 60.0}),
        ),
        (
            "radio_sensor_1",
            "radio",
            json!({
                "sampling_rate": 1,
                "threshold": -80.0,
                "radio_config": {
                    "frequency": 433.0,
                    "modulation": "FSK",
                    "bandwidth": 125.0,
                    "tx_power": 14
                },
                "mode": "receive"
            }),
        ),
    ];

    for (id, node_type, config) in node_configs {
        let node_config = NodeConfig {
            node_id: id.to_string(),
            config,
        };

        let _node = Node::new(
            id.to_string(),
            node_type.to_string(),
            node_config.clone(),
            session.clone(),
            None,
        )
        .await?;

        orchestrator
            .subscribe_to_node(
                id,
                Box::new(move |data| {
                    println!("Received data from {} node: {:?}", node_type, data);
                }),
            )
            .await?;
    }

    // Run the orchestrator for a short time
    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();

    let orchestrator_task = tokio::spawn(async move { orchestrator.run(cancel_token).await });

    // Let the orchestrator run for a short time
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Stop the orchestrator
    cancel_token_clone.cancel();

    // Wait for the orchestrator task to complete
    orchestrator_task.await??;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_orchestrator_config_updates() -> Result<()> {
    let session = create_test_session().await;
    let orchestrator = Orchestrator::new("test_orchestrator".to_string(), session.clone()).await?;

    // Create test nodes
    let node_configs = vec![
        (
            "temp_sensor_1",
            "temperature",
            json!({"sampling_rate": 5, "threshold": 30.0}),
        ),
        (
            "humidity_sensor_1",
            "humidity",
            json!({"sampling_rate": 15, "threshold": 60.0}),
        ),
        (
            "radio_sensor_1",
            "radio",
            json!({
                "sampling_rate": 1,
                "threshold": -80.0,
                "radio_config": {
                    "frequency": 433.0,
                    "modulation": "FSK",
                    "bandwidth": 125.0,
                    "tx_power": 14
                },
                "mode": "receive"
            }),
        ),
    ];

    let mut nodes = Vec::new();

    for (id, node_type, config) in node_configs {
        let node_config = NodeConfig {
            node_id: id.to_string(),
            config: config.clone(),
        };

        let node = Node::new(
            id.to_string(),
            node_type.to_string(),
            node_config.clone(),
            session.clone(),
            None,
        )
        .await?;

        nodes.push(node);

        orchestrator
            .subscribe_to_node(
                id,
                Box::new(move |data| {
                    println!("Received data from {} node: {:?}", node_type, data);
                }),
            )
            .await?;
    }

    // Prepare updated configs
    let updated_configs = vec![
        (
            "temp_sensor_1",
            json!({"sampling_rate": 10, "threshold": 35.0}),
        ),
        (
            "humidity_sensor_1",
            json!({"sampling_rate": 20, "threshold": 70.0}),
        ),
        (
            "radio_sensor_1",
            json!({
                "sampling_rate": 2,
                "threshold": -75.0,
                "radio_config": {
                    "frequency": 915.0,
                    "modulation": "LoRa",
                    "bandwidth": 250.0,
                    "tx_power": 20
                },
                "mode": "transmit"
            }),
        ),
    ];

    // Update configs
    for (id, config) in updated_configs {
        let node_config = NodeConfig {
            node_id: id.to_string(),
            config,
        };
        orchestrator.publish_node_config(id, &node_config).await?;
    }

    // Verify updated configs with retry
    for node in nodes {
        let node_id = node.get_id().to_string();
        let result = timeout(Duration::from_secs(5), async {
            loop {
                let updated_config = node.get_config().await;
                match node_id.as_str() {
                    "temp_sensor_1" => {
                        if updated_config.config["sampling_rate"] == 10
                            && updated_config.config["threshold"] == 35.0
                        {
                            println!("Temperature sensor updated config: {:?}", updated_config);
                            // Return Ok(()) as Result<()>
                            return Ok(()) as Result<()>;
                        }
                    }
                    "humidity_sensor_1" => {
                        if updated_config.config["sampling_rate"] == 20
                            && updated_config.config["threshold"] == 70.0
                        {
                            println!("Humidity sensor updated config: {:?}", updated_config);
                            // Return Ok(()) as Result<()>
                            return Ok(()) as Result<()>;
                        }
                    }
                    "radio_sensor_1" => {
                        if updated_config.config["sampling_rate"] == 2
                            && updated_config.config["threshold"] == -75.0
                            && updated_config.config["radio_config"]["frequency"] == 915.0
                            && updated_config.config["radio_config"]["modulation"] == "LoRa"
                            && updated_config.config["radio_config"]["bandwidth"] == 250.0
                            && updated_config.config["radio_config"]["tx_power"] == 20
                            && updated_config.config["mode"] == "transmit"
                        {
                            println!("Radio sensor updated config: {:?}", updated_config);
                            // Return Ok(()) as Result<()>
                            return Ok(()) as Result<()>;
                        }
                    }
                    // Do nothing if the node ID is not recognized
                    _ => (),
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await;

        assert!(
            result.is_ok(),
            "Configuration update for {} did not propagate in time",
            node_id
        );
    }

    Ok(())
}
