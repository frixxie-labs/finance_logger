use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Sensor {
    #[serde(skip_serializing)]
    pub id: i32,
    pub name: String,
    pub unit: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen, quickcheck};

    impl Arbitrary for Sensor {
        fn arbitrary(g: &mut Gen) -> Self {
            Sensor {
                id: i32::arbitrary(g),
                name: String::arbitrary(g),
                unit: String::arbitrary(g),
            }
        }
    }

    quickcheck! {
        fn prop_identical_sensors_are_equal(sensor: Sensor) -> bool {
            sensor == sensor.clone()
        }

        fn prop_serialization_excludes_id(sensor: Sensor) -> bool {
            let json = serde_json::to_string(&sensor).unwrap();
            !json.contains("\"id\"")
        }

        fn prop_serialization_preserves_name_and_unit(sensor: Sensor) -> bool {
            let json = serde_json::to_string(&sensor).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            parsed["name"] == sensor.name && parsed["unit"] == sensor.unit
        }

        fn prop_deserialization_round_trips(sensor: Sensor) -> bool {
            let json = format!(
                r#"{{"id":{},"name":{},"unit":{}}}"#,
                sensor.id,
                serde_json::to_string(&sensor.name).unwrap(),
                serde_json::to_string(&sensor.unit).unwrap()
            );
            let parsed: Sensor = serde_json::from_str(&json).unwrap();
            parsed == sensor
        }
    }
}
