use crate::error::StorageError;

pub fn url_decode(s: &str) -> Result<String, StorageError> {
    urlencoding::decode(s)
        .map(|cow| cow.into_owned())
        .map_err(|e| StorageError::UrlDecode(format!("Failed to decode '{}': {}", s, e)))
}

pub fn clean_value(raw: &str) -> String {
    let mut v = raw.trim().to_string();

    if v.starts_with("Some(") && v.ends_with(')') {
        v = v[5..v.len() - 1].to_string();
    }

    if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
        v = v[1..v.len() - 1].to_string();
    }

    v.trim().to_string()
}
