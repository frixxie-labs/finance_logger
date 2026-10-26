use anyhow::Result;

pub mod sensor_client;
pub mod types;

pub use sensor_client::SensorClient;
pub use types::{NewSensor, Sensor};

pub trait SensorApi {
    async fn get_sensors(&mut self, url: &str) -> Result<Vec<Sensor>>;
    async fn setup_sensor(&mut self, url: &str, sensor: &NewSensor) -> Result<Sensor>;
}
