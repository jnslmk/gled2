use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;

pub fn deserialize_usize_index_btreemap<'de, D, T: DeserializeOwned>(
    deserializer: D,
) -> Result<BTreeMap<usize, T>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    BTreeMap::<String, T>::deserialize(deserializer).map(|map| {
        map.into_iter()
            .filter_map(|(index, value)| index.parse().ok().map(|index| (index, value)))
            .collect()
    })
}

pub fn deserialize_u16_index_btreemap<'de, D, T: DeserializeOwned>(
    deserializer: D,
) -> Result<BTreeMap<u16, T>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    BTreeMap::<String, T>::deserialize(deserializer).map(|map| {
        map.into_iter()
            .filter_map(|(index, value)| index.parse().ok().map(|index| (index, value)))
            .collect()
    })
}
