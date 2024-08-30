use fabric::error::{FabricError, Result};
use fabric::orchestrator::Orchestrator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

const CONFIG_TOPIC: &str = "fabric/config";

#[derive(Debug, Deserialize)]
struct CorrectionFactor {
    temperature: f64,
    factor: f64,
}

#[derive(Debug, Deserialize)]
struct RadioConfig {
    frequency: f64,
    modulation: String,
    bandwidth: f64,
    tx_power: i64,
}

#[derive(Debug, Deserialize)]
struct SensorConfig {
    id: String,
    #[serde(rename = "type")]
    sensor_type: String,
    config: SensorConfigData,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SensorConfigData {
    Temperature {
        sampling_rate: i64,
        threshold: f64,
        calibration_data: Vec<f64>,
    },
    Humidity {
        sampling_rate: i64,
        threshold: f64,
        correction_factors: Vec<CorrectionFactor>,
    },
    Radio {
        sampling_rate: i64,
        threshold: f64,
        radio_config: RadioConfig,
        mode: String,
        antenna_gain: f64,
    },
}

#[derive(Debug, Deserialize)]
struct OrchestratorConfig {
    sensors: Vec<SensorConfig>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config_str = std::fs::read_to_string("config.yaml").map_err(FabricError::from)?;
    let config: OrchestratorConfig =
        serde_yaml::from_str(&config_str).map_err(FabricError::from)?;

    // Create a HashMap for quick config lookup
    let config_map: HashMap<String, SensorConfig> = config
        .sensors
        .into_iter()
        .map(|sensor| (sensor.id.clone(), sensor))
        .collect();

    // Initialize Zenoh session
    let session = Arc::new(zenoh::open(zenoh::config::Config::default()).res().await?);

    // Create Orchestrator
    let orchestrator = Orchestrator::new("main_orchestrator".to_string(), session.clone()).await?;

    // Subscribe to config requests
    let subscriber = session.declare_subscriber(CONFIG_TOPIC).res().await?;

    // Handle config requests
    tokio::spawn(async move {
        while let Ok(sample) = subscriber.recv_async().await {
            if let Ok(node_id) = String::from_utf8(sample.value.payload.contiguous().to_vec()) {
                let config = config_map
                    .get(&node_id)
                    .cloned()
                    .unwrap_or_else(|| SensorConfig {
                        id: node_id.clone(),
                        sensor_type: "default".to_string(),
                        config: serde_json::json!({}),
                    });

                let config_json = serde_json::to_string(&config).unwrap();
                let _ = session.put(CONFIG_TOPIC, config_json).res().await;
                println!("Sent config for node: {}", node_id);
            }
        }
    });

    // Run the orchestrator
    let cancel_token = CancellationToken::new();
    orchestrator.run(cancel_token.clone()).await?;

    Ok(())
}
