# Changelog

## [Unreleased]

### Added
- `tracing` instrumentation — structured spans and events for API requests, HTTP transport, and webhook verification
- `secrecy` crate integration — `api_token` and webhook `secret` are now `SecretString`
- Re-exported `secrecy::{ExposeSecret, SecretString}` from crate root
- `Endpoint::parse_response()` for custom (non-JSON) response parsing
- `BatchError` enum (`Empty`, `TooLarge`) for batch construction validation
- `WebhookError::EmptySecret` variant
- `EmailStatus::Unknown` catch-all variant (`#[serde(other)]`) for forward compatibility
- `#[non_exhaustive]` on `EmailStatus` and `QueryError`
- `QueryError::SerializeBody` variant (split from former `Json`)
- E2E test suite: live API tests (`#[ignore]`) and `wiremock`-based mock tests
- `wiremock` and `dotenvy` dev-dependencies

### Changed
- Migrated from `typed-builder` to `bon` for all builder derives
- `LettermintClient` uses builder pattern (`builder().api_token().build()`)
- `Webhook` uses builder pattern (`builder().secret().build()`) and returns `Result` instead of panicking
- `BatchSendRequest::new()` returns `Result<Self, BatchError>` instead of `Option`
- `Attachment` uses builder pattern
- `PingResponse` field changed from `status: u16` to `message: String` (plain-text `"pong"`)
- `QueryError::Json` renamed to `QueryError::DeserializeResponse`
- Version bumped to 0.3.0

### Removed
- `typed-builder` dependency
- `LettermintClient::new()`, `with_base_url()`, `with_reqwest_client()` (use `builder()`)
- `Webhook::with_tolerance()` (use `builder().tolerance()`)
- `Attachment::new()`, `Attachment::inline()`, `Attachment::with_content_type()` (use `builder()`)
- `tests/integration.rs` (replaced by `tests/e2e.rs`)

## [0.2.2] - 2026-04-10

### Added
- `testing::emails` module with `Scenario` enum for CI/testing email addresses
- `Scenario::email()` for base addresses, `Scenario::random()` for unique addresses
- `emails::custom()` for arbitrary local parts

### Changed
- Bumped `hmac` to 0.13, `sha2` to 0.11

## [0.2.0] - 2026-03-27

### Changed
- Rust edition bumped to 2024
- Replaced `async-trait` with native async fn in traits

### Removed
- `async-trait` dependency

## [0.1.1] - 2026-03-27

### Added
- Batch sending via `BatchSendRequest` (up to 500 emails per request)
- `PingRequest` endpoint for health checks and credential validation
- `WebhookEvent` struct with event type, delivery timestamp, and attempt number
- `content_type` field on `Attachment` for explicit MIME types
- Granular error variants: `Validation` (422), `Authentication` (401/403), `RateLimit` (429)
- `EmailStatus` variants: `Suppressed`, `Opened`, `Clicked`, `SpamComplaint`, `Blocked`, `PolicyRejected`, `Unsubscribed`

### Changed
- `Webhook::verify_headers` now accepts event/attempt headers and returns `WebhookEvent`
- `QueryError::Api` split into specific variants; generic `Api` remains as catch-all for other status codes

### Removed
- `Webhook::verify_once` (use `Webhook::new(secret).verify(...)` instead)
