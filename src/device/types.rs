use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Device {
    #[serde(skip_serializing)]
    pub id: i32,
    pub name: String,
    pub location: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{quickcheck, Arbitrary, Gen};

    impl Arbitrary for Device {
        fn arbitrary(g: &mut Gen) -> Self {
            Device {
                id: i32::arbitrary(g),
                name: String::arbitrary(g),
                location: String::arbitrary(g),
            }
        }
    }

    quickcheck! {
        fn prop_identical_devices_are_equal(device: Device) -> bool {
            device == device.clone()
        }

        fn prop_differing_field_means_inequality(
            id: i32, name: String, loc_a: String, loc_b: String
        ) -> bool {
            if loc_a == loc_b {
                return true; // vacuously true when locations happen to match
            }
            let d1 = Device { id, name: name.clone(), location: loc_a };
            let d2 = Device { id, name, location: loc_b };
            d1 != d2
        }

        fn prop_serialization_excludes_id(device: Device) -> bool {
            let json = serde_json::to_string(&device).unwrap();
            // id must not appear in serialized output
            !json.contains("\"id\"")
        }

        fn prop_serialization_preserves_name_and_location(device: Device) -> bool {
            let json = serde_json::to_string(&device).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            parsed["name"] == device.name && parsed["location"] == device.location
        }

        fn prop_deserialization_round_trips(device: Device) -> bool {
            // Serialize with id manually (since skip_serializing omits it),
            // then deserialize and check all fields match.
            let json = format!(
                r#"{{"id":{},"name":{},"location":{}}}"#,
                device.id,
                serde_json::to_string(&device.name).unwrap(),
                serde_json::to_string(&device.location).unwrap()
            );
            let parsed: Device = serde_json::from_str(&json).unwrap();
            parsed == device
        }
    }
}
