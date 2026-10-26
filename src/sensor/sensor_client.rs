use std::collections::BTreeMap;

use anyhow::{Context, Result, anyhow};

use super::{NewSensor, Sensor, SensorApi};

pub struct SensorClient {
    client: reqwest::Client,
    cache: BTreeMap<String, Sensor>,
}

impl SensorClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            cache: BTreeMap::new(),
        }
    }

    pub fn lookup_sensor(&self, name: &str) -> Option<&Sensor> {
        self.cache.get(name)
    }
}

impl SensorApi for SensorClient {
    async fn get_sensors(&mut self, url: &str) -> Result<Vec<Sensor>> {
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

        for sensor in &sensors {
            self.cache.insert(sensor.name.clone(), sensor.clone());
        }

        Ok(sensors)
    }

    async fn setup_sensor(&mut self, url: &str, sensor: &NewSensor) -> Result<Sensor> {
        if let Some(cached) = self.lookup_sensor(&sensor.name) {
            tracing::info!("Found cached sensor: {:?}", cached);
            return Ok(cached.clone());
        }

        let find = |sensors: Vec<Sensor>| sensors.into_iter().find(|s| sensor.matches(s));

        if let Some(existing) = find(self.get_sensors(url).await?) {
            tracing::info!("Found existing sensor: {:?}", existing);
            return Ok(existing);
        }

        let response = self
            .client
            .post(url)
            .json(sensor)
            .send()
            .await
            .context("Failed to create sensor")?
            .error_for_status()
            .context("hemrs rejected sensor creation")?;

        tracing::info!("Created new sensor: {:?}", response);

        find(self.get_sensors(url).await?).ok_or_else(|| anyhow!("Sensor not found after creation"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client() -> SensorClient {
        SensorClient::new(reqwest::Client::new())
    }

    fn new_sensor(name: &str, unit: &str) -> NewSensor {
        NewSensor {
            name: name.to_string(),
            unit: unit.to_string(),
        }
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

        let mut sensor_client = make_client();
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

        let mut sensor_client = make_client();
        let result = sensor_client
            .setup_sensor(&server.url(), &new_sensor("temperature", "°C"))
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, 1);
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

        let mut sensor_client = make_client();
        let result = sensor_client
            .setup_sensor(&server.url(), &new_sensor("temperature", "°C"))
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, 2);

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

        let mut sensor_client = make_client();
        let result = sensor_client
            .setup_sensor(&server.url(), &new_sensor("ghost_sensor", "unit"))
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

        let mut sensor_client = make_client();
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

        let mut sensor_client = make_client();
        let result = sensor_client
            .setup_sensor(&server.url(), &new_sensor("any_sensor", "any_unit"))
            .await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Failed to parse sensors response"),
            "Expected parse error propagated from get_sensors, got: {err_msg}"
        );

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn should_populate_cache_after_get_sensors() {
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

        let mut sensor_client = make_client();
        assert!(sensor_client.lookup_sensor("temperature").is_none());

        sensor_client.get_sensors(&server.url()).await.unwrap();

        let cached = sensor_client.lookup_sensor("temperature");
        assert!(cached.is_some());
        let sensor = cached.unwrap();
        assert_eq!(sensor.id, 1);
        assert_eq!(sensor.name, "temperature");
        assert_eq!(sensor.unit, "°C");

        let cached = sensor_client.lookup_sensor("humidity");
        assert!(cached.is_some());
        let sensor = cached.unwrap();
        assert_eq!(sensor.id, 2);
        assert_eq!(sensor.name, "humidity");
        assert_eq!(sensor.unit, "%");

        assert!(sensor_client.lookup_sensor("nonexistent").is_none());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn should_update_cache_on_subsequent_get_sensors() {
        let mut server = mockito::Server::new_async().await;

        let mock_first = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"id": 1, "name": "temperature", "unit": "°C"}]"#)
            .create_async()
            .await;

        let mock_second = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"id": 1, "name": "temperature", "unit": "°F"}]"#)
            .create_async()
            .await;

        let mut sensor_client = make_client();

        sensor_client.get_sensors(&server.url()).await.unwrap();
        assert_eq!(
            sensor_client.lookup_sensor("temperature").unwrap().unit,
            "°C"
        );

        sensor_client.get_sensors(&server.url()).await.unwrap();
        assert_eq!(
            sensor_client.lookup_sensor("temperature").unwrap().unit,
            "°F"
        );

        mock_first.assert_async().await;
        mock_second.assert_async().await;
    }

    #[tokio::test]
    async fn should_not_populate_cache_on_failed_get_sensors() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("not valid json")
            .create_async()
            .await;

        let mut sensor_client = make_client();
        let _ = sensor_client.get_sensors(&server.url()).await;

        assert!(sensor_client.lookup_sensor("temperature").is_none());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn should_return_error_when_sensor_creation_is_rejected() {
        let mut server = mockito::Server::new_async().await;
        let mock_get = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("[]")
            .create_async()
            .await;
        let mock_post = server
            .mock("POST", "/")
            .with_status(500)
            .create_async()
            .await;

        let result = make_client()
            .setup_sensor(&server.url(), &new_sensor("temperature", "°C"))
            .await;

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("hemrs rejected sensor creation"),
            "Expected rejection error, got: {err_msg}"
        );
        mock_get.assert_async().await;
        mock_post.assert_async().await;
    }
}
