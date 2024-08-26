use crate::error::Result;
use crate::sensor::interface::{SensorConfig, SensorFactory, SensorInterface};
use async_trait::async_trait;

pub struct RadioSensor {
    config: SensorConfig,
}

#[async_trait]
impl SensorInterface for RadioSensor {
    async fn read(&self) -> Result<f64> {
        // Implement radio sensor reading logic here
        Ok(0.0)
    }

    fn get_config(&self) -> SensorConfig {
        self.config.clone()
    }

    fn set_config(&mut self, config: SensorConfig) {
        self.config = config;
    }

    fn get_type(&self) -> String {
        "radio".to_string()
    }

    async fn handle_event(&mut self, _event: &str, _payload: &str) -> Result<()> {
        // Implement event handling logic here
        Ok(())
    }
}

pub struct RadioSensorFactory;

impl SensorFactory for RadioSensorFactory {
    fn create(&self, config: SensorConfig) -> Box<dyn SensorInterface> {
        Box::new(RadioSensor { config })
    }
}
