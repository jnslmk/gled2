use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;

pub fn deserialize_index_btreemap<'de, D, T: DeserializeOwned>(
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
