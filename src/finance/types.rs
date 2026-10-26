use chrono::{DateTime, Utc};
use std::fmt;
use yfinance_rs::{Candle, PriceAmount, QuantityAmount};

use crate::device::{DeviceId, NewDevice};
use crate::measurement::NewMeasurement;
use crate::sensor::{NewSensor, Sensor};

const DEFAULT_UNIT: &str = "USD";
const VOLUME_UNIT: &str = "shares";

/// The values tracked for every ticker. Each metric maps to one hemrs sensor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    Price,
    Volume,
    Change,
    Open,
    High,
    Low,
}

impl Metric {
    pub const ALL: [Metric; 6] = [
        Metric::Price,
        Metric::Volume,
        Metric::Change,
        Metric::Open,
        Metric::High,
        Metric::Low,
    ];

    fn name(self) -> &'static str {
        match self {
            Metric::Price => "price",
            Metric::Volume => "volume",
            Metric::Change => "change",
            Metric::Open => "open",
            Metric::High => "high",
            Metric::Low => "low",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ticker {
    pub symbol: String,
    pub unit: String,
    pub price: Option<f64>,
    pub volume: Option<u64>,
    pub change: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
}

/// The single hemrs device that all ticker sensors belong to.
pub fn finance_device() -> NewDevice {
    NewDevice {
        name: "finance".to_string(),
        location: "finance".to_string(),
    }
}

fn price_to_f64(value: &PriceAmount) -> Option<f64> {
    value.as_decimal().to_string().parse().ok()
}

fn quantity_to_u64(value: &QuantityAmount) -> Option<u64> {
    value.as_decimal().to_string().parse().ok()
}

impl Ticker {
    pub(crate) fn from_candles(symbol: &str, candles: &[Candle]) -> Self {
        let (previous, last) = match candles {
            [.., previous, last] => (Some(previous), Some(last)),
            [last] => (None, Some(last)),
            [] => (None, None),
        };
        let close = |candle: &Candle| price_to_f64(&candle.ohlc.close);

        Self {
            symbol: symbol.to_string(),
            unit: last.map_or_else(|| DEFAULT_UNIT.to_string(), |c| c.currency.to_string()),
            price: last.and_then(close),
            volume: last
                .and_then(|c| c.volume.as_ref())
                .and_then(quantity_to_u64),
            change: last
                .and_then(close)
                .zip(previous.and_then(close))
                .map(|(last, previous)| last - previous),
            open: last.and_then(|c| price_to_f64(&c.ohlc.open)),
            high: last.and_then(|c| price_to_f64(&c.ohlc.high)),
            low: last.and_then(|c| price_to_f64(&c.ohlc.low)),
        }
    }

    fn value(&self, metric: Metric) -> Option<f64> {
        match metric {
            Metric::Price => self.price,
            Metric::Volume => self.volume.map(|value| value as f64),
            Metric::Change => self.change,
            Metric::Open => self.open,
            Metric::High => self.high,
            Metric::Low => self.low,
        }
    }

    fn unit_of(&self, metric: Metric) -> &str {
        match metric {
            Metric::Volume => VOLUME_UNIT,
            _ => &self.unit,
        }
    }

    fn sensor_name(&self, metric: Metric) -> String {
        format!("{}_{}", self.symbol, metric.name())
    }

    /// The sensors required to record every metric of this ticker.
    pub fn sensors(&self) -> Vec<NewSensor> {
        Metric::ALL
            .into_iter()
            .map(|metric| NewSensor {
                name: self.sensor_name(metric),
                unit: self.unit_of(metric).to_string(),
            })
            .collect()
    }

    /// Measurements for every metric that has a value and a matching registered sensor.
    pub fn to_measurements(
        &self,
        timestamp: DateTime<Utc>,
        device: DeviceId,
        sensors: &[Sensor],
    ) -> Vec<NewMeasurement> {
        Metric::ALL
            .into_iter()
            .filter_map(|metric| {
                let value = self.value(metric)?;
                let name = self.sensor_name(metric);
                let sensor = sensors.iter().find(|sensor| sensor.name == name)?;
                Some(NewMeasurement::new_with_ts(
                    timestamp,
                    device,
                    sensor.id,
                    value as f32,
                ))
            })
            .collect()
    }
}

impl fmt::Display for Ticker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let format_price = |value: Option<f64>| match value {
            Some(value) => format!("{value:.2} {}", self.unit),
            None => "n/a".to_string(),
        };

        let volume = self.volume.map_or_else(
            || "n/a".to_string(),
            |value| format!("{value} {VOLUME_UNIT}"),
        );

        write!(
            f,
            "{}: price {}, volume {}, change {}, open {}, high {}, low {}",
            self.symbol,
            format_price(self.price),
            volume,
            format_price(self.change),
            format_price(self.open),
            format_price(self.high),
            format_price(self.low),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use quickcheck::{Arbitrary, Gen, quickcheck};
    use std::str::FromStr;
    use yfinance_rs::{Currency, Decimal, Ohlc};

    fn price_amount(value: f64) -> PriceAmount {
        PriceAmount::new(Decimal::from_str(&value.to_string()).unwrap())
    }

    fn quantity_amount(value: u64) -> QuantityAmount {
        QuantityAmount::from_decimal(Decimal::from(value)).unwrap()
    }

    fn candle(
        ts: i64,
        unit: &str,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: Option<u64>,
    ) -> Candle {
        Candle {
            ts: Utc.timestamp_opt(ts, 0).single().unwrap(),
            currency: Currency::from_str(unit).unwrap(),
            ohlc: Ohlc::new(
                price_amount(open),
                price_amount(high),
                price_amount(low),
                price_amount(close),
            ),
            close_unadj: None,
            volume: volume.map(quantity_amount),
            provider: (),
        }
    }

    fn timestamp() -> DateTime<Utc> {
        Utc.timestamp_opt(1_710_086_400, 0).single().unwrap()
    }

    fn sensor(id: i32, name: &str, unit: &str) -> Sensor {
        Sensor {
            id,
            name: name.to_string(),
            unit: unit.to_string(),
        }
    }

    impl Arbitrary for Ticker {
        fn arbitrary(g: &mut Gen) -> Self {
            let finite = |g: &mut Gen| Option::<f64>::arbitrary(g).filter(|v| v.is_finite());
            Ticker {
                symbol: String::arbitrary(g),
                unit: String::arbitrary(g),
                price: finite(g),
                volume: Option::<u64>::arbitrary(g),
                change: finite(g),
                open: finite(g),
                high: finite(g),
                low: finite(g),
            }
        }
    }

    quickcheck! {
        fn prop_one_sensor_per_metric(ticker: Ticker) -> bool {
            ticker.sensors().len() == Metric::ALL.len()
        }

        fn prop_sensor_names_are_prefixed_with_symbol(ticker: Ticker) -> bool {
            ticker
                .sensors()
                .iter()
                .all(|sensor| sensor.name.starts_with(&format!("{}_", ticker.symbol)))
        }

        fn prop_one_measurement_per_present_value_when_all_sensors_exist(ticker: Ticker) -> bool {
            let sensors: Vec<Sensor> = ticker
                .sensors()
                .into_iter()
                .zip(1..)
                .map(|(new, id)| Sensor { id, name: new.name, unit: new.unit })
                .collect();
            let present = Metric::ALL.iter().filter(|m| ticker.value(**m).is_some()).count();

            ticker.to_measurements(timestamp(), 1, &sensors).len() == present
        }

        fn prop_no_measurements_without_sensors(ticker: Ticker) -> bool {
            ticker.to_measurements(timestamp(), 1, &[]).is_empty()
        }
    }

    #[test]
    fn builds_ticker_from_latest_candle() {
        let ticker = Ticker::from_candles(
            "AAPL",
            &[
                candle(1_710_000_000, "USD", 100.0, 102.0, 99.0, 101.0, Some(1_000)),
                candle(
                    1_710_086_400,
                    "USD",
                    101.5,
                    103.0,
                    100.5,
                    102.5,
                    Some(2_500),
                ),
            ],
        );

        assert_eq!(
            ticker,
            Ticker {
                symbol: "AAPL".to_string(),
                unit: "USD".to_string(),
                price: Some(102.5),
                volume: Some(2_500),
                change: Some(1.5),
                open: Some(101.5),
                high: Some(103.0),
                low: Some(100.5),
            }
        );
    }

    #[test]
    fn builds_ticker_without_change_when_only_one_candle_exists() {
        let ticker = Ticker::from_candles(
            "MSFT",
            &[candle(
                1_710_000_000,
                "USD",
                200.0,
                205.0,
                198.0,
                204.0,
                None,
            )],
        );

        assert_eq!(ticker.price, Some(204.0));
        assert_eq!(ticker.volume, None);
        assert_eq!(ticker.change, None);
    }

    #[test]
    fn builds_empty_ticker_when_no_candles_exist() {
        let ticker = Ticker::from_candles("NVDA", &[]);

        assert_eq!(
            ticker,
            Ticker {
                symbol: "NVDA".to_string(),
                unit: "USD".to_string(),
                price: None,
                volume: None,
                change: None,
                open: None,
                high: None,
                low: None,
            }
        );
    }

    #[test]
    fn keeps_non_usd_unit_from_latest_candle() {
        let ticker = Ticker::from_candles(
            "ORK.OL",
            &[
                candle(1_710_000_000, "USD", 10.0, 11.0, 9.0, 10.5, Some(100)),
                candle(1_710_086_400, "NOK", 110.0, 111.0, 109.0, 110.5, Some(200)),
            ],
        );

        assert_eq!(ticker.unit, "NOK");
    }

    #[test]
    fn converts_ticker_to_sensors_with_expected_names_and_units() {
        let ticker = Ticker::from_candles(
            "EQNR.OL",
            &[candle(
                1_710_086_400,
                "NOK",
                101.5,
                103.0,
                100.5,
                102.5,
                Some(2_500),
            )],
        );

        let new_sensor = |name: &str, unit: &str| NewSensor {
            name: name.to_string(),
            unit: unit.to_string(),
        };
        assert_eq!(
            ticker.sensors(),
            vec![
                new_sensor("EQNR.OL_price", "NOK"),
                new_sensor("EQNR.OL_volume", "shares"),
                new_sensor("EQNR.OL_change", "NOK"),
                new_sensor("EQNR.OL_open", "NOK"),
                new_sensor("EQNR.OL_high", "NOK"),
                new_sensor("EQNR.OL_low", "NOK"),
            ]
        );
    }

    #[test]
    fn displays_ticker_with_all_fields() {
        let ticker = Ticker {
            symbol: "AAPL".to_string(),
            unit: "USD".to_string(),
            price: Some(102.5),
            volume: Some(2_500),
            change: Some(1.5),
            open: Some(101.5),
            high: Some(103.0),
            low: Some(100.5),
        };

        assert_eq!(
            ticker.to_string(),
            "AAPL: price 102.50 USD, volume 2500 shares, change 1.50 USD, open 101.50 USD, high 103.00 USD, low 100.50 USD"
        );
    }

    #[test]
    fn displays_ticker_with_missing_fields_as_na() {
        let ticker = Ticker::from_candles("NVDA", &[]);

        assert_eq!(
            ticker.to_string(),
            "NVDA: price n/a, volume n/a, change n/a, open n/a, high n/a, low n/a"
        );
    }

    #[test]
    fn builds_shared_finance_device() {
        assert_eq!(
            finance_device(),
            NewDevice {
                name: "finance".to_string(),
                location: "finance".to_string(),
            }
        );
    }

    #[test]
    fn converts_ticker_to_measurements() {
        let ticker = Ticker {
            symbol: "EQNR.OL".to_string(),
            unit: "NOK".to_string(),
            price: Some(102.5),
            volume: Some(2_500),
            change: Some(1.5),
            open: Some(101.5),
            high: Some(103.0),
            low: Some(100.5),
        };
        let sensors = vec![
            sensor(11, "EQNR.OL_price", "NOK"),
            sensor(12, "EQNR.OL_volume", "shares"),
            sensor(13, "EQNR.OL_change", "NOK"),
            sensor(14, "EQNR.OL_open", "NOK"),
            sensor(15, "EQNR.OL_high", "NOK"),
            sensor(16, "EQNR.OL_low", "NOK"),
        ];

        let ts = timestamp();
        assert_eq!(
            ticker.to_measurements(ts, 7, &sensors),
            vec![
                NewMeasurement::new_with_ts(ts, 7, 11, 102.5),
                NewMeasurement::new_with_ts(ts, 7, 12, 2_500.0),
                NewMeasurement::new_with_ts(ts, 7, 13, 1.5),
                NewMeasurement::new_with_ts(ts, 7, 14, 101.5),
                NewMeasurement::new_with_ts(ts, 7, 15, 103.0),
                NewMeasurement::new_with_ts(ts, 7, 16, 100.5),
            ]
        );
    }

    #[test]
    fn skips_missing_values_and_unknown_sensors_when_building_measurements() {
        let ticker = Ticker {
            symbol: "NVDA".to_string(),
            unit: "USD".to_string(),
            price: Some(875.25),
            volume: None,
            change: None,
            open: Some(870.0),
            high: None,
            low: None,
        };
        let sensors = vec![sensor(21, "NVDA_price", "USD")];

        let ts = timestamp();
        assert_eq!(
            ticker.to_measurements(ts, 3, &sensors),
            vec![NewMeasurement::new_with_ts(ts, 3, 21, 875.25)]
        );
    }
}
