use async_trait::async_trait;
use fabric::error::Result;
use fabric::node::interface::{NodeConfig, NodeInterface};
use serde_json::Value;

pub struct TemperatureSensor {
    config: NodeConfig,
}

pub struct HumiditySensor {
    config: NodeConfig,
}

pub struct RadioSensor {
    config: NodeConfig,
}

pub struct DefaultSensor {
    config: NodeConfig,
}

#[async_trait]
impl NodeInterface for TemperatureSensor {
    async fn process(&self, _input: Option<Value>) -> Result<Option<Value>> {
        println!("Processing temperature data");
        // Implement temperature-specific logic here
        Ok(Some(serde_json::json!({"temperature": 25.5})))
    }
}

#[async_trait]
impl NodeInterface for HumiditySensor {
    async fn process(&self, _input: Option<Value>) -> Result<Option<Value>> {
        println!("Processing humidity data");
        // Implement humidity-specific logic here
        Ok(Some(serde_json::json!({"humidity": 60.0})))
    }
}

#[async_trait]
impl NodeInterface for RadioSensor {
    async fn process(&self, _input: Option<Value>) -> Result<Option<Value>> {
        println!("Processing radio data");
        // Implement radio-specific logic here
        Ok(Some(serde_json::json!({"signal_strength": -70})))
    }
}

#[async_trait]
impl NodeInterface for DefaultSensor {
    async fn process(&self, _input: Option<Value>) -> Result<Option<Value>> {
        println!("Processing default sensor data");
        // Implement default sensor logic here
        Ok(Some(serde_json::json!({"status": "ok"})))
    }
}

pub fn create_sensor_interface(sensor_type: &str, config: NodeConfig) -> Box<dyn NodeInterface> {
    match sensor_type {
        "temperature" => Box::new(TemperatureSensor { config }),
        "humidity" => Box::new(HumiditySensor { config }),
        "radio" => Box::new(RadioSensor { config }),
        _ => Box::new(DefaultSensor { config }),
    }
}
