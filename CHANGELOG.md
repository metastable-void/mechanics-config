# Changelog

All notable changes to this crate are documented in this file.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1]

- Added crate-level and method doc comments.

## [0.1.0]

Initial published API extracted from `mechanics-core`:

- Added schema types: `MechanicsConfig`, `HttpEndpoint`, `HttpMethod`,
  URL-template/query/header/retry supporting types.
- Added pure structural validation (`MechanicsConfig::validate`,
  `HttpEndpoint::validate_config`) and URL/header/query resolution helpers.
- Added schema-focused tests migrated from `mechanics-core`.
- Kept the crate Boa-free and runtime-free (`boa_*`, `reqwest`, `tokio` absent).

## [0.0.0]

Name reservation on crates.io. No functional content yet.
