use async_trait::async_trait;
use fabric::node::interface::{NodeConfig, NodeInterface};
use fabric::node::Node;
use fabric::Result;
use log::{debug, error, info, warn};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;
use zenoh::config;
use zenoh::prelude::r#async::*;
use zenoh::Session;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QuadcopterConfig {
    max_altitude: f32,
    max_speed: f32,
    home_position: [f32; 3],
    battery_threshold: f32,
}

#[derive(Debug, Serialize, Deserialize)]
enum QuadcopterCommand {
    MoveTo([f64; 3]),
    Land,
    TakeOff,
}

struct QuadcopterNode {
    altitude: f32,
    battery_level: f32,
    command_mode: String,
    config: NodeConfig,
    quadcopter_config: Arc<Mutex<QuadcopterConfig>>,
    rng: StdRng,
}

#[async_trait]
impl NodeInterface for QuadcopterNode {
    fn get_config(&self) -> NodeConfig {
        self.config.clone()
    }

    async fn set_config(&mut self, config: NodeConfig) {
        self.config = config.clone();
        if let Ok(quad_config) = serde_json::from_value::<QuadcopterConfig>(config.config) {
            let mut current_config = self.quadcopter_config.lock().await;
            *current_config = quad_config;
            info!("Updated quadcopter config: {:?}", *current_config);
        } else {
            warn!("Failed to parse quadcopter config");
        }
    }

    fn get_type(&self) -> String {
        "quadcopter".to_string()
    }

    async fn handle_event(&mut self, event: &str, payload: &str) -> Result<()> {
        match event {
            "takeoff" => {
                info!("Received takeoff command");
                self.altitude = 10.0;
                self.command_mode = "auto_take_off".to_string();
            }
            "land" => {
                info!("Received land command");
                self.altitude = 0.0;
                self.command_mode = "auto_land".to_string();
            }
            "move_to" => {
                if let Ok(position) = serde_json::from_str::<[f64; 3]>(payload) {
                    info!("Received move_to command: {:?}", position);
                    self.command_mode = "move_to".to_string();
                } else {
                    warn!("Invalid move_to payload");
                }
            }
            _ => warn!("Received unknown command: {}", event),
        }
        Ok(())
    }

