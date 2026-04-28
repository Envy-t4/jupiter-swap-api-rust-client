use serde::Serializer;

pub fn serialize<S>(vec: &Option<Vec<String>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match vec {
        Some(v) if !v.is_empty() => {
            let joined = v.join(",");
            serializer.serialize_str(&joined)
        }
        _ => serializer.serialize_none(),
    }
}