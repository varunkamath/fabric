mod node;
pub use node::ControlNode;

use crate::sensor::interface::{SensorConfig, SensorData};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensorState {
    pub last_value: f64,
    pub last_update: std::time::SystemTime,
}

pub type CallbackFunction = Box<dyn Fn(SensorData) + Send + Sync>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ControlConfig {
    pub sensors: Vec<SensorConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use zenoh::prelude::r#async::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_control_node_new() {
        let config = zenoh::config::Config::default();
        let session = zenoh::open(config).res().await.unwrap();

        let result = ControlNode::new("test_control".to_string(), Arc::new(session)).await;

        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_update_sensor_state() {
        let config = zenoh::config::Config::default();
        let session = zenoh::open(config).res().await.unwrap();

        let control_node = ControlNode::new("test_control".to_string(), Arc::new(session))
            .await
            .unwrap();

        let sensor_data = SensorData {
            sensor_id: "test_sensor".to_string(),
            sensor_type: "radio".to_string(),
            value: 42.0,
            timestamp: 1234567890,
            metadata: None,
        };

        control_node.update_sensor_state(sensor_data.clone()).await;

        let sensors = control_node.sensors.lock().await;
        assert!(sensors.contains_key(&sensor_data.sensor_id));
        let state = sensors.get(&sensor_data.sensor_id).unwrap();
        assert_eq!(state.last_value, sensor_data.value);
    }
}
