#[cfg(test)]
use super::MechanicsConfig;
use super::{
    query::{
        resolve_slotted_query_value, validate_byte_len, validate_min_max_bounds,
        validate_query_key, validate_slot_name,
    },
    retry::EndpointRetryPolicy,
    template::{UrlTemplateChunk, percent_encode_component},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

mod validate;

/// Supported HTTP methods for runtime-managed endpoint calls.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum HttpMethod {
    /// HTTP `GET`.
    Get,
    /// HTTP `POST`.
    Post,
    /// HTTP `PUT`.
    Put,
    /// HTTP `PATCH`.
    Patch,
    /// HTTP `DELETE`.
    Delete,
    /// HTTP `HEAD`.
    Head,
    /// HTTP `OPTIONS`.
    Options,
}

impl HttpMethod {
    /// Returns the canonical uppercase method token.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }

    /// Whether this method supports a request body under mechanics semantics.
    pub fn supports_request_body(&self) -> bool {
        matches!(self, Self::Post | Self::Put | Self::Patch)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
/// Body encoding/decoding mode used for endpoint request/response handling.
pub enum EndpointBodyType {
    /// JSON payload (`application/json`).
    #[default]
    Json,
    /// UTF-8 string payload (`text/plain; charset=utf-8`).
    Utf8,
    /// Raw bytes payload (`application/octet-stream`).
    Bytes,
}

/// Validation and default policy for one URL template slot.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct UrlParamSpec {
    /// Optional fallback value used when the JS-provided value is missing or empty.
    #[serde(default)]
    pub default: Option<String>,
    /// Optional minimum UTF-8 byte length accepted for the resolved value.
    #[serde(default)]
    pub min_bytes: Option<usize>,
    /// Optional maximum UTF-8 byte length accepted for the resolved value.
    #[serde(default)]
    pub max_bytes: Option<usize>,
}

impl UrlParamSpec {
    fn resolve_value(&self, slot: &str, provided: Option<&str>) -> std::io::Result<String> {
        validate_min_max_bounds(slot, self.min_bytes, self.max_bytes)?;
        let value = match provided {
            Some(v) if !v.is_empty() => v,
            Some(_) | None => self.default.as_deref().unwrap_or(""),
        };
        validate_byte_len(slot, value, self.min_bytes, self.max_bytes)?;
        Ok(value.to_owned())
    }
}

/// Emission mode for a slotted query parameter.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum SlottedQueryMode {
    /// Slot must resolve and must be non-empty.
    #[default]
    Required,
    /// Slot must resolve and may be empty.
    RequiredAllowEmpty,
    /// Missing/empty is treated as omitted.
    Optional,
    /// Missing is omitted; if provided, empty is emitted.
    OptionalAllowEmpty,
}

/// One query emission rule.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum QuerySpec {
    /// Emits a constant key/value pair.
    Const {
        /// Query key to emit.
        key: String,
        /// Constant value to emit.
        value: String,
    },
    /// Emits a query pair from a JS slot (`queries[slot]`) under configured policy.
    Slotted {
        /// Query key to emit.
        key: String,
        /// JS `queries` slot name to read.
        slot: String,
        /// Resolution and omission policy.
        #[serde(default)]
        mode: SlottedQueryMode,
        /// Optional fallback value used when slot input is missing.
        #[serde(default)]
        default: Option<String>,
        /// Optional minimum UTF-8 byte length for emitted value.
        #[serde(default)]
        min_bytes: Option<usize>,
        /// Optional maximum UTF-8 byte length for emitted value.
        #[serde(default)]
        max_bytes: Option<usize>,
    },
}

/// HTTP endpoint configuration used by the runtime-provided JS helper.
///
/// Endpoint definitions are pure configuration inputs and should be treated as stateless.
/// Any caching behavior should be implemented outside this crate.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct HttpEndpoint {
    method: HttpMethod,
    url_template: String,
    #[serde(default)]
    url_param_specs: HashMap<String, UrlParamSpec>,
    #[serde(default)]
    query_specs: Vec<QuerySpec>,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    overridable_request_headers: Vec<String>,
    #[serde(default)]
    exposed_response_headers: Vec<String>,
    #[serde(default)]
    request_body_type: Option<EndpointBodyType>,
    #[serde(default)]
    response_body_type: EndpointBodyType,
    #[serde(default)]
    response_max_bytes: Option<usize>,
    timeout_ms: Option<u64>,
    #[serde(default)]
    allow_non_2xx_status: bool,
    #[serde(default)]
    retry_policy: EndpointRetryPolicy,
}

impl HttpEndpoint {
    /// Constructs an endpoint definition used by runtime-owned HTTP helpers.
    pub fn new(method: HttpMethod, url_template: &str, headers: HashMap<String, String>) -> Self {
        Self {
            method,
            url_template: url_template.to_owned(),
            url_param_specs: HashMap::new(),
            query_specs: Vec::new(),
            headers,
            overridable_request_headers: Vec::new(),
            exposed_response_headers: Vec::new(),
            request_body_type: None,
            response_body_type: EndpointBodyType::Json,
            response_max_bytes: None,
            timeout_ms: None,
            allow_non_2xx_status: false,
            retry_policy: EndpointRetryPolicy::default(),
        }
    }

