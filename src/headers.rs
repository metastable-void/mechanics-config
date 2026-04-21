use http::header::{HeaderName, HeaderValue};
use std::{
    collections::HashSet,
    io::{Error, ErrorKind},
};

pub(crate) fn normalize_header_name(header: &str, field_name: &str) -> std::io::Result<String> {
    HeaderName::from_bytes(header.as_bytes()).map_err(|e| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("invalid header name `{header}` in `{field_name}`: {e}"),
        )
    })?;
    Ok(header.to_ascii_lowercase())
}

pub(crate) fn validate_header_name_list(
    headers: &[String],
    field_name: &str,
) -> std::io::Result<()> {
    for header in headers {
        normalize_header_name(header, field_name)?;
    }
    Ok(())
}

pub(crate) fn allowlisted_header_names(
    headers: &[String],
    field_name: &str,
) -> std::io::Result<HashSet<String>> {
    headers
        .iter()
        .map(|header| normalize_header_name(header, field_name))
        .collect()
}

pub(crate) fn validate_header_value(
    name: &str,
    value: &str,
    field_name: &str,
) -> std::io::Result<()> {
    HeaderValue::from_str(value).map_err(|e| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("invalid header value for `{name}` in `{field_name}`: {e}"),
        )
    })?;
    Ok(())
}
