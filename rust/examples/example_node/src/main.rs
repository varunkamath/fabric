use async_trait::async_trait;
use fabric::node::interface::{NodeConfig, NodeData, NodeInterface};
use fabric::node::Node;
use fabric::Result;
use log::{error, info, warn};
use rand::rngs::SmallRng;
use rand::Rng;
use rand::SeedableRng;
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

#[derive(Clone)]
struct QuadcopterNode {
    node_id: String,
    altitude: f32,
    battery_level: f32,
    command_mode: String,
    config: NodeConfig,
    quadcopter_config: Arc<Mutex<QuadcopterConfig>>,
    rng: Arc<Mutex<SmallRng>>,
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
        }
    }

    fn get_type(&self) -> String {
        "quadcopter".to_string()
    }

    async fn handle_event(&mut self, event: &str, payload: &str) -> Result<()> {
        match event {
            "move_to" => {
                self.command_mode = "moving".to_string();
                info!("Moving to position: {}", payload);
            }
            "land" => {
                self.command_mode = "landing".to_string();
                info!("Landing quadcopter");
            }
            "take_off" => {
                self.command_mode = "taking_off".to_string();
                info!("Taking off");
            }
            _ => {
                warn!("Unknown event: {}", event);
            }
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
    async fn run(&mut self, node: &Node, cancel_token: CancellationToken) -> Result<()> {
        let telemetry_topic = format!("node/{}/quadcopter/telemetry", self.node_id);
        node.create_publisher(telemetry_topic.clone()).await?;

        let mut interval = interval(Duration::from_secs(1));

        while !cancel_token.is_cancelled() {
            tokio::select! {
                _ = interval.tick() => {
                    let mut rng = self.rng.lock().await;
                    self.altitude += rng.gen_range(-0.1..0.1);
                    self.battery_level -= rng.gen_range(0.1..0.5);

                    let config = self.quadcopter_config.lock().await;
                    if self.battery_level < config.battery_threshold {
                        warn!("Low battery! Returning to home position.");
                        self.command_mode = "returning_home".to_string();
                    }

                    let telemetry_data = serde_json::json!({
                        "altitude": self.altitude,
                        "battery_level": self.battery_level,
                        "command_mode": self.command_mode,
                    });

                    let node_data = NodeData {
                        node_id: self.node_id.clone(),
                        node_type: self.get_type(),
                        timestamp: chrono::Utc::now().timestamp() as u64,
                        metadata: Some(telemetry_data),
                        status: "online".to_string(),
                    };

                    if let Err(e) = node.publish(&telemetry_topic, serde_json::to_string(&node_data)?.into_bytes()).await {
                        error!("Failed to publish telemetry: {:?}", e);
                    }
                }
                _ = cancel_token.cancelled() => {
                    break;
                }
            }
        }

        Ok(())
    }
}

async fn create_zenoh_session() -> Result<Session> {
    let config = config::Config::default();
    let session = zenoh::open(config)
        .res()
        .await
        .map_err(fabric::error::FabricError::ZenohError)?;
    let info = session.info();
    info!("Zenoh session created with ZID: {:?}", info.zid());
    Ok(session)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let vehicle_id = env::var("VEHICLE_ID").unwrap_or_else(|_| "quadcopter_1".to_string());

    info!("Starting quadcopter node with ID: {}", vehicle_id);

    let initial_config = serde_json::json!({
        "quadcopter_config": {
            "max_altitude": 100.0,
            "max_speed": 10.0,
            "home_position": [0.0, 0.0, 0.0],
            "battery_threshold": 20.0,
        }
    });

    let config = NodeConfig {
        node_id: vehicle_id.clone(),
        config: initial_config,
    };

    let mut quadcopter_node = QuadcopterNode {
        node_id: vehicle_id.clone(),
        altitude: 0.0,
        battery_level: 100.0,
        command_mode: "idle".to_string(),
        config: config.clone(),
        quadcopter_config: Arc::new(Mutex::new(QuadcopterConfig {
            max_altitude: 100.0,
            max_speed: 10.0,
            home_position: [0.0, 0.0, 0.0],
            battery_threshold: 20.0,
        })),
        rng: Arc::new(Mutex::new(SmallRng::from_entropy())),
    };

    let session = create_zenoh_session().await?;
    let node = Node::new(
        config.node_id.clone(),
        "quadcopter".to_string(),
        config.clone(),
        Arc::new(session),
        Some(Box::new(quadcopter_node.clone())),
    )
    .await?;

    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();

    tokio::select! {
        result = quadcopter_node.run(&node, cancel_token.clone()) => {
            if let Err(e) = result {
                error!("Error running quadcopter node: {:?}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down");
            cancel_token_clone.cancel();
        }
    }

    info!("Node shut down successfully");
    Ok(())
}
