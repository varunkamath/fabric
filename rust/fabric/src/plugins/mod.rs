use crate::sensor::interface::{SensorConfig, SensorFactory, SensorInterface};
use std::collections::HashMap;
use std::sync::Arc;

mod radio;

pub struct SensorRegistry {
    factories: HashMap<String, Arc<dyn SensorFactory>>,
}

impl Default for SensorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SensorRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
        };
        registry.register_default_sensors();
        registry
    }

    fn register_default_sensors(&mut self) {
        self.register_sensor("radio", Arc::new(radio::RadioSensorFactory));
        // Register more default sensors here
    }

    pub fn register_sensor(&mut self, sensor_type: &str, factory: Arc<dyn SensorFactory>) {
        self.factories.insert(sensor_type.to_string(), factory);
    }

    pub fn create_sensor(
        &self,
        sensor_type: &str,
        config: SensorConfig,
    ) -> Option<Box<dyn SensorInterface>> {
        self.factories
            .get(sensor_type)
            .map(|factory| factory.create(config))
    }
}
