# mechanics-config

Schema types and pure-Rust validation helpers for mechanics-style
HTTP endpoints and job configuration. Extracted from
[`mechanics-core`](https://crates.io/crates/mechanics-core) so the
same schema can be consumed from contexts that don't want to pull
in the Boa JavaScript engine, a Tokio runtime, or an HTTP client.

## What this crate provides

- `MechanicsConfig` — a per-job endpoint map with validation on
  construction and mutation.
- `HttpEndpoint` / `PreparedHttpEndpoint` — one endpoint definition
  with URL template, method, headers, query policy, body mode, and
  retry policy. `prepare()` resolves slot values into a request-ready
  form.
- `HttpMethod`, `EndpointBodyType` — the supported method set
  (`GET`/`POST`/`PUT`/`PATCH`/`DELETE`/`HEAD`/`OPTIONS`) and body
  encodings (`json`/`utf8`/`bytes`).
- `UrlParamSpec`, `QuerySpec`, `SlottedQueryMode` — per-slot
  validation and default policy for URL template variables and
  query parameters, including min/max byte-length bounds and
  slotted-query resolution modes.
- `EndpointRetryPolicy` — JSON-deserializable retry/backoff/
  rate-limit policy.

All definition types (`MechanicsConfig`, `HttpEndpoint`,
`UrlParamSpec`, `QuerySpec`, `EndpointRetryPolicy`, method/body
enums) implement `serde::Serialize` + `serde::Deserialize` with
`#[serde(deny_unknown_fields)]` on struct definitions, so JSON
parseability is a first-class concern. `PreparedHttpEndpoint` is
the runtime-resolved form and is not serde-round-trippable.

## Design properties

- **Boa-free.** No transitive dependency on `boa_engine`, `boa_gc`,
  or any JavaScript engine. This crate can be used from contexts
  that never embed JS.
- **Runtime-free.** No transitive dependency on `tokio`, `reqwest`,
  or any async runtime. Validation is synchronous and side-effect
  free.
- **Validation on construction.** `MechanicsConfig::new`,
  `::with_endpoint`, and `::with_endpoint_overrides` run the full
  validator on every endpoint. Invalid configs can't be assembled
  — they fail at construction, not at call time.
- **Small dependency surface.** Only `serde`, `serde_json`, `http`,
  and `url`.

## Who uses this

- [`mechanics-core`](https://crates.io/crates/mechanics-core) —
  re-exports `MechanicsConfig`, `HttpEndpoint`, and the supporting
  types so existing users' imports keep working. mechanics-core
  wraps these in Boa-GC-aware newtypes for the JS runtime side.
- The Philharmonic connector lowerer (in the sibling
  `philharmonic-workspace`) — consumes endpoint schemas without
  running any JavaScript, so it wants the schema types without
  paying for Boa.

If you're writing automation jobs that execute JavaScript, depend
on `mechanics-core` directly; its public API re-exports these types.
If you're consuming mechanics-style endpoint schemas from a
non-JavaScript context (configuration tooling, schema validators,
declarative lowerers), depend on `mechanics-config` and skip the
Boa/Tokio weight.

## Basic usage

```rust
use mechanics_config::{HttpEndpoint, MechanicsConfig};
use std::collections::HashMap;

// Parse one endpoint from JSON — serde handles structural
// validation (unknown fields rejected, required fields enforced).
let endpoint: HttpEndpoint = serde_json::from_str(r#"{
    "method": "get",
    "url_template": "https://api.example.com/v1/users/{id}",
    "url_param_specs": {
        "id": { "min_bytes": 1, "max_bytes": 64 }
    }
}"#)?;

// Assembling a MechanicsConfig runs the semantic validator on
// every endpoint. An invalid config fails here, not at call time.
let mut endpoints = HashMap::new();
endpoints.insert("get_user".to_string(), endpoint);
let config = MechanicsConfig::new(endpoints)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## In scope / out of scope

### In scope
- Declarative schema types for HTTP endpoint configuration.
- Pure-Rust validation helpers (synchronous, side-effect free).
- URL template / query / header resolution with explicit slot
  policy enforcement.
- `serde` round-tripping suitable for JSON-first configuration.

### Out of scope
- HTTP request execution — this crate never performs I/O.
- JavaScript embedding — no Boa types, no GC integration.
- Async runtime coupling — no Tokio, no `async fn`.
- Cross-process scheduling, persistence, or observability.

## License

Licensed under either [Apache License, Version 2.0](LICENSE-APACHE)
or [Mozilla Public License, Version 2.0](LICENSE-MPL) at your
option. SPDX: `Apache-2.0 OR MPL-2.0`.
