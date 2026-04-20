use super::super::*;
use std::{collections::HashMap, io::ErrorKind};

fn headers_to_map(headers: Vec<(String, String)>) -> HashMap<String, String> {
    headers
        .into_iter()
        .map(|(k, v)| (k.to_ascii_lowercase(), v))
        .collect()
}

#[test]
fn build_headers_rejects_invalid_name() {
    let mut headers = HashMap::new();
    headers.insert("bad header".to_owned(), "ok".to_owned());
    let endpoint = HttpEndpoint::new(HttpMethod::Post, "https://example.com", headers);
    let err = endpoint
        .build_headers(&HashMap::new())
        .expect_err("invalid header name must fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("invalid header name"));
}

#[test]
fn build_headers_allows_case_insensitive_allowlisted_overrides() {
    let mut static_headers = HashMap::new();
    static_headers.insert("X-Fixed".to_owned(), "fixed".to_owned());
    let endpoint = HttpEndpoint::new(HttpMethod::Post, "https://example.com", static_headers)
        .with_overridable_request_headers(vec!["x-fixed".to_owned(), "content-type".to_owned()]);

    let mut overrides = HashMap::new();
    overrides.insert("X-FiXeD".to_owned(), "overridden".to_owned());
    overrides.insert(
        "Content-Type".to_owned(),
        "application/custom+json".to_owned(),
    );

    let headers = endpoint
        .build_headers(&overrides)
        .expect("allowlisted overrides should succeed");
    let headers = headers_to_map(headers);
    assert_eq!(headers["x-fixed"], "overridden");
    assert_eq!(headers["content-type"], "application/custom+json");
}

#[test]
fn build_headers_rejects_non_allowlisted_override() {
    let endpoint = HttpEndpoint::new(HttpMethod::Post, "https://example.com", HashMap::new())
        .with_overridable_request_headers(vec!["x-allowed".to_owned()]);
    let overrides = HashMap::from([("x-not-allowed".to_owned(), "value".to_owned())]);

    let err = endpoint
        .build_headers(&overrides)
        .expect_err("non-allowlisted override should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("not allowlisted"));
}

#[test]
fn build_headers_applies_intended_precedence_order() {
    let endpoint = HttpEndpoint::new(
        HttpMethod::Post,
        "https://example.com",
        HashMap::from([
            ("content-type".to_owned(), "configured/type".to_owned()),
            ("x-static".to_owned(), "configured".to_owned()),
        ]),
    )
    .with_overridable_request_headers(vec!["content-type".to_owned()]);

    let overrides = HashMap::from([("content-type".to_owned(), "override/type".to_owned())]);

    let headers = endpoint
        .build_headers(&overrides)
        .expect("header layering should succeed");
    let headers = headers_to_map(headers);

    assert_eq!(headers["content-type"], "override/type");
    assert_eq!(headers["x-static"], "configured");
}
