use serde::{Deserialize, Serialize};

/// A sensor that has been registered with hemrs and therefore has an id.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Sensor {
    pub id: i32,
    pub name: String,
    pub unit: String,
}

/// A sensor description that has not yet been registered with hemrs.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct NewSensor {
    pub name: String,
    pub unit: String,
}

impl NewSensor {
    pub fn matches(&self, sensor: &Sensor) -> bool {
        self.name == sensor.name
    }
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

    impl Arbitrary for NewSensor {
        fn arbitrary(g: &mut Gen) -> Self {
            NewSensor {
                name: String::arbitrary(g),
                unit: String::arbitrary(g),
            }
        }
    }

    quickcheck! {
        fn prop_new_sensor_serializes_exactly_name_and_unit(sensor: NewSensor) -> bool {
            let value = serde_json::to_value(&sensor).unwrap();
            value == serde_json::json!({ "name": sensor.name, "unit": sensor.unit })
        }

        fn prop_deserialization_round_trips(sensor: Sensor) -> bool {
            let json = serde_json::json!({
                "id": sensor.id,
                "name": sensor.name,
                "unit": sensor.unit,
            });
            serde_json::from_value::<Sensor>(json).unwrap() == sensor
        }

        fn prop_new_sensor_matches_registered_sensor_with_same_name(sensor: Sensor) -> bool {
            NewSensor { name: sensor.name.clone(), unit: String::new() }.matches(&sensor)
        }
    }
}
