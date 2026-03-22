use anyhow::{Context, Result};

use crate::measurement::{MeasurementApi, NewMeasurement};

pub struct MeasurementClient {
    client: reqwest::Client,
}

impl MeasurementClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

impl MeasurementApi for MeasurementClient {
    async fn store_measurements(&self, url: &str, measurements: &[NewMeasurement]) -> Result<()> {
        self.client
            .post(url)
            .json(measurements)
            .send()
            .await
            .with_context(|| format!("Failed to send measurements to {url}"))?
            .error_for_status()
            .with_context(|| format!("Storage service rejected measurements at {url}"))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn make_client() -> MeasurementClient {
        MeasurementClient::new(reqwest::Client::new())
    }

    fn make_measurements() -> Vec<NewMeasurement> {
        let timestamp = Utc.timestamp_opt(1_710_086_400, 0).single().unwrap();

        vec![
            NewMeasurement::new_with_ts(timestamp, 7, 11, 102.5),
            NewMeasurement::new_with_ts(timestamp, 7, 12, 2_500.0),
        ]
    }

    #[tokio::test]
    async fn stores_measurements_successfully() {
        let mut server = mockito::Server::new_async().await;
        let mock = server.mock("POST", "/").with_status(200).create_async().await;

        let result = make_client()
            .store_measurements(&server.url(), &make_measurements())
            .await;

        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn returns_error_when_storage_rejects_measurements() {
        let mut server = mockito::Server::new_async().await;
        let mock = server.mock("POST", "/").with_status(500).create_async().await;

        let result = make_client()
            .store_measurements(&server.url(), &make_measurements())
            .await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Storage service rejected measurements"),
            "Expected storage rejection error, got: {err_msg}"
        );
        mock.assert_async().await;
    }
}
