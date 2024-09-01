use chrono::Utc;
use dashmap::DashMap;
use fabric::node::interface::NodeConfig;
use fabric::node::interface::NodeData;
use fabric::orchestrator::Orchestrator;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;
use zenoh::Session;

#[derive(Debug, Serialize, Deserialize)]
struct QuadcopterState {
    position: [f64; 3],
    velocity: [f64; 3],
    battery_level: f32,
}

#[derive(Debug, Serialize, Deserialize)]
enum QuadcopterCommand {
    MoveTo([f64; 3]),
    Land,
    TakeOff,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TelemetryData {
    quadcopter_id: String,
    altitude: f32,
    battery_level: f32,
    max_altitude: f32,
    max_speed: f32,
    home_position: [f32; 3],
    battery_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QuadcopterConfig {
    max_altitude: f32,
    max_speed: f32,
    home_position: [f32; 3],
    battery_threshold: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct QuadcopterConfigs {
    quadcopters: Vec<QuadcopterConfig>,
}

async fn create_zenoh_session() -> Arc<Session> {
    let mut config = zenoh::config::peer();
    config.scouting.multicast.set_enabled(Some(true)).unwrap();
    config.transport.shared_memory.set_enabled(true).unwrap();

    // Optimize for low latency
    let _ = config.transport.unicast.qos.set_enabled(false); // Disable QoS for unicast
    let _ = config.transport.multicast.qos.set_enabled(false); // Disable QoS for multicast
    let _ = config.transport.unicast.compression.set_enabled(false); // Disable compression for unicast
    let _ = config.transport.multicast.compression.set_enabled(false); // Disable compression for multicast

    // Note: Zenoh no longer exposes direct control over batch size, lease time, and keep-alive interval
    // in the public API. These are now handled internally by Zenoh for optimal performance.

    let session = zenoh::open(config).res().await.unwrap().into_arc();
    let info = session.info();
    info!("Zenoh session created with ZID: {:?}", info.zid());
    session
}

async fn publish_heartbeat(session: Arc<Session>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let key_expression = "orchestrator/heartbeat";
    loop {
        let value = format!("Heartbeat at {}", Utc::now());
        info!("Attempting to publish heartbeat: {}", value);
        match session.put(key_expression, value.clone()).res().await {
            Ok(_) => info!("Successfully published heartbeat: {}", value),
            Err(e) => error!("Failed to publish heartbeat: {}", e),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logger
    env_logger::init();

    info!("Starting quadcopter orchestrator");
    let session = create_zenoh_session().await;

    // Load quadcopter configurations
    info!("Loading quadcopter configurations");
    let configs = load_quadcopter_configs("quadcopter_configs.yaml")?;
    let config_map: Arc<DashMap<String, QuadcopterConfig>> = Arc::new(DashMap::new());
    for (i, config) in configs.quadcopters.into_iter().enumerate() {
        config_map.insert(format!("quadcopter{}", i + 1), config);
    }
    info!("Loaded {} quadcopter configurations", config_map.len());

    // Create a shared state to store battery levels and configurations
    let quadcopter_state: Arc<DashMap<String, (TelemetryData, QuadcopterConfig)>> =
        Arc::new(DashMap::new());

    // Create orchestrator
    info!("Creating orchestrator");
    let orchestrator =
        Arc::new(Orchestrator::new("quad_orchestrator".to_string(), session.clone()).await?);

    // Publish heartbeat
    let session_clone = session.clone();
    tokio::spawn(async move {
        if let Err(e) = publish_heartbeat(session_clone).await {
            error!("Heartbeat publisher error: {:?}", e);
        }
    });

    // Create a set to keep track of nodes that have received their config
    let configured_nodes: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    // Subscribe to node health
    info!("Subscribing to node health");
    let config_map_clone = config_map.clone();
    let orchestrator_clone = orchestrator.clone();
    let configured_nodes_clone = configured_nodes.clone();
    match orchestrator
        .create_subscriber(
            "fabric/*/status".to_string(),
            Arc::new(Mutex::new(move |sample: Sample| {
                let config_map = config_map_clone.clone();
                let orchestrator = orchestrator_clone.clone();
                let configured_nodes = configured_nodes_clone.clone();
                tokio::spawn(async move {
                    if let Ok(node_data) =
                        serde_json::from_slice::<NodeData>(&sample.value.payload.contiguous())
                    {
                        debug!("Received node health update: {:?}", node_data);
                        let mut should_send_config = false;
                        {
                            let mut configured = configured_nodes.lock().await;
                            if node_data.status == "online"
                                && !configured.contains(&node_data.node_id)
                            {
                                configured.insert(node_data.node_id.clone());
                                should_send_config = true;
                            }
                        }
                        if should_send_config {
                            if let Some(config) = config_map.get(&node_data.node_id) {
                                if let Err(e) = send_node_config(
                                    &orchestrator,
                                    &node_data.node_id,
                                    config.value(),
                                )
                                .await
                                {
                                    error!(
                                        "Failed to send config to node {}: {:?}",
                                        node_data.node_id, e
                                    );
                                } else {
                                    info!("Sent initial config to node {}", node_data.node_id);
                                }
                            } else {
                                warn!("No configuration found for node {}", node_data.node_id);
                            }
                        }
                    } else {
                        error!("Failed to parse node health data");
                    }
                });
            })),
        )
        .await
    {
        Ok(_) => info!("Successfully subscribed to node health"),
        Err(e) => error!("Failed to subscribe to node health: {:?}", e),
    }

    // Periodic config resend task
    let config_map_clone = config_map.clone();
    let orchestrator_clone = orchestrator.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60)); // Resend every 60 seconds
        loop {
            interval.tick().await;
            for entry in config_map_clone.iter() {
                let node_id = entry.key();
                let config = entry.value();
                if let Err(e) = send_node_config(&orchestrator_clone, node_id, config).await {
                    error!("Failed to resend config to node {}: {:?}", node_id, e);
                }
            }
        }
    });

    // Create a channel for telemetry processing
    let (telemetry_tx, telemetry_rx) = mpsc::channel(1000);

    // Spawn a task to handle telemetry processing
    let quadcopter_state_clone = quadcopter_state.clone();
    let config_map_clone = config_map.clone();
    let telemetry_task = tokio::task::spawn(async move {
        process_telemetry_stream(telemetry_rx, quadcopter_state_clone, config_map_clone).await;
    });

    // Subscribe to telemetry
    info!("Subscribing to telemetry");
    match orchestrator
        .create_subscriber(
            "node/*/quadcopter/telemetry".to_string(),
            Arc::new(Mutex::new(move |sample: Sample| {
                let telemetry_tx = telemetry_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = telemetry_tx.send(sample).await {
                        error!("Failed to send telemetry to processing task: {:?}", e);
                    }
                });
            })),
        )
        .await
    {
        Ok(_) => info!("Successfully subscribed to telemetry"),
        Err(e) => error!("Failed to subscribe to telemetry: {:?}", e),
    }

    // Run the orchestrator
    info!("Starting orchestrator");
    let orchestrator_cancel_token = CancellationToken::new();
    let orchestrator_clone = orchestrator.clone();
    let orchestrator_handle =
        tokio::spawn(async move { orchestrator_clone.run(orchestrator_cancel_token).await });

    // Main loop cancel token
    let main_cancel_token = CancellationToken::new();

    // Main loop
    info!("Entering main loop");
    loop {
        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(1000)) => {
                // Example: Check battery levels and send commands
                let states = quadcopter_state.clone();
                for entry in states.iter() {
                    let (quadcopter_id, (telemetry, _)) = entry.pair();
                    info!("Current state of {}: battery_level = {}", quadcopter_id, telemetry.battery_level);
                    if telemetry.battery_level < 20.0 {
                        let command = QuadcopterCommand::Land;
                        warn!("Battery level for {} is low. Commanding to land.", quadcopter_id);
                        if let Err(e) = send_command(quadcopter_id, &command).await {
                            error!("Failed to send land command to {}: {:?}", quadcopter_id, e);
                        }
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down...");
                main_cancel_token.cancel();
                break;
            }
        }
    }

    // Cancel the orchestrator and wait for it to finish
    info!("Shutting down orchestrator");
    orchestrator_handle.abort();
    if let Err(e) = orchestrator_handle.await {
        error!("Error shutting down orchestrator: {:?}", e);
    }

    // Cancel the telemetry task before shutting down
    telemetry_task.abort();
    if let Err(e) = telemetry_task.await {
        error!("Error shutting down telemetry task: {:?}", e);
    }

    info!("Orchestrator shut down successfully");
    Ok(())
}