    async fn update_config(&mut self, config: NodeConfig) {
        self.set_config(config).await;
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl QuadcopterNode {
    fn new(config: NodeConfig) -> Self {
        info!("Creating new QuadcopterNode with config: {:?}", config);
        let quadcopter_config = Arc::new(Mutex::new(QuadcopterConfig {
            max_altitude: 100.0,
            max_speed: 10.0,
            home_position: [0.0, 0.0, 0.0],
            battery_threshold: 20.0,
        }));

        Self {
            altitude: 0.0,
            battery_level: 100.0,
            command_mode: "manual".to_string(),
            config,
            quadcopter_config,
            rng: StdRng::from_entropy(),
        }
    }

    async fn update_telemetry(&mut self) {
        let config = self.quadcopter_config.lock().await;
        self.altitude += self.rng.gen_range(-0.5..0.5);
        self.altitude = self.altitude.min(config.max_altitude);
        self.battery_level -= self.rng.gen_range(0.1..0.3);
        self.battery_level = self.battery_level.max(0.0);
        debug!(
            "Updated telemetry: altitude={}, battery_level={}",
            self.altitude, self.battery_level
        );
    }

    async fn get_telemetry(&self) -> TelemetryData {
        let config = self.quadcopter_config.lock().await;
        TelemetryData {
            quadcopter_id: self.config.node_id.clone(),
            altitude: self.altitude,
            battery_level: self.battery_level,
            max_altitude: config.max_altitude,
            max_speed: config.max_speed,
            home_position: config.home_position,
            battery_threshold: config.battery_threshold,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TelemetryData {
    quadcopter_id: String,
    altitude: f32,
    battery_level: f32,
    max_altitude: f32,
    max_speed: f32,
    home_position: [f32; 3],
    battery_threshold: f32,
}

async fn create_zenoh_session() -> Arc<Session> {
    let mut config = config::peer();
    config.transport.shared_memory.set_enabled(true).unwrap();
    config.scouting.multicast.set_enabled(Some(true)).unwrap();
    let session = zenoh::open(config).res().await.unwrap().into_arc();
    let info = session.info();
    info!("Zenoh session created with ZID: {:?}", info.zid());
    session
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    // Read VEHICLE_ID from environment variable
    let vehicle_id = env::var("VEHICLE_ID").expect("VEHICLE_ID must be set");

    info!("Starting quadcopter node with ID: {}", vehicle_id);

    let config = NodeConfig {
        node_id: vehicle_id.clone(),
        config: serde_json::json!({
            "node_type": "quadcopter"
        }),
    };

    let quadcopter_node = QuadcopterNode::new(config.clone());

    let session = create_zenoh_session().await;
    info!("Created Zenoh session");
    let node = Node::new(
        config.node_id.clone(),
        "quadcopter".to_string(),
        config.clone(),
        session,
        Some(Box::new(quadcopter_node)),
    )
    .await?;
    info!("Created Node");

    // Set up subscriber to receive commands
    let topic = "quadcopter/commands";
    let command_callback = Arc::new(Mutex::new(move |sample: Sample| {
        if let Ok(command) =
            serde_json::from_slice::<QuadcopterCommand>(&sample.value.payload.contiguous())
        {
            info!("Received command: {:?}", command);
            // Handle the command here
        } else {
            warn!("Failed to parse command");
        }
    }));
    node.create_subscriber(topic.to_string(), command_callback)
        .await?;
    info!("Created subscriber for commands");

    // Set up subscriber to receive heartbeat messages from the orchestrator
    let heartbeat_topic = "orchestrator/**".to_string();
    info!(
        "Setting up subscriber for heartbeat topic: {}",
        heartbeat_topic
    );
    let heartbeat_callback = Arc::new(Mutex::new(move |sample: Sample| {
        info!("Received sample on heartbeat topic: {:?}", sample);
        if let Ok(heartbeat) = std::str::from_utf8(&sample.value.payload.contiguous()) {
            info!("Received orchestrator heartbeat: {}", heartbeat);
        } else {
            warn!("Failed to parse orchestrator heartbeat");
        }
    }));
    match node
        .create_subscriber(heartbeat_topic.clone(), heartbeat_callback)
        .await
    {
        Ok(_) => info!("Successfully created subscriber for orchestrator heartbeats"),
        Err(e) => error!(
            "Failed to create subscriber for orchestrator heartbeats: {:?}",
            e
        ),
    }

    // Set up publisher to send telemetry data
    let telemetry_topic = format!("node/{}/quadcopter/telemetry", vehicle_id);
    node.create_publisher(telemetry_topic.clone()).await?;
    info!("Created publisher for telemetry");

    // Create a cancellation token
    let cancel_token = CancellationToken::new();

    // Spawn a task to publish telemetry data
    let telemetry_task = {
        let node_clone = node.clone();
        let cancel_token_clone = cancel_token.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(500));
            info!("Starting telemetry publishing task");
            loop {
                tokio::select! {
                    _ = cancel_token_clone.cancelled() => {
                        info!("Telemetry task cancelled");
                        break;
                    }
                    _ = interval.tick() => {
                        if let Ok(interface) = node_clone.get_interface().await {
                            let mut interface = interface.lock().await;
                            if let Some(quadcopter_node) = interface.as_any().downcast_mut::<QuadcopterNode>() {
                                quadcopter_node.update_telemetry().await;
                                let telemetry = quadcopter_node.get_telemetry().await;
                                match serde_json::to_vec(&telemetry) {
                                    Ok(telemetry_json) => {
                                        info!("Attempting to publish telemetry: {:?}", telemetry);
                                        info!("Publishing to topic: {}", telemetry_topic);
                                        if let Err(e) = node_clone.publish(&telemetry_topic, telemetry_json).await {
                                            error!("Failed to publish telemetry: {:?}", e);
                                        } else {
                                            info!("Successfully published telemetry");
                                        }
                                    },
                                    Err(e) => {
                                        error!("Failed to serialize telemetry: {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        })
    };

    // Set up publisher for random integers
    let random_int_topic = "node/data";
    node.create_publisher(random_int_topic.to_string()).await?;
    info!("Created publisher for random integers");

    // Spawn a task to publish random integers
    let random_int_task = {
        let node_clone = node.clone();
        let cancel_token_clone = cancel_token.clone();
        let config = config.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            let mut rng = rand::rngs::StdRng::from_entropy();
            loop {
                tokio::select! {
                    _ = cancel_token_clone.cancelled() => {
                        info!("Random int task cancelled");
                        break;
                    }
                    _ = interval.tick() => {
                        let random_int = rng.gen_range(0..100);
                        let payload = serde_json::json!({
                            "node_id": config.node_id,
                            "node_type": "quadcopter",
                            "value": random_int
                        }).to_string();
                        if let Err(e) = node_clone.publish(random_int_topic, payload.into_bytes()).await {
                            error!("Failed to publish random int: {:?}", e);
                        } else {
                            info!("Published random int: {}", random_int);
                        }
                    }
                }
            }
        })
    };

    // Run the node
    info!("Running node");
    node.run(cancel_token.clone()).await?;

    // Cancel the telemetry task on Ctrl+C
    info!("Waiting for Ctrl+C");
    tokio::signal::ctrl_c().await.unwrap();
    info!("Received Ctrl+C, shutting down");
    cancel_token.cancel();
    telemetry_task.await.unwrap();

    // Cancel the random int task on Ctrl+C
    random_int_task.await.unwrap();

    info!("Node shut down successfully");
    Ok(())
}

#[allow(dead_code)]
async fn publish_telemetry(node: &Node, topic: &str, telemetry: &TelemetryData) -> Result<()> {
    let payload = serde_json::to_string(&telemetry)?;
    node.publish(topic, payload.into_bytes()).await?;
    info!("Successfully published telemetry");
    Ok(())
}
