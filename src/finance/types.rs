use crate::device::Device;
use crate::measurement::NewMeasurement;
use crate::sensor::Sensor;
use chrono::Utc;
use std::fmt;
use yfinance_rs::{
    core::conversions::{money_to_currency_str, money_to_f64},
    Candle,
};

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct FinanceResources {
    pub device: Device,
    pub sensors: Vec<Sensor>,
    pub measurements: Vec<NewMeasurement>,
}

impl Ticker {
    pub fn finance_device() -> Device {
        Device {
            id: 0,
            name: "finance".to_string(),
            location: "finance".to_string(),
        }
    }

    pub(crate) fn from_candles(symbol: &str, candles: &[Candle]) -> Self {
        let last_candle = candles.last();
        let previous_candle = if candles.len() > 1 {
            candles.get(candles.len() - 2)
        } else {
            None
        };
        let unit = last_candle
            .and_then(|candle| money_to_currency_str(&candle.close))
            .unwrap_or_else(|| "USD".to_string());

        Self {
            symbol: symbol.to_string(),
            unit,
            price: last_candle.map(|candle| money_to_f64(&candle.close)),
            volume: last_candle.and_then(|candle| candle.volume),
            change: last_candle
                .zip(previous_candle)
                .map(|(last, previous)| money_to_f64(&last.close) - money_to_f64(&previous.close)),
            open: last_candle.map(|candle| money_to_f64(&candle.open)),
            high: last_candle.map(|candle| money_to_f64(&candle.high)),
            low: last_candle.map(|candle| money_to_f64(&candle.low)),
        }
    }

    pub fn to_finance_resources(&self, device_id: i32, sensors: &[Sensor]) -> FinanceResources {
        FinanceResources {
            device: Device {
                id: device_id,
                ..Self::finance_device()
            },
            sensors: sensors.to_vec(),
            measurements: self.to_measurements(device_id, sensors),
        }
    }
}

impl fmt::Display for Ticker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let format_price = |value: Option<f64>| match value {
            Some(value) => format!("{value:.2} {}", self.unit),
            None => "n/a".to_string(),
        };

        let volume = self
            .volume
            .map(|value| format!("{value} shares"))
            .unwrap_or_else(|| "n/a".to_string());

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

impl Ticker {
    pub fn to_measurements(&self, device_id: i32, sensors: &[Sensor]) -> Vec<NewMeasurement> {
        let timestamp = Some(Utc::now());

        [
            ("price", self.price.map(|value| value as f32)),
            ("volume", self.volume.map(|value| value as f32)),
            ("change", self.change.map(|value| value as f32)),
            ("open", self.open.map(|value| value as f32)),
            ("high", self.high.map(|value| value as f32)),
            ("low", self.low.map(|value| value as f32)),
        ]
        .into_iter()
        .filter_map(|(name, measurement)| {
            let measurement = measurement?;
            let sensor_name = format!("{}_{}", self.symbol, name);
            let sensor = sensors.iter().find(|sensor| sensor.name == sensor_name)?;

            Some(NewMeasurement {
                timestamp,
                device: device_id,
                sensor: sensor.id,
                measurement,
            })
        })
        .collect()
    }
}

