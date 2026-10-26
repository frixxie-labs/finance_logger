use anyhow::Result;

pub mod device_client;
pub mod types;

pub use device_client::DeviceClient;
pub use types::{Device, NewDevice};

pub type DeviceId = i32;

pub trait DeviceApi {
    async fn get_devices(&self, url: &str) -> Result<Vec<Device>>;
    async fn setup_device(&self, url: &str, device: &NewDevice) -> Result<DeviceId>;
}