    /// Replaces URL slot constraints used by `url_template` placeholders.
    pub fn with_url_param_specs(mut self, url_param_specs: HashMap<String, UrlParamSpec>) -> Self {
        self.url_param_specs = url_param_specs;
        self
    }

    /// Replaces query emission rules.
    pub fn with_query_specs(mut self, query_specs: Vec<QuerySpec>) -> Self {
        self.query_specs = query_specs;
        self
    }

    /// Sets request body decoding mode.
    ///
    /// If unset, request body mode defaults to JSON.
    pub fn with_request_body_type(mut self, body_type: EndpointBodyType) -> Self {
        self.request_body_type = Some(body_type);
        self
    }

    /// Sets request header names that JS may override via `endpoint(..., { headers })`.
    ///
    /// Matching is case-insensitive.
    pub fn with_overridable_request_headers(mut self, headers: Vec<String>) -> Self {
        self.overridable_request_headers = headers;
        self
    }

    /// Sets response header names that are exposed to JS in endpoint response objects.
    ///
    /// Matching is case-insensitive.
    pub fn with_exposed_response_headers(mut self, headers: Vec<String>) -> Self {
        self.exposed_response_headers = headers;
        self
    }

    /// Sets response body decoding mode.
    ///
    /// Defaults to JSON.
    pub fn with_response_body_type(mut self, body_type: EndpointBodyType) -> Self {
        self.response_body_type = body_type;
        self
    }

    /// Sets a per-endpoint maximum response-body size in bytes.
    ///
    /// If this is `Some`, it overrides the pool default response limit.
    /// If this is `None`, the pool default response limit is used.
    pub fn with_response_max_bytes(mut self, response_max_bytes: Option<usize>) -> Self {
        self.response_max_bytes = response_max_bytes;
        self
    }

    /// Sets a per-endpoint timeout in milliseconds.
    ///
    /// If this is `Some`, it overrides the pool default endpoint timeout.
    /// If this is `None`, the pool default timeout is used.
    pub fn with_timeout_ms(mut self, timeout_ms: Option<u64>) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Allows non-success (non-2xx) HTTP status responses to proceed.
    ///
    /// Defaults to `false`, which treats non-success statuses as request errors.
    pub fn with_allow_non_2xx_status(mut self, allow: bool) -> Self {
        self.allow_non_2xx_status = allow;
        self
    }

    /// Sets endpoint retry/backoff/rate-limit policy.
    pub fn with_retry_policy(mut self, retry_policy: EndpointRetryPolicy) -> Self {
        self.retry_policy = retry_policy;
        self
    }

    /// Returns the endpoint HTTP method.
    pub fn method(&self) -> HttpMethod {
        self.method
    }

    /// Returns configured URL template.
    pub fn url_template(&self) -> &str {
        &self.url_template
    }

    /// Returns configured URL parameter constraints.
    pub fn url_param_specs(&self) -> &HashMap<String, UrlParamSpec> {
        &self.url_param_specs
    }

    /// Returns configured query emission rules.
    pub fn query_specs(&self) -> &[QuerySpec] {
        &self.query_specs
    }

    /// Returns configured static request headers.
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    /// Returns request header names that JS can override.
    pub fn overridable_request_headers(&self) -> &[String] {
        &self.overridable_request_headers
    }

    /// Returns response header names that are exposed to JS.
    pub fn exposed_response_headers(&self) -> &[String] {
        &self.exposed_response_headers
    }

    /// Returns request body mode if explicitly configured.
    pub fn request_body_type(&self) -> Option<EndpointBodyType> {
        self.request_body_type
    }

    /// Returns request body mode after applying endpoint defaults.
    pub fn effective_request_body_type(&self) -> EndpointBodyType {
        self.request_body_type.unwrap_or(EndpointBodyType::Json)
    }

    /// Returns response body decoding mode.
    pub fn response_body_type(&self) -> EndpointBodyType {
        self.response_body_type
    }

    /// Returns endpoint-specific response byte limit.
    pub fn response_max_bytes(&self) -> Option<usize> {
        self.response_max_bytes
    }

    /// Returns endpoint-specific timeout in milliseconds.
    pub fn timeout_ms(&self) -> Option<u64> {
        self.timeout_ms
    }

    /// Returns whether non-2xx statuses are allowed.
    pub fn allow_non_2xx_status(&self) -> bool {
        self.allow_non_2xx_status
    }

    /// Returns endpoint retry policy.
    pub fn retry_policy(&self) -> &EndpointRetryPolicy {
        &self.retry_policy
    }

    /// Builds an absolute request URL from provided URL params and query slots.
    pub fn build_url(
        &self,
        url_params: &HashMap<String, String>,
        queries: &HashMap<String, String>,
    ) -> std::io::Result<String> {
        let prepared = self.prepare_runtime()?;
        self.build_url_prepared(url_params, queries, &prepared)
    }

