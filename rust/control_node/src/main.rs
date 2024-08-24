use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use zenoh::config::EndPoint;
use zenoh::prelude::r#async::*;

#[derive(Debug)]
struct OrchestratorError(String);

impl fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Orchestrator error: {}", self.0)
    }
}

impl std::error::Error for OrchestratorError {}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SensorData {
    sensor_id: String,
    value: f64,
}

struct SensorState {
    last_value: f64,
    last_update: std::time::Instant,
}

// Add this type alias before the Orchestrator struct
type CallbackFunction = Box<dyn Fn(SensorData) + Send + Sync>;

struct Orchestrator {
    session: Arc<Session>,
    sensors: Arc<Mutex<HashMap<String, SensorState>>>,
    callbacks: Arc<Mutex<HashMap<String, CallbackFunction>>>,
}

impl Orchestrator {
    async fn new() -> Result<Self, OrchestratorError> {
        let mut config = config::peer();
        config.listen.endpoints.push(
            "tcp/0.0.0.0:7447"
                .parse::<EndPoint>()
                .map_err(|e| OrchestratorError(e.to_string()))?,
        );
        let session = Arc::new(
            zenoh::open(config)
                .res()
                .await
                .map_err(|e| OrchestratorError(e.to_string()))?,
        );
        Ok(Self {
            session,
            sensors: Arc::new(Mutex::new(HashMap::new())),
            callbacks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    async fn run(&self, cancel: CancellationToken) -> Result<(), OrchestratorError> {
        let subscriber = self
            .session
            .declare_subscriber("sensor/#")
            .res()
            .await
            .map_err(|e| OrchestratorError(e.to_string()))?;

        while !cancel.is_cancelled() {
            tokio::select! {
                Ok(sample) = subscriber.recv_async() => {
                    if let Ok(payload) = std::str::from_utf8(&sample.value.payload.contiguous()) {
                        if let Ok(data) = serde_json::from_str::<SensorData>(payload) {
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

    async fn update_sensor_state(&self, data: SensorData) {
        let mut sensors = self.sensors.lock().await;
        sensors
            .entry(data.sensor_id.clone())
            .or_insert(SensorState {
                last_value: data.value,
                last_update: std::time::Instant::now(),
            });

        println!("Updated sensor {}: {:.2}", data.sensor_id, data.value);
    }

    async fn trigger_callbacks(&self, data: SensorData) {
        let callbacks = self.callbacks.lock().await;
        if let Some(callback) = callbacks.get(&data.sensor_id) {
            callback(data);
        }
    }

    async fn subscribe_to_sensor(
        &self,
        sensor_id: &str,
        callback: impl Fn(SensorData) + Send + Sync + 'static,
    ) {
        let mut callbacks = self.callbacks.lock().await;
        callbacks.insert(sensor_id.to_string(), Box::new(callback));
    }

    async fn monitor_sensors(&self, cancel: CancellationToken) {
        while !cancel.is_cancelled() {
            let sensors = self.sensors.lock().await;
            println!("Current sensor states:");
            for (id, state) in sensors.iter() {
                println!(
                    "  Sensor {}: {:.2} (last update: {:?} ago)",
                    id,
                    state.last_value,
                    state.last_update.elapsed()
                );
            }
            drop(sensors);
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    }

    async fn load_config(path: &str) -> Result<Config, OrchestratorError> {
        let config_str = fs::read_to_string(path)
            .map_err(|e| OrchestratorError(format!("Failed to read config file: {}", e)))?;
        let config: Config = serde_yaml::from_str(&config_str)
            .map_err(|e| OrchestratorError(format!("Failed to parse config: {}", e)))?;
        Ok(config)
    }

    async fn publish_sensor_config(
        &self,
        sensor_id: &str,
        sensor_config: &SensorConfig,
    ) -> Result<(), OrchestratorError> {
        let key = format!("sensor/{}/config", sensor_id);
        let config_json = serde_json::to_string(sensor_config)
            .map_err(|e| OrchestratorError(format!("Failed to serialize config: {}", e)))?;

        self.session
            .put(&key, config_json)
            .res()
            .await
            .map_err(|e| OrchestratorError(format!("Failed to publish config: {}", e)))?;

        println!("Published configuration for sensor {}", sensor_id);
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    sensors: HashMap<String, SensorConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SensorConfig {
    sampling_rate: u64,
    threshold: f64,
}

#[tokio::main]
async fn main() -> Result<(), OrchestratorError> {
    println!("Starting control node...");
    let orchestrator = Arc::new(Orchestrator::new().await?);
    let cancel = CancellationToken::new();

    // Load configuration
    let config = Orchestrator::load_config("config.yaml").await?;

    // Publish configurations to sensors
    for (sensor_id, sensor_config) in &config.sensors {
        orchestrator
            .publish_sensor_config(sensor_id, sensor_config)
            .await?;
    }

    // Subscribe to all sensors
    orchestrator
        .subscribe_to_sensor("sensor/#", |data| {
            println!(
                "Received data from sensor {}: {:.2}",
                data.sensor_id, data.value
            );
            // Add your custom logic here
        })
        .await;

    let run_task = tokio::spawn({
        let orchestrator = orchestrator.clone();
        let cancel = cancel.clone();
        async move {
            if let Err(e) = orchestrator.run(cancel).await {
                eprintln!("Orchestrator run error: {}", e);
            }
        }
    });

    let monitor_task = tokio::spawn({
        let orchestrator = orchestrator.clone();
        let cancel = cancel.clone();
        async move { orchestrator.monitor_sensors(cancel).await }
    });

    // Run indefinitely
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| OrchestratorError(e.to_string()))?;
    println!("Ctrl-C received, shutting down...");
    cancel.cancel();

    let _ = tokio::join!(run_task, monitor_task);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Update the test attribute to use multi-thread runtime
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_orchestrator_new() {
        let result = Orchestrator::new().await;
        assert!(result.is_ok());
    }

    // Update the test attribute to use multi-thread runtime
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_update_sensor_state() {
        let orchestrator = Orchestrator::new().await.unwrap();
        let data = SensorData {
            sensor_id: "test-sensor".to_string(),
            value: 42.0,
        };
        orchestrator.update_sensor_state(data.clone()).await;
        let sensors = orchestrator.sensors.lock().await;
        assert!(sensors.contains_key(&data.sensor_id));
        let state = sensors.get(&data.sensor_id).unwrap();
        assert_eq!(state.last_value, data.value);
    }

    // Update the test attribute to use multi-thread runtime
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_publish_sensor_config() {
        let orchestrator = Orchestrator::new().await.unwrap();
        let sensor_id = "test-sensor";
        let config = SensorConfig {
            sampling_rate: 10,
            threshold: 75.0,
        };
        let result = orchestrator.publish_sensor_config(sensor_id, &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_load_config() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let config_content = r#"
        sensors:
          sensor1:
            sampling_rate: 5
            threshold: 50.0
          sensor2:
            sampling_rate: 10
            threshold: 75.0
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", config_content).unwrap();

        let config = Orchestrator::load_config(temp_file.path().to_str().unwrap()).await;
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.sensors.len(), 2);
        assert!(config.sensors.contains_key("sensor1"));
        assert!(config.sensors.contains_key("sensor2"));
    }

    #[tokio::test]
    async fn test_orchestrator_error() {
        let error = OrchestratorError("Test error".to_string());
        assert_eq!(error.to_string(), "Orchestrator error: Test error");
    }
}