fn load_quadcopter_configs(path: &str) -> Result<QuadcopterConfigs, Box<dyn Error + Send + Sync>> {
    let file = File::open(path)?;
    let configs: QuadcopterConfigs = serde_yaml::from_reader(file)?;
    Ok(configs)
}

impl Default for QuadcopterConfig {
    fn default() -> Self {
        Self {
            max_altitude: 100.0,
            max_speed: 10.0,
            home_position: [0.0, 0.0, 0.0],
            battery_threshold: 20.0,
        }
    }
}

async fn process_telemetry_stream(
    mut rx: mpsc::Receiver<Sample>,
    quadcopter_state: Arc<DashMap<String, (TelemetryData, QuadcopterConfig)>>,
    config_map: Arc<DashMap<String, QuadcopterConfig>>,
) {
    while let Some(sample) = rx.recv().await {
        if let Err(e) =
            process_telemetry(sample, quadcopter_state.clone(), config_map.clone()).await
        {
            error!("Error processing telemetry: {:?}", e);
        }
    }
}

async fn process_telemetry(
    sample: Sample,
    quadcopter_state: Arc<DashMap<String, (TelemetryData, QuadcopterConfig)>>,
    config_map: Arc<DashMap<String, QuadcopterConfig>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let telemetry: TelemetryData = serde_json::from_slice(&sample.value.payload.contiguous())?;
    let quadcopter_id = telemetry.quadcopter_id.clone();

    quadcopter_state
        .entry(quadcopter_id.clone())
        .and_modify(|(old_telemetry, _config)| {
            *old_telemetry = telemetry.clone();
        })
        .or_insert_with(|| {
            let config = config_map
                .get(&quadcopter_id)
                .map(|entry| entry.value().clone())
                .unwrap_or_else(|| {
                    warn!("No configuration found for quadcopter: {}", quadcopter_id);
                    QuadcopterConfig::default()
                });
            (telemetry.clone(), config)
        });

    info!("Processed telemetry for {}: {:?}", quadcopter_id, telemetry);
    Ok(())
}

async fn send_command(
    node_name: &str,
    command: &QuadcopterCommand,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Implement the actual command sending logic here
    info!("Sending command {:?} to {}", command, node_name);
    // For now, just simulate a delay
    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok(())
}

async fn send_node_config(
    orchestrator: &Orchestrator,
    node_id: &str,
    config: &QuadcopterConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let node_config = NodeConfig {
        node_id: node_id.to_string(),
        config: serde_json::to_value(config)?,
    };
    orchestrator
        .publish_node_config(node_id, &node_config)
        .await?;
    info!("Sent configuration to node {}: {:?}", node_id, node_config);
    Ok(())
}
