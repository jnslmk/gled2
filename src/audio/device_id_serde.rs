use cpal::DeviceId;
use serde::{Deserialize, Serialize, Serializer};
use std::str::FromStr;


pub fn serialize_device_id<S>(device_id: &Option<DeviceId>, s: S)
                              -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    device_id.as_ref().map(|id| id.to_string()).serialize(s)
}




pub fn deserialize_scene_instances<'de, D>(
    deserializer: D,
) -> Result<Option<DeviceId>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let serialized = Option::<String>::deserialize(deserializer)?;
    serialized.map(|s| DeviceId::from_str(&*s).map_err(|_| serde::de::Error::custom("Invalid device id"))).transpose()
}