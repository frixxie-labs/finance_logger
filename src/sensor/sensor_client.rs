use anyhow::{Context, Result, anyhow};

use super::SensorApi;
use super::types::Sensor;

pub struct SensorClient {
    client: reqwest::Client,
}

impl SensorClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

impl SensorApi for SensorClient {
    async fn get_sensors(&self, url: &str) -> Result<Vec<Sensor>> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch sensors")?;

        let sensors = response
            .json::<Vec<Sensor>>()
            .await
            .context("Failed to parse sensors response")?;

        Ok(sensors)
    }

    async fn setup_sensor(&self, url: &str, sensor_name: &str, sensor_unit: &str) -> Result<i32> {
        let sensors = self.get_sensors(url).await?;
        let sensor = sensors.iter().find(|s| s.name == sensor_name);

        match sensor {
            Some(s) => {
                tracing::info!("Found existing sensor: {:?}", s);
                Ok(s.id)
            }
            None => {
                let new_sensor = Sensor {
                    id: 0,
                    name: sensor_name.to_string(),
                    unit: sensor_unit.to_string(),
                };

                let response = self
                    .client
                    .post(url)
                    .json(&new_sensor)
                    .send()
                    .await
                    .context("Failed to create sensor")?;

                tracing::info!("Created new sensor: {:?}", response);

                let sensors = self.get_sensors(url).await?;
                sensors
                    .iter()
                    .find(|s| s.name == sensor_name)
                    .map(|s| s.id)
                    .ok_or_else(|| anyhow!("Sensor not found after creation"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client() -> SensorClient {
        SensorClient::new(reqwest::Client::new())
    }

    #[tokio::test]
    async fn should_fetch_sensors_successfully() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                {"id": 1, "name": "temperature", "unit": "°C"},
                {"id": 2, "name": "humidity", "unit": "%"}
            ]"#,
            )
            .create_async()
            .await;

        let sensor_client = make_client();
        let result = sensor_client.get_sensors(&server.url()).await;

        assert!(result.is_ok());
        let sensors = result.unwrap();
        assert_eq!(sensors.len(), 2);
        assert_eq!(sensors[0].id, 1);
        assert_eq!(sensors[0].name, "temperature");
        assert_eq!(sensors[0].unit, "°C");
        assert_eq!(sensors[1].id, 2);
        assert_eq!(sensors[1].name, "humidity");
        assert_eq!(sensors[1].unit, "%");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn should_setup_existing_sensor() {
        let mut server = mockito::Server::new_async().await;
        let mock_get = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                {"id": 1, "name": "temperature", "unit": "°C"}
            ]"#,
            )
            .create_async()
            .await;

        let sensor_client = make_client();
        let result = sensor_client
            .setup_sensor(&server.url(), "temperature", "°C")
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
        mock_get.assert_async().await;
    }

    #[tokio::test]
    async fn should_setup_new_sensor() {
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
            .with_body(r#"{"id": 2, "name": "temperature", "unit": "°C"}"#)
            .create_async()
            .await;

        let mock_get_created = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                {"id": 2, "name": "temperature", "unit": "°C"}
            ]"#,
            )
            .create_async()
            .await;

        let sensor_client = make_client();
        let result = sensor_client
            .setup_sensor(&server.url(), "temperature", "°C")
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);

        mock_get_empty.assert_async().await;
        mock_post.assert_async().await;
        mock_get_created.assert_async().await;
    }

    #[tokio::test]
    async fn should_return_error_when_sensor_creation_fails() {
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

        let sensor_client = make_client();
        let result = sensor_client
            .setup_sensor(&server.url(), "ghost_sensor", "unit")
            .await;

        assert!(result.is_err());

        mock_get_empty.assert_async().await;
        mock_post.assert_async().await;
        mock_get_still_empty.assert_async().await;
    }

    #[tokio::test]
    async fn should_return_error_when_get_sensors_returns_invalid_json() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("not valid json")
            .create_async()
            .await;

        let sensor_client = make_client();
        let result = sensor_client.get_sensors(&server.url()).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Failed to parse sensors response"),
            "Expected parse error, got: {err_msg}"
        );

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn should_propagate_get_sensors_error_in_setup_sensor() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("not valid json")
            .create_async()
            .await;

        let sensor_client = make_client();
        let result = sensor_client
            .setup_sensor(&server.url(), "any_sensor", "any_unit")
            .await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Failed to parse sensors response"),
            "Expected parse error propagated from get_sensors, got: {err_msg}"
        );

        mock.assert_async().await;
    }

}
