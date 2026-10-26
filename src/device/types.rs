use serde::{Deserialize, Serialize};

use super::DeviceId;

/// A device that has been registered with hemrs and therefore has an id.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Device {
    pub id: DeviceId,
    pub name: String,
    pub location: String,
}

/// A device description that has not yet been registered with hemrs.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct NewDevice {
    pub name: String,
    pub location: String,
}

impl NewDevice {
    pub fn matches(&self, device: &Device) -> bool {
        self.name == device.name && self.location == device.location
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen, quickcheck};

    impl Arbitrary for Device {
        fn arbitrary(g: &mut Gen) -> Self {
            Device {
                id: i32::arbitrary(g),
                name: String::arbitrary(g),
                location: String::arbitrary(g),
            }
        }
    }

    impl Arbitrary for NewDevice {
        fn arbitrary(g: &mut Gen) -> Self {
            NewDevice {
                name: String::arbitrary(g),
                location: String::arbitrary(g),
            }
        }
    }

    quickcheck! {
        fn prop_new_device_serializes_exactly_name_and_location(device: NewDevice) -> bool {
            let value = serde_json::to_value(&device).unwrap();
            value == serde_json::json!({ "name": device.name, "location": device.location })
        }

        fn prop_deserialization_round_trips(device: Device) -> bool {
            let json = serde_json::json!({
                "id": device.id,
                "name": device.name,
                "location": device.location,
            });
            serde_json::from_value::<Device>(json).unwrap() == device
        }

        fn prop_new_device_matches_only_on_name_and_location(device: Device, other_location: String) -> bool {
            let same = NewDevice { name: device.name.clone(), location: device.location.clone() };
            let different = NewDevice { name: device.name.clone(), location: other_location.clone() };
            same.matches(&device) && different.matches(&device) == (other_location == device.location)
        }
    }
}