    /// Validates and layers endpoint-configured headers with call-time overrides.
    pub fn build_headers(
        &self,
        overrides: &HashMap<String, String>,
    ) -> std::io::Result<Vec<(String, String)>> {
        let prepared = self.prepare_runtime()?;
        self.build_headers_prepared(overrides, &prepared)
    }

    pub(crate) fn build_url_prepared(
        &self,
        url_params: &HashMap<String, String>,
        queries: &HashMap<String, String>,
        prepared: &PreparedHttpEndpoint,
    ) -> std::io::Result<String> {
        debug_assert!(
            prepared
                .url_slot_names
                .iter()
                .all(|slot| self.url_param_specs.contains_key(slot))
        );
        debug_assert!(
            self.url_param_specs
                .keys()
                .all(|configured| prepared.url_slot_set.contains(configured))
        );

        for provided in url_params.keys() {
            if !prepared.url_slot_set.contains(provided) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "unknown urlParams key `{provided}` for endpoint template `{}`",
                        self.url_template
                    ),
                ));
            }
        }

        let mut resolved_url = String::with_capacity(self.url_template.len().saturating_add(16));
        for chunk in &prepared.parsed_url_chunks {
            match chunk {
                UrlTemplateChunk::Literal(s) => resolved_url.push_str(s),
                UrlTemplateChunk::Slot(slot) => {
                    let spec =
                        self.url_param_specs
                            .get(slot.as_str())
                            .ok_or(std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                format!("missing url_param_specs entry for slot `{slot}`"),
                            ))?;
                    let provided = url_params.get(slot.as_str()).map(String::as_str);
                    let value = spec.resolve_value(slot, provided)?;
                    resolved_url.push_str(&percent_encode_component(&value));
                }
            }
        }

        let mut url = url::Url::parse(&resolved_url).map_err(std::io::Error::other)?;
        if url.fragment().is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "url_template must not include URL fragments",
            ));
        }
        if url.query().is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "url_template must not include query parameters; use query_specs instead",
            ));
        }

        for provided in queries.keys() {
            if !prepared.allowed_query_slots.contains(provided) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown queries key `{provided}` for endpoint"),
                ));
            }
        }

        let mut emitted_pairs = Vec::<(String, String)>::new();
        for spec in &self.query_specs {
            match spec {
                QuerySpec::Const { key, value } => {
                    validate_query_key(key)?;
                    emitted_pairs.push((key.clone(), value.clone()));
                }
                QuerySpec::Slotted {
                    key,
                    slot,
                    mode,
                    default,
                    min_bytes,
                    max_bytes,
                } => {
                    validate_query_key(key)?;
                    validate_slot_name(slot)?;
                    validate_min_max_bounds(slot, *min_bytes, *max_bytes)?;

                    let provided = queries.get(slot).map(String::as_str);
                    if let Some(value) = resolve_slotted_query_value(
                        slot,
                        *mode,
                        default.as_deref(),
                        provided,
                        *min_bytes,
                        *max_bytes,
                    )? {
                        emitted_pairs.push((key.clone(), value));
                    }
                }
            }
        }

        if !emitted_pairs.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in emitted_pairs {
                pairs.append_pair(&key, &value);
            }
        }

        Ok(url.into())
    }

    pub(crate) fn build_headers_prepared(
        &self,
        overrides: &HashMap<String, String>,
        prepared: &PreparedHttpEndpoint,
    ) -> std::io::Result<Vec<(String, String)>> {
        use crate::headers::{normalize_header_name, validate_header_value};

        let mut layered = Vec::with_capacity(self.headers.len().saturating_add(overrides.len()));

        for (name, value) in &self.headers {
            normalize_header_name(name, "headers")?;
            validate_header_value(name, value, "headers")?;
            layered.push((name.clone(), value.clone()));
        }

        let mut seen_override_names = HashSet::new();
        for (name, value) in overrides {
            let normalized = normalize_header_name(name, "headers")?;
            if !seen_override_names.insert(normalized.clone()) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "duplicate override header `{name}` in options.headers (case-insensitive)"
                    ),
                ));
            }
            if !prepared.allowed_overrides.contains(&normalized) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "override header `{name}` is not allowlisted in overridable_request_headers"
                    ),
                ));
            }
            validate_header_value(name, value, "headers")?;
            layered.push((name.clone(), value.clone()));
        }

        Ok(layered)
    }
}

/// Pre-parsed endpoint state ready for repeated URL/header building.
#[derive(Clone, Debug)]
pub struct PreparedHttpEndpoint {
    parsed_url_chunks: Vec<UrlTemplateChunk>,
    url_slot_names: Vec<String>,
    url_slot_set: HashSet<String>,
    allowed_query_slots: HashSet<String>,
    allowed_overrides: HashSet<String>,
    exposed_response_allowlist: HashSet<String>,
}

impl PreparedHttpEndpoint {
    /// Returns normalized response-header allowlist names.
    pub fn exposed_response_allowlist(&self) -> &HashSet<String> {
        &self.exposed_response_allowlist
    }
}

#[cfg(test)]
#[path = "../tests/mod.rs"]
mod tests;
