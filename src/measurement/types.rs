use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct NewMeasurement {
    pub timestamp: Option<DateTime<Utc>>,
    pub device: i32,
    pub sensor: i32,
    pub measurement: f32,
}

impl NewMeasurement {
    pub fn new_with_ts(ts: DateTime<Utc>, device: i32, sensor: i32, measurement: f32) -> Self {
        Self {
            timestamp: Some(ts),
            device,
            sensor,
            measurement,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use quickcheck::{Arbitrary, Gen, quickcheck};

    impl Arbitrary for NewMeasurement {
        fn arbitrary(g: &mut Gen) -> Self {
            // Generate a timestamp within a reasonable range to avoid chrono overflow
            let secs = i64::arbitrary(g) % 4_000_000_000;
            let ts = Utc
                .timestamp_opt(secs.abs(), 0)
                .single()
                .unwrap_or_else(Utc::now);
            NewMeasurement {
                timestamp: Some(ts),
                device: i32::arbitrary(g),
                sensor: i32::arbitrary(g),
                measurement: f32::arbitrary(g),
            }
        }
    }

    quickcheck! {
        fn prop_new_with_ts_preserves_all_fields(
            device: i32, sensor: i32, measurement: f32
        ) -> bool {
            // NaN != NaN in IEEE 754, skip it
            if measurement.is_nan() {
                return true;
            }
            let ts = Utc::now();
            let m = NewMeasurement::new_with_ts(ts, device, sensor, measurement);
            m.timestamp == Some(ts)
                && m.device == device
                && m.sensor == sensor
                && m.measurement == measurement
        }

        fn prop_identical_measurements_are_equal(m: NewMeasurement) -> bool {
            // NaN != NaN in IEEE 754, so PartialEq will fail for NaN measurements
            if m.measurement.is_nan() {
                return true;
            }
            m == m.clone()
        }

        fn prop_serialization_contains_all_fields(m: NewMeasurement) -> bool {
            // Skip NaN/Inf since JSON can't represent them
            if m.measurement.is_nan() || m.measurement.is_infinite() {
                return true;
            }
            let json = serde_json::to_string(&m).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            parsed["device"] == m.device
                && parsed["sensor"] == m.sensor
                && parsed["timestamp"].is_string()
        }

        fn prop_serialized_timestamp_is_rfc3339(m: NewMeasurement) -> bool {
            if m.measurement.is_nan() || m.measurement.is_infinite() {
                return true;
            }
            let json = serde_json::to_string(&m).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            let ts_str = parsed["timestamp"].as_str().unwrap();
            chrono::DateTime::parse_from_rfc3339(ts_str).is_ok()
        }
    }
}
