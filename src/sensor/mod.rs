use anyhow::Result;

pub mod types;
pub mod sensor_client;

pub use types::{Sensor};
pub use sensor_client::SensorClient;

pub trait SensorApi {
    async fn get_sensors(&mut self, url: &str) -> Result<Vec<Sensor>>;
    async fn setup_sensor(
        &mut self,
        url: &str,
        sensor_name: &str,
        sensor_unit: &str,
    ) -> Result<i32>;
}
