use {serde::{Deserialize, Deserializer, Serialize, Serializer}, std::str::FromStr};

pub fn serialize<T, S>(t: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    T: ToString,
    S: Serializer,
{
    if let Some(t) = t {
        t.to_string().serialize(serializer)
    } else {
        serializer.serialize_none()
    }
}

pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: FromStr,
    T::Err: std::fmt::Display,
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) => s
            .parse::<T>()
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}
