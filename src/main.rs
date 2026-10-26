use anyhow::Result;
use chrono::Utc;
use structopt::StructOpt;
use tracing::{Level, info, warn};
use tracing_subscriber::FmtSubscriber;

mod device;
mod finance;
mod measurement;
mod sensor;

use device::{DeviceApi, DeviceClient};
use finance::{FinanceApi, FinanceClient};
use measurement::{MeasurementApi, MeasurementClient};
use sensor::{NewSensor, Sensor, SensorApi, SensorClient};

#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(name = "SYMBOL", default_value = "AAPL")]
    symbols: Vec<String>,

    #[structopt(long, default_value = "http://hemrs/")]
    hemrs_url: String,

    #[structopt(long, default_value = "info")]
    log_level: LogLevel,
}

#[derive(Debug, Clone)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            _ => Err("unknown log level".to_string()),
        }
    }
}

impl From<LogLevel> for Level {
    fn from(log_level: LogLevel) -> Self {
        match log_level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

fn init_tracing(log_level: LogLevel) -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::from(log_level))
        .json()
        .flatten_event(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

fn api_url(base: &str, resource: &str) -> String {
    format!("{}/api/{resource}", base.trim_end_matches('/'))
}

async fn register_sensors(
    sensor_client: &mut SensorClient,
    sensor_url: &str,
    sensors: &[NewSensor],
) -> Result<Vec<Sensor>> {
    let mut registered = Vec::with_capacity(sensors.len());
    for sensor in sensors {
        registered.push(sensor_client.setup_sensor(sensor_url, sensor).await?);
    }
    Ok(registered)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::from_args();
    init_tracing(cli.log_level.clone())?;

    info!(symbols = ?cli.symbols, hemrs_url = %cli.hemrs_url, "Starting finance logger");

    let finance_client = FinanceClient::new();
    let http_client = reqwest::Client::new();
    let device_client = DeviceClient::new(http_client.clone());
    let mut sensor_client = SensorClient::new(http_client.clone());
    let measurement_client = MeasurementClient::new(http_client);
    let device_url = api_url(&cli.hemrs_url, "devices");
    let sensor_url = api_url(&cli.hemrs_url, "sensors");
    let measurement_url = api_url(&cli.hemrs_url, "measurements");

    let symbols = cli.symbols.iter().map(String::as_str).collect::<Vec<_>>();
    info!(count = symbols.len(), "Fetching tickers");
    let tickers = finance_client.get_tickers(&symbols).await?;
    info!(count = tickers.len(), "Fetched tickers successfully");

    let device_id = device_client
        .setup_device(&device_url, &finance::finance_device())
        .await?;
    info!(device_id, "Shared finance device ready");

    let timestamp = Utc::now();
    let mut all_measurements = Vec::new();

    for ticker in &tickers {
        info!("{ticker}");
        let sensors = register_sensors(&mut sensor_client, &sensor_url, &ticker.sensors()).await?;
        let measurements = ticker.to_measurements(timestamp, device_id, &sensors);

        if measurements.is_empty() {
            warn!(symbol = %ticker.symbol, "No measurements generated for ticker");
        } else {
            info!(
                symbol = %ticker.symbol,
                sensors = sensors.len(),
                measurements = measurements.len(),
                "Prepared ticker measurements"
            );
        }
        all_measurements.extend(measurements);
    }

    if all_measurements.is_empty() {
        warn!("No ticker measurements to store");
    } else {
        measurement_client
            .store_measurements(&measurement_url, &all_measurements)
            .await?;
        info!(
            measurements = all_measurements.len(),
            "Stored ticker measurements batch"
        );
    }

    info!("Finance logger completed");

    Ok(())
}