impl From<&Ticker> for Vec<Sensor> {
    fn from(ticker: &Ticker) -> Self {
        [
            ("price", ticker.unit.as_str()),
            ("volume", "shares"),
            ("change", ticker.unit.as_str()),
            ("open", ticker.unit.as_str()),
            ("high", ticker.unit.as_str()),
            ("low", ticker.unit.as_str()),
        ]
        .into_iter()
        .map(|(name, unit)| Sensor {
            id: 0,
            name: format!("{}_{}", ticker.symbol, name),
            unit: unit.to_string(),
        })
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::device::Device;
    use crate::measurement::NewMeasurement;
    use crate::sensor::Sensor;
    use chrono::{TimeZone, Utc};
    use yfinance_rs::core::conversions::f64_to_money_with_currency_str;
    use yfinance_rs::Candle;

    use super::Ticker;

    /// Helper to check measurements ignoring timestamp (which is Utc::now()).
    fn assert_measurements_match_ignoring_ts(
        actual: &[NewMeasurement],
        expected: &[(i32, i32, f32)],
    ) {
        assert_eq!(actual.len(), expected.len());
        for (m, (device, sensor, measurement)) in actual.iter().zip(expected.iter()) {
            assert!(m.timestamp.is_some(), "timestamp should be Some(now)");
            assert_eq!(m.device, *device);
            assert_eq!(m.sensor, *sensor);
            assert_eq!(m.measurement, *measurement);
        }
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
            open: f64_to_money_with_currency_str(open, Some(unit)),
            high: f64_to_money_with_currency_str(high, Some(unit)),
            low: f64_to_money_with_currency_str(low, Some(unit)),
            close: f64_to_money_with_currency_str(close, Some(unit)),
            close_unadj: None,
            volume,
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

        assert_eq!(ticker.symbol, "AAPL");
        assert_eq!(ticker.unit, "USD");
        assert_eq!(ticker.price, Some(102.5));
        assert_eq!(ticker.volume, Some(2_500));
        assert_eq!(ticker.change, Some(1.5));
        assert_eq!(ticker.open, Some(101.5));
        assert_eq!(ticker.high, Some(103.0));
        assert_eq!(ticker.low, Some(100.5));
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

        assert_eq!(ticker.symbol, "MSFT");
        assert_eq!(ticker.unit, "USD");
        assert_eq!(ticker.price, Some(204.0));
        assert_eq!(ticker.volume, None);
        assert_eq!(ticker.change, None);
    }

    #[test]
    fn builds_empty_ticker_when_no_candles_exist() {
        let ticker = Ticker::from_candles("NVDA", &[]);

        assert_eq!(ticker.symbol, "NVDA");
        assert_eq!(ticker.unit, "USD");
        assert_eq!(ticker.price, None);
        assert_eq!(ticker.volume, None);
        assert_eq!(ticker.change, None);
        assert_eq!(ticker.open, None);
        assert_eq!(ticker.high, None);
        assert_eq!(ticker.low, None);
    }

    #[test]
    fn converts_ticker_to_sensors() {
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

        let sensors = Vec::<Sensor>::from(&ticker);

        assert_eq!(
            sensors,
            vec![
                Sensor {
                    id: 0,
                    name: "EQNR.OL_price".to_string(),
                    unit: "NOK".to_string(),
                },
                Sensor {
                    id: 0,
                    name: "EQNR.OL_volume".to_string(),
                    unit: "shares".to_string(),
                },
                Sensor {
                    id: 0,
                    name: "EQNR.OL_change".to_string(),
                    unit: "NOK".to_string(),
                },
                Sensor {
                    id: 0,
                    name: "EQNR.OL_open".to_string(),
                    unit: "NOK".to_string(),
                },
                Sensor {
                    id: 0,
                    name: "EQNR.OL_high".to_string(),
                    unit: "NOK".to_string(),
                },
                Sensor {
                    id: 0,
                    name: "EQNR.OL_low".to_string(),
                    unit: "NOK".to_string(),
                },
            ]
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
    fn converts_manual_ticker_to_sensors_with_expected_units() {
        let ticker = Ticker {
            symbol: "TEL.OL".to_string(),
            unit: "NOK".to_string(),
            price: Some(150.0),
            volume: Some(42),
            change: Some(-1.2),
            open: Some(151.0),
            high: Some(152.0),
            low: Some(149.5),
        };

        let sensors = Vec::<Sensor>::from(&ticker);

        assert_eq!(sensors.len(), 6);
        assert!(sensors.iter().all(|sensor| sensor.id == 0));
        assert_eq!(sensors[0].unit, "NOK");
        assert_eq!(sensors[1].unit, "shares");
        assert_eq!(sensors[2].unit, "NOK");
        assert_eq!(sensors[3].unit, "NOK");
        assert_eq!(sensors[4].unit, "NOK");
        assert_eq!(sensors[5].unit, "NOK");
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
        let ticker = Ticker {
            symbol: "NVDA".to_string(),
            unit: "USD".to_string(),
            price: None,
            volume: None,
            change: None,
            open: None,
            high: None,
            low: None,
        };

        assert_eq!(
            ticker.to_string(),
            "NVDA: price n/a, volume n/a, change n/a, open n/a, high n/a, low n/a"
        );
    }

    #[test]
    fn builds_shared_finance_device() {
        assert_eq!(
            Ticker::finance_device(),
            Device {
                id: 0,
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
            Sensor {
                id: 11,
                name: "EQNR.OL_price".to_string(),
                unit: "NOK".to_string(),
            },
            Sensor {
                id: 12,
                name: "EQNR.OL_volume".to_string(),
                unit: "shares".to_string(),
            },
            Sensor {
                id: 13,
                name: "EQNR.OL_change".to_string(),
                unit: "NOK".to_string(),
            },
            Sensor {
                id: 14,
                name: "EQNR.OL_open".to_string(),
                unit: "NOK".to_string(),
            },
            Sensor {
                id: 15,
                name: "EQNR.OL_high".to_string(),
                unit: "NOK".to_string(),
            },
            Sensor {
                id: 16,
                name: "EQNR.OL_low".to_string(),
                unit: "NOK".to_string(),
            },
        ];

        assert_measurements_match_ignoring_ts(
            &ticker.to_measurements(7, &sensors),
            &[
                (7, 11, 102.5),
                (7, 12, 2_500.0),
                (7, 13, 1.5),
                (7, 14, 101.5),
                (7, 15, 103.0),
                (7, 16, 100.5),
            ],
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
        let sensors = vec![Sensor {
            id: 21,
            name: "NVDA_price".to_string(),
            unit: "USD".to_string(),
        }];

        let measurements = ticker.to_measurements(3, &sensors);
        assert_eq!(measurements.len(), 1);
        assert!(measurements[0].timestamp.is_some());
        assert_eq!(measurements[0].device, 3);
        assert_eq!(measurements[0].sensor, 21);
        assert_eq!(measurements[0].measurement, 875.25);
    }

    #[test]
    fn builds_finance_resources_in_one_call() {
        let ticker = Ticker {
            symbol: "AAPL".to_string(),
            unit: "USD".to_string(),
            price: Some(102.5),
            volume: Some(2_500),
            change: None,
            open: Some(101.5),
            high: None,
            low: Some(100.5),
        };
        let sensors = vec![
            Sensor {
                id: 31,
                name: "AAPL_price".to_string(),
                unit: "USD".to_string(),
            },
            Sensor {
                id: 32,
                name: "AAPL_volume".to_string(),
                unit: "shares".to_string(),
            },
            Sensor {
                id: 33,
                name: "AAPL_open".to_string(),
                unit: "USD".to_string(),
            },
            Sensor {
                id: 34,
                name: "AAPL_low".to_string(),
                unit: "USD".to_string(),
            },
        ];

        let resources = ticker.to_finance_resources(9, &sensors);

        assert_eq!(
            resources.device,
            Device {
                id: 9,
                name: "finance".to_string(),
                location: "finance".to_string(),
            }
        );
        assert_eq!(resources.sensors, sensors);
        assert_measurements_match_ignoring_ts(
            &resources.measurements,
            &[
                (9, 31, 102.5),
                (9, 32, 2_500.0),
                (9, 33, 101.5),
                (9, 34, 100.5),
            ],
        );
    }
}
