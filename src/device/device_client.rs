use anyhow::{Context, Result, anyhow};

use super::types::Device;
use super::{DeviceApi, DeviceId};

pub struct DeviceClient {
    client: reqwest::Client,
}

impl DeviceClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

impl DeviceApi for DeviceClient {
    async fn get_devices(&self, url: &str) -> Result<Vec<Device>> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch devices")?;

        let devices = response
            .json::<Vec<Device>>()
            .await
            .context("Failed to parse devices response")?;

        Ok(devices)
    }

    async fn setup_device(
        &self,
        url: &str,
        device_name: &str,
        device_location: &str,
    ) -> Result<DeviceId> {
        let devices = self.get_devices(url).await?;
        let device = devices
            .iter()
            .find(|d| d.name == device_name && d.location == device_location);

        match device {
            Some(d) => {
                tracing::info!("Found existing device: {:?}", d);
                Ok(d.id)
            }
            None => {
                let new_device = Device {
                    id: 0,
                    name: device_name.to_string(),
                    location: device_location.to_string(),
                };

                let response = self
                    .client
                    .post(url)
                    .json(&new_device)
                    .send()
                    .await
                    .context("Failed to create device")?;

                tracing::info!("Created new device: {:?}", response);

                let devices = self.get_devices(url).await?;
                devices
                    .iter()
                    .find(|d| d.name == device_name && d.location == device_location)
                    .map(|d| d.id)
                    .ok_or_else(|| anyhow!("Device not found after creation"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client() -> DeviceClient {
        DeviceClient::new(reqwest::Client::new())
    }

    #[tokio::test]
    async fn should_fetch_devices_successfully() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                {"id": 1, "name": "test_device", "location": "test_location"},
                {"id": 2, "name": "another_device", "location": "another_location"}
            ]"#,
            )
            .create_async()
            .await;

        let device_client = make_client();
        let result = device_client.get_devices(&server.url()).await;

        assert!(result.is_ok());
        let devices = result.unwrap();
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].id, 1);
        assert_eq!(devices[0].name, "test_device");
        assert_eq!(devices[0].location, "test_location");
        assert_eq!(devices[1].id, 2);
        assert_eq!(devices[1].name, "another_device");
        assert_eq!(devices[1].location, "another_location");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn should_setup_existing_device() {
        let mut server = mockito::Server::new_async().await;
        let mock_get = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                {"id": 1, "name": "existing_device", "location": "test_location"}
            ]"#,
            )
            .create_async()
            .await;

        let device_client = make_client();
        let result = device_client
            .setup_device(&server.url(), "existing_device", "test_location")
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
        mock_get.assert_async().await;
    }

    #[tokio::test]
    async fn should_setup_new_device() {
        let mut server = mockito::Server::new_async().await;

        let mock_get_empty = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("[]")
            .create_async()
            .await;

        let mock_post = server
            .mock("POST", "/")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id": 2, "name": "new_device", "location": "new_location"}"#)
            .create_async()
            .await;

        let mock_get_created = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                {"id": 2, "name": "new_device", "location": "new_location"}
            ]"#,
            )
            .create_async()
            .await;

        let device_client = make_client();
        let result = device_client
            .setup_device(&server.url(), "new_device", "new_location")
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);

        mock_get_empty.assert_async().await;
        mock_post.assert_async().await;
        mock_get_created.assert_async().await;
    }

    #[tokio::test]
    async fn should_return_error_when_device_creation_fails() {
        let mut server = mockito::Server::new_async().await;

        let mock_get_empty = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("[]")
            .create_async()
            .await;

        let mock_post = server
            .mock("POST", "/")
            .with_status(201)
            .create_async()
            .await;

        let mock_get_still_empty = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("[]")
            .create_async()
            .await;

        let device_client = make_client();
        let result = device_client
            .setup_device(&server.url(), "ghost_device", "nowhere")
            .await;

        assert!(result.is_err());

        mock_get_empty.assert_async().await;
        mock_post.assert_async().await;
        mock_get_still_empty.assert_async().await;
    }

    #[tokio::test]
    async fn should_return_error_when_get_devices_returns_invalid_json() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("not valid json")
            .create_async()
            .await;

        let device_client = make_client();
        let result = device_client.get_devices(&server.url()).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Failed to parse devices response"),
            "Expected parse error, got: {err_msg}"
        );

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn should_propagate_get_devices_error_in_setup_device() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("not valid json")
            .create_async()
            .await;

        let device_client = make_client();
        let result = device_client
            .setup_device(&server.url(), "any_device", "any_location")
            .await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Failed to parse devices response"),
            "Expected parse error propagated from get_devices, got: {err_msg}"
        );

        mock.assert_async().await;
    }
}
