use super::{CallbackFunction, ControlConfig, SensorState};
use crate::sensor::interface::{SensorConfig, SensorData};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

use crate::error::Result;

pub struct ControlNode {
    id: String,
    session: Arc<Session>,
    pub sensors: Arc<Mutex<HashMap<String, SensorState>>>,
    callbacks: Arc<Mutex<HashMap<String, CallbackFunction>>>,
}

impl ControlNode {
    pub async fn new(id: String, session: Arc<Session>) -> Result<Self> {
        Ok(Self {
            id,
            session,
            sensors: Arc::new(Mutex::new(HashMap::new())),
            callbacks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn run(&self, cancel: CancellationToken) -> Result<()> {
        let subscriber = self.session.declare_subscriber("sensor/data").res().await?;

        while !cancel.is_cancelled() {
            tokio::select! {
                Ok(sample) = subscriber.recv_async() => {
                    if let Ok(payload) = std::str::from_utf8(&sample.value.payload.contiguous()) {
                        if let Ok(data) = serde_json::from_str::<SensorData>(payload) {
                            println!("Control node {} received data from sensor {}: {:.2}", self.id, data.sensor_id, data.value);
                            self.update_sensor_state(data.clone()).await;
                            self.trigger_callbacks(data).await;
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

    pub async fn update_sensor_state(&self, data: SensorData) {
        let mut sensors = self.sensors.lock().await;
        sensors.insert(
            data.sensor_id.clone(),
            SensorState {
                last_value: data.value,
                last_update: std::time::SystemTime::now(),
            },
        );
    }

    async fn trigger_callbacks(&self, data: SensorData) {
        let callbacks = self.callbacks.lock().await;
        if let Some(callback) = callbacks.get(&data.sensor_id) {
            callback(data);
        }
    }

    pub async fn subscribe_to_sensor(
        &self,
        sensor_id: &str,
        callback: impl Fn(SensorData) + Send + Sync + 'static,
    ) -> Result<()> {
        let mut callbacks = self.callbacks.lock().await;
        callbacks.insert(sensor_id.to_string(), Box::new(callback));

        let subscriber = self.session.declare_subscriber("sensor/data").res().await?;

        tokio::spawn({
            let sensor_id = sensor_id.to_string();
            let callbacks = self.callbacks.clone();
            async move {
                while let Ok(sample) = subscriber.recv_async().await {
                    if let Ok(payload) = std::str::from_utf8(&sample.value.payload.contiguous()) {
                        if let Ok(data) = serde_json::from_str::<SensorData>(payload) {
                            if data.sensor_id == sensor_id {
                                println!("Received data for sensor {}: {:?}", sensor_id, data);
                                let callbacks = callbacks.lock().await;
                                if let Some(callback) = callbacks.get(&sensor_id) {
                                    callback(data);
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn publish_sensor_config(
        &self,
        sensor_id: &str,
        config: &SensorConfig,
    ) -> Result<()> {
        let key = format!("sensor/{}/config", sensor_id);
        let config_json = serde_json::to_string(config)?;

        self.session.put(&key, config_json).res().await?;

        println!("Published configuration for sensor {}", sensor_id);
        Ok(())
    }

    pub async fn publish_sensor_configs(&self, config: &ControlConfig) -> Result<()> {
        for sensor_config in &config.sensors {
            self.publish_sensor_config(&sensor_config.sensor_id, sensor_config)
                .await?;
        }
        Ok(())
    }
}
