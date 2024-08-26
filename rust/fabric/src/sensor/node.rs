use super::interface::{SensorConfig, SensorData, SensorInterface};
use crate::error::{FabricError, Result};
use crate::plugins::SensorRegistry;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

pub struct SensorNode {
    id: String,
    sensor: Arc<Mutex<Box<dyn SensorInterface>>>,
    session: Arc<Session>,
}

impl SensorNode {
    pub async fn new(
        id: String,
        sensor_type: String,
        config: SensorConfig,
        session: Arc<Session>,
    ) -> Result<Self> {
        let registry = SensorRegistry::new();
        let sensor = registry
            .create_sensor(&sensor_type, config)
            .ok_or_else(|| FabricError::Other(format!("Unknown sensor type: {}", sensor_type)))?;

        Ok(Self {
            id,
            sensor: Arc::new(Mutex::new(sensor)),
            session,
        })
    }

    pub async fn run(&self, cancel: CancellationToken) -> Result<()> {
        let publisher = self
            .session
            .declare_publisher("sensor/data")
            .res()
            .await
            .map_err(FabricError::ZenohError)?;

        let config_subscriber = self
            .session
            .declare_subscriber(&format!("sensor/{}/config", self.id))
            .res()
            .await
            .map_err(FabricError::ZenohError)?;

        let event_subscriber = self
            .session
            .declare_subscriber(&format!("sensor/{}/event/*", self.id))
            .res()
            .await
            .map_err(FabricError::ZenohError)?;

        let mut last_publish = Instant::now();
        let mut sampling_interval = Duration::from_secs(5); // Default interval

        while !cancel.is_cancelled() {
            tokio::select! {
                _ = tokio::time::sleep_until(last_publish + sampling_interval) => {
                    let sensor_value = {
                        let sensor = self.sensor.lock().await;
                        sensor.read().await?
                    };

                    let sensor_data = SensorData {
                        sensor_id: self.id.clone(),
                        sensor_type: {
                            let sensor = self.sensor.lock().await;
                            sensor.get_type()
                        },
                        value: sensor_value,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        metadata: None,
                    };

                    let data_json = serde_json::to_string(&sensor_data)?;
                    publisher.put(data_json).res().await.map_err(FabricError::ZenohError)?;
                    println!("Published sensor data: {:?}", sensor_data);

                    last_publish = Instant::now();
                }

                Ok(sample) = config_subscriber.recv_async() => {
                    if let Ok(config_json) = std::str::from_utf8(&sample.value.payload.contiguous()) {
                        if let Ok(new_config) = serde_json::from_str::<SensorConfig>(config_json) {
                            println!("Received new configuration: {:?}", new_config);
                            let mut sensor = self.sensor.lock().await;
                            sensor.set_config(new_config.clone());
                            sampling_interval = Duration::from_secs(new_config.sampling_rate);
                        }
                    }
                }

                Ok(sample) = event_subscriber.recv_async() => {
                    if let Ok(payload) = std::str::from_utf8(&sample.value.payload.contiguous()) {
                        let key_expr = sample.key_expr.as_str();
                        if let Some(event) = key_expr.split('/').last() {
                            println!("Received event for sensor {}: {} - {}", self.id, event, payload);
                            let mut sensor = self.sensor.lock().await;
                            if let Err(e) = sensor.handle_event(event, payload).await {
                                eprintln!("Error handling event: {}", e);
                            }
                        }
                    }
                }

                _ = cancel.cancelled() => {
                    break;
                }
            }
        }
        Ok(())
    }

    pub async fn get_config(&self) -> SensorConfig {
        let sensor = self.sensor.lock().await;
        sensor.get_config()
    }
}
