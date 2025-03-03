use serde::{Deserialize, Deserializer};

pub fn fail_as_none<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    // We must fully consume valid json otherwise the error leaves the
    // deserializer in an invalid state and it'll still fail
    //
    // This assumes we always use a json format
    let intermediate = serde_json::Value::deserialize(deserializer)?;

    Ok(Option::<T>::deserialize(intermediate).unwrap_or_default())
}
