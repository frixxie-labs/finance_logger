use anyhow::Result;
use structopt::StructOpt;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

mod device;
mod finance;
mod measurement;
mod sensor;

use device::{DeviceApi, DeviceClient};
use finance::{FinanceApi, FinanceClient};
use measurement::{MeasurementApi, MeasurementClient, NewMeasurement};
use sensor::{Sensor, SensorApi, SensorClient};

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

fn init_tracing(log_level: LogLevel) {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::from(log_level))
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();
}

fn api_url(base: &str, resource: &str) -> String {
    format!("{}/api/{resource}", base.trim_end_matches('/'))
}

async fn setup_finance_sensors(
    sensor_client: &SensorClient,
    sensor_url: &str,
    ticker_sensors: &[Sensor],
) -> Result<Vec<Sensor>> {
    let mut resolved_sensors = Vec::with_capacity(ticker_sensors.len());

    for sensor in ticker_sensors {
        let sensor_id = sensor_client
            .setup_sensor(sensor_url, &sensor.name, &sensor.unit)
            .await?;

        resolved_sensors.push(Sensor {
            id: sensor_id,
            name: sensor.name.clone(),
            unit: sensor.unit.clone(),
        });
    }

    Ok(resolved_sensors)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::from_args();
    init_tracing(cli.log_level.clone());

    tracing::info!(symbols = ?cli.symbols, hemrs_url = %cli.hemrs_url, "Starting finance logger");

    let finance_client = FinanceClient::new();
    let http_client = reqwest::Client::new();
    let device_client = DeviceClient::new(http_client.clone());
    let sensor_client = SensorClient::new(http_client);
    let measurement_client = MeasurementClient::new(reqwest::Client::new());
    let device_url = api_url(&cli.hemrs_url, "devices");
    let sensor_url = api_url(&cli.hemrs_url, "sensors");
    let measurement_url = api_url(&cli.hemrs_url, "measurements");
    let symbols = cli.symbols.iter().map(String::as_str).collect::<Vec<_>>();
    tracing::info!(count = symbols.len(), "Fetching tickers");
    let tickers = finance_client.get_tickers(&symbols).await?;
    tracing::info!(count = tickers.len(), "Fetched tickers successfully");

    let finance_device = finance::Ticker::finance_device();
    let device_id = device_client
        .setup_device(&device_url, &finance_device.name, &finance_device.location)
        .await?;
    tracing::info!(device_id, "Shared finance device ready");
    let mut all_measurements = Vec::<NewMeasurement>::new();

    for ticker in tickers {
        tracing::info!(symbol = %ticker.symbol, "Processing ticker");
        let ticker_sensors = Vec::<Sensor>::from(&ticker);
        let resolved_sensors =
            setup_finance_sensors(&sensor_client, &sensor_url, &ticker_sensors).await?;
        let resources = ticker.to_finance_resources(device_id, &resolved_sensors);

        tracing::info!(
            symbol = %ticker.symbol,
            sensors = resources.sensors.len(),
            measurements = resources.measurements.len(),
            "Prepared finance resources"
        );

        if resources.measurements.is_empty() {
            tracing::warn!(symbol = %ticker.symbol, "No measurements generated for ticker");
        } else {
            all_measurements.extend(resources.measurements.iter().cloned());
            tracing::info!(
                symbol = %ticker.symbol,
                measurements = resources.measurements.len(),
                "Prepared ticker measurements"
            );
        }

        tracing::info!("{ticker}");
        tracing::info!(
            "device={} sensors={} measurements={}",
            resources.device.id,
            resources.sensors.len(),
            resources.measurements.len()
        );
    }

    if all_measurements.is_empty() {
        tracing::warn!("No ticker measurements to store");
    } else {
        measurement_client
            .store_measurements(&measurement_url, &all_measurements)
            .await?;
        tracing::info!(
            measurements = all_measurements.len(),
            "Stored ticker measurements batch"
        );
    }

    tracing::info!("Finance logger completed");

    Ok(())
}
