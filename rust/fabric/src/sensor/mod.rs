pub mod interface;
pub mod node;

pub use interface::{SensorConfig, SensorData, SensorInterface};
pub use node::SensorNode;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use zenoh::prelude::r#async::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_sensor_node_new() {
        let config = zenoh::config::Config::default();
        let session = zenoh::open(config).res().await.unwrap();
        let sensor_config = SensorConfig {
            sensor_id: "test_sensor".to_string(),
            sampling_rate: 5,
            threshold: 50.0,
            custom_config: serde_json::json!({"radio_config": {"frequency": 100e6, "sample_rate": 2e6, "gain": 20.0, "mode": "receive"}}),
        };

        let result = SensorNode::new(
            "test_sensor".to_string(),
            "radio".to_string(),
            sensor_config,
            Arc::new(session),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_sensor_node_unknown_type() {
        let config = zenoh::config::Config::default();
        let session = zenoh::open(config).res().await.unwrap();
        let sensor_config = SensorConfig {
            sensor_id: "test_sensor".to_string(),
            sampling_rate: 5,
            threshold: 50.0,
            custom_config: serde_json::Value::Null,
        };

        let result = SensorNode::new(
            "test_sensor".to_string(),
            "unknown".to_string(),
            sensor_config,
            Arc::new(session),
        )
        .await;

        assert!(result.is_err());
    }
}
