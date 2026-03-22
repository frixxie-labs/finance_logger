use anyhow::Result;

pub mod measurement_client;
pub mod types;

pub use measurement_client::MeasurementClient;
pub use types::NewMeasurement;

pub trait MeasurementApi {
    async fn store_measurements(&self, url: &str, measurements: &[NewMeasurement]) -> Result<()>;
}
