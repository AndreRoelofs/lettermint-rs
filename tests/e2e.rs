//! End-to-end tests for the Lettermint Rust SDK.
//!
//! ## `mod live` — tests against the real Lettermint API
//!
//! All marked `#[ignore]` so they don't run in CI by default. They require:
//!
//! - `LETTERMINT_API_TOKEN` — a valid API token
//! - `LETTERMINT_SENDER` — a verified sender address (e.g. `you@yourdomain.com`)
//!
//! Run with:
//! ```sh
//! cargo test --test e2e --all-features -- --ignored
//! ```
//!
//! Lettermint provides test addresses at `@testing.lettermint.co` that don't
//! count toward quotas or affect bounce/complaint rates.
//!
//! ## `mod mock` — tests against a local [`wiremock`] server
//!
//! These always run (no env vars needed) and cover things that can't be verified
//! against the real API: exact header values, request body structure, and error
//! codes that aren't triggerable on demand (403, 429, 500, 502).
//!
//! Run with:
//! ```sh
//! cargo test --test e2e --all-features
//! ```

// ---------------------------------------------------------------------------
// Live API tests
// ---------------------------------------------------------------------------

mod live {
    use std::collections::HashMap;
    use std::sync::Once;

    use lettermint_rs::api::email::*;
    use lettermint_rs::api::ping::PingRequest;
    use lettermint_rs::reqwest::{LettermintClient, LettermintClientError};
    use lettermint_rs::testing::emails::{self, Scenario};
    use lettermint_rs::{Query, QueryError};

    type Result = std::result::Result<(), Box<dyn std::error::Error>>;

    static INIT: Once = Once::new();

    /// Load `.env` (if present) exactly once, then read env vars.
    fn load_env() {
        INIT.call_once(|| {
            dotenvy::dotenv().ok();
        });
    }

    fn client() -> LettermintClient {
        load_env();
        let token =
            std::env::var("LETTERMINT_API_TOKEN").expect("LETTERMINT_API_TOKEN must be set");
        LettermintClient::builder().api_token(token).build()
    }

    fn sender() -> String {
        load_env();
        std::env::var("LETTERMINT_SENDER").expect("LETTERMINT_SENDER must be set")
    }

    fn format_api_error(err: &QueryError<LettermintClientError>) -> String {
        match err {
            QueryError::Validation {
                message, errors, ..
            } => {
                let mut msg = "Validation error".to_string();
                if let Some(m) = message {
                    msg.push_str(&format!(": {m}"));
                }
                if let Some(errs) = errors {
                    for (field, msgs) in errs {
                        for m in msgs {
                            msg.push_str(&format!("\n  {field}: {m}"));
                        }
                    }
                }
                msg
            }
            QueryError::Authentication { message, .. } => {
                format!("Authentication error: {message:?}")
            }
            QueryError::RateLimit { message, .. } => {
                format!("Rate limit: {message:?}")
            }
            QueryError::Api {
                status, message, ..
            } => {
                let mut msg = format!("API {status}");
                if let Some(m) = message {
                    msg.push_str(&format!(": {m}"));
                }
                msg
            }
            other => format!("{other}"),
        }
    }

    // -- ping ---------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn ping_ok() -> Result {
        let resp = PingRequest
            .execute(&client())
            .await
            .map_err(|e| format_api_error(&e))?;

        assert_eq!(resp.message, "pong");
        Ok(())
    }

    // -- send ---------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn send_text_email_ok() -> Result {
        let resp = SendEmailRequest::builder()
            .from(sender())
            .to(vec![Scenario::Ok.email()])
            .subject("E2E test: text")
            .text("This is a plain text e2e test.")
            .build()
            .execute(&client())
            .await
            .map_err(|e| format_api_error(&e))?;

        assert!(!resp.message_id.is_empty());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn send_html_email_ok() -> Result {
        let resp = SendEmailRequest::builder()
            .from(sender())
            .to(vec![Scenario::Ok.email()])
            .subject("E2E test: html")
            .html("<h1>Hello</h1><p>HTML e2e test.</p>")
            .build()
            .execute(&client())
            .await
            .map_err(|e| format_api_error(&e))?;

        assert!(!resp.message_id.is_empty());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn send_html_and_text_email_ok() -> Result {
        let resp = SendEmailRequest::builder()
            .from(sender())
            .to(vec![Scenario::Ok.email()])
            .subject("E2E test: html+text")
            .html("<h1>Hello</h1>")
            .text("Hello")
            .build()
            .execute(&client())
            .await
            .map_err(|e| format_api_error(&e))?;

        assert!(!resp.message_id.is_empty());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn send_with_all_options() -> Result {
        let from = sender();

        let resp = SendEmailRequest::builder()
            .from(from.clone())
            .to(vec![Scenario::Ok.email()])
            .subject("E2E test: full options")
            .html("<h1>Full test</h1>")
            .text("Full test")
            .cc(vec![emails::custom("ok+cc")])
            .reply_to(vec![from])
            .tag("e2e-test")
            .metadata(HashMap::from([("test".into(), "true".into())]))
            .idempotency_key(format!(
                "e2e-test-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            ))
            .build()
            .execute(&client())
            .await
            .map_err(|e| format_api_error(&e))?;

        assert!(!resp.message_id.is_empty());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn send_with_attachment() -> Result {
        use base64::Engine;
        let content = base64::engine::general_purpose::STANDARD.encode(b"Hello from e2e test");

        let resp = SendEmailRequest::builder()
            .from(sender())
            .to(vec![Scenario::Ok.email()])
            .subject("E2E test: attachment")
            .text("See attached file.")
            .attachments(vec![
                Attachment::builder()
                    .filename("test.txt")
                    .content(content)
                    .build(),
            ])
            .build()
            .execute(&client())
            .await
            .map_err(|e| format_api_error(&e))?;

        assert!(!resp.message_id.is_empty());
        Ok(())
    }

    // -- bounce scenarios ---------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn send_to_soft_bounce() -> Result {
        let resp = SendEmailRequest::builder()
            .from(sender())
            .to(vec![Scenario::SoftBounce.email()])
            .subject("E2E test: soft bounce")
            .text("This should soft bounce.")
            .build()
            .execute(&client())
            .await
            .map_err(|e| format_api_error(&e))?;

        assert!(!resp.message_id.is_empty());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn send_to_random_soft_bounce() -> Result {
        let resp = SendEmailRequest::builder()
            .from(sender())
            .to(vec![Scenario::SoftBounce.random()])
            .subject("E2E test: random soft bounce")
            .text("This should soft bounce with a unique address.")
            .build()
            .execute(&client())
            .await
            .map_err(|e| format_api_error(&e))?;

        assert!(!resp.message_id.is_empty());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn send_to_hard_bounce() -> Result {
        let resp = SendEmailRequest::builder()
            .from(sender())
            .to(vec![Scenario::HardBounce.email()])
            .subject("E2E test: hard bounce")
            .text("This should hard bounce.")
            .build()
            .execute(&client())
            .await
            .map_err(|e| format_api_error(&e))?;

        assert!(!resp.message_id.is_empty());
        Ok(())
    }

    // -- batch --------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn batch_send_ok() -> Result {
        let from = sender();

        let batch = BatchSendRequest::builder()
            .emails(vec![
                SendEmailRequest::builder()
                    .from(from.clone())
                    .to(vec![Scenario::Ok.email()])
                    .subject("E2E test: batch 1/2")
                    .text("First email in batch.")
                    .build(),
                SendEmailRequest::builder()
                    .from(from)
                    .to(vec![Scenario::Ok.email()])
                    .subject("E2E test: batch 2/2")
                    .text("Second email in batch.")
                    .build(),
            ])
            .build()
            .expect("batch should be valid");

        let responses = batch
            .execute(&client())
            .await
            .map_err(|e| format_api_error(&e))?;

        assert_eq!(responses.len(), 2);
        for resp in &responses {
            assert!(!resp.message_id.is_empty());
        }
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn batch_send_with_idempotency_key() -> Result {
        let from = sender();
        let key = format!(
            "batch-e2e-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        let batch = BatchSendRequest::builder()
            .emails(vec![
                SendEmailRequest::builder()
                    .from(from)
                    .to(vec![Scenario::Ok.email()])
                    .subject("E2E test: batch idempotency")
                    .text("Batch with idempotency key.")
                    .build(),
            ])
            .idempotency_key(key)
            .build()
            .expect("batch should be valid");

        let responses = batch
            .execute(&client())
            .await
            .map_err(|e| format_api_error(&e))?;

        assert_eq!(responses.len(), 1);
        assert!(!responses[0].message_id.is_empty());
        Ok(())
    }

    // -- error scenarios ----------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn send_from_unverified_domain_returns_validation_error() -> Result {
        let err = SendEmailRequest::builder()
            .from("test@unverified-domain-that-does-not-exist.example")
            .to(vec![Scenario::Ok.email()])
            .subject("E2E test: unverified domain")
            .text("This should fail with a validation error.")
            .build()
            .execute(&client())
            .await
            .expect_err("should fail with unverified domain");

        match &err {
            QueryError::Validation { errors, .. } => {
                assert!(errors.is_some(), "expected per-field validation errors");
                let errs = errors.as_ref().unwrap();
                assert!(errs.contains_key("from"), "expected error on 'from' field");
            }
            _ => return Err(format!("expected Validation error, got: {err:?}").into()),
        }
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn invalid_token_returns_authentication_error() -> Result {
        let bad_client = LettermintClient::builder()
            .api_token("this-token-does-not-exist")
            .build();

        let err = SendEmailRequest::builder()
            .from("test@example.com")
            .to(vec![Scenario::Ok.email()])
            .subject("E2E test: bad token")
            .text("This should fail with an authentication error.")
            .build()
            .execute(&bad_client)
            .await
            .expect_err("should fail with invalid token");

        assert!(
            matches!(err, QueryError::Authentication { .. }),
            "expected Authentication error, got: {err:?}"
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Mock tests (wiremock) — things that can't be verified against the real API
// ---------------------------------------------------------------------------

mod mock {
    use std::collections::HashMap;

    use lettermint_rs::api::email::*;
    use lettermint_rs::api::ping::PingRequest;
    use lettermint_rs::reqwest::LettermintClient;
    use lettermint_rs::{Query, QueryError};
    use serde_json::json;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn mock_client(server: &MockServer) -> LettermintClient {
        LettermintClient::builder()
            .api_token("test-token")
            .base_url(server.uri())
            .build()
    }

    fn ok_send_response() -> serde_json::Value {
        json!({ "message_id": "msg-123", "status": "queued" })
    }

    fn minimal_email() -> SendEmailRequest {
        SendEmailRequest::builder()
            .from("sender@example.com")
            .to(vec!["recipient@example.com".into()])
            .subject("Test")
            .text("Hello")
            .build()
    }

    // -- header contract ----------------------------------------------------

    #[tokio::test]
    async fn auth_header_is_set() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/send"))
            .and(header("x-lettermint-token", "test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_send_response()))
            .expect(1)
            .mount(&server)
            .await;

        minimal_email()
            .execute(&mock_client(&server))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn content_type_and_accept_on_post() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/send"))
            .and(header("content-type", "application/json"))
            .and(header("accept", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_send_response()))
            .expect(1)
            .mount(&server)
            .await;

        minimal_email()
            .execute(&mock_client(&server))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn accept_on_get_no_content_type() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ping"))
            .and(header("accept", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_string("pong"))
            .expect(1)
            .mount(&server)
            .await;

        PingRequest.execute(&mock_client(&server)).await.unwrap();

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(
            requests[0].headers.get("content-type").is_none(),
            "GET request should not have Content-Type header"
        );
    }

    #[tokio::test]
    async fn user_agent_header_is_set() {
        let expected_ua = format!("Lettermint/{} (Rust)", env!("CARGO_PKG_VERSION"));

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_send_response()))
            .expect(1)
            .mount(&server)
            .await;

        minimal_email()
            .execute(&mock_client(&server))
            .await
            .unwrap();

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let ua = requests[0]
            .headers
            .get("user-agent")
            .expect("User-Agent header should be present")
            .to_str()
            .unwrap();
        assert_eq!(ua, expected_ua);
    }

    #[tokio::test]
    async fn idempotency_key_sent_as_header() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/send"))
            .and(header("Idempotency-Key", "unique-123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_send_response()))
            .expect(1)
            .mount(&server)
            .await;

        SendEmailRequest::builder()
            .from("sender@example.com")
            .to(vec!["recipient@example.com".into()])
            .subject("Test")
            .text("Hello")
            .idempotency_key("unique-123")
            .build()
            .execute(&mock_client(&server))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn batch_idempotency_key_sent_as_header() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/send/batch"))
            .and(header("Idempotency-Key", "batch-key-456"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!([{ "message_id": "msg-1", "status": "queued" }])),
            )
            .expect(1)
            .mount(&server)
            .await;

        BatchSendRequest::builder()
            .emails(vec![minimal_email()])
            .idempotency_key("batch-key-456")
            .build()
            .unwrap()
            .execute(&mock_client(&server))
            .await
            .unwrap();
    }

    // -- body contract ------------------------------------------------------

    #[tokio::test]
    async fn minimal_payload_has_no_optional_fields() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/send"))
            .and(body_json(json!({
                "from": "sender@example.com",
                "to": ["recipient@example.com"],
                "subject": "Test",
                "text": "Hello"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_send_response()))
            .expect(1)
            .mount(&server)
            .await;

        minimal_email()
            .execute(&mock_client(&server))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn full_payload_includes_all_fields() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/send"))
            .and(body_json(json!({
                "from": "John Doe <john@example.com>",
                "to": ["alice@example.com", "bob@example.com"],
                "subject": "Newsletter",
                "html": "<h1>News</h1>",
                "text": "News",
                "cc": ["cc@example.com"],
                "bcc": ["bcc@example.com"],
                "reply_to": ["reply@example.com"],
                "headers": { "X-Custom": "value" },
                "attachments": [
                    { "filename": "report.pdf", "content": "base64data" },
                    { "filename": "logo.png", "content": "base64logo", "content_id": "logo" }
                ],
                "route": "my-route",
                "metadata": { "campaign": "spring" },
                "tag": "newsletter"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_send_response()))
            .expect(1)
            .mount(&server)
            .await;

        SendEmailRequest::builder()
            .from("John Doe <john@example.com>")
            .to(vec!["alice@example.com".into(), "bob@example.com".into()])
            .subject("Newsletter")
            .html("<h1>News</h1>")
            .text("News")
            .cc(vec!["cc@example.com".into()])
            .bcc(vec!["bcc@example.com".into()])
            .reply_to(vec!["reply@example.com".into()])
            .headers(HashMap::from([("X-Custom".into(), "value".into())]))
            .attachments(vec![
                Attachment::builder()
                    .filename("report.pdf")
                    .content("base64data")
                    .build(),
                Attachment::builder()
                    .filename("logo.png")
                    .content("base64logo")
                    .content_id("logo")
                    .build(),
            ])
            .route("my-route")
            .metadata(HashMap::from([("campaign".into(), "spring".into())]))
            .tag("newsletter")
            .idempotency_key("idem-key")
            .build()
            .execute(&mock_client(&server))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn idempotency_key_excluded_from_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_send_response()))
            .expect(1)
            .mount(&server)
            .await;

        SendEmailRequest::builder()
            .from("sender@example.com")
            .to(vec!["recipient@example.com".into()])
            .subject("Test")
            .text("Hello")
            .idempotency_key("secret-key")
            .build()
            .execute(&mock_client(&server))
            .await
            .unwrap();

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert!(
            body.get("idempotency_key").is_none(),
            "idempotency_key must not appear in the request body"
        );
    }

    #[tokio::test]
    async fn batch_serializes_as_array() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/send/batch"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                { "message_id": "msg-1", "status": "queued" },
                { "message_id": "msg-2", "status": "queued" },
            ])))
            .expect(1)
            .mount(&server)
            .await;

        let email_a = SendEmailRequest::builder()
            .from("sender@example.com")
            .to(vec!["alice@example.com".into()])
            .subject("Hello Alice")
            .text("Hi Alice!")
            .build();
        let email_b = SendEmailRequest::builder()
            .from("sender@example.com")
            .to(vec!["bob@example.com".into()])
            .subject("Hello Bob")
            .text("Hi Bob!")
            .build();

        BatchSendRequest::builder()
            .emails(vec![email_a, email_b])
            .build()
            .unwrap()
            .execute(&mock_client(&server))
            .await
            .unwrap();

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        let arr = body.as_array().expect("batch body must be a JSON array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["to"], json!(["alice@example.com"]));
        assert_eq!(arr[1]["to"], json!(["bob@example.com"]));
    }

    // -- custom client configuration ----------------------------------------

    #[tokio::test]
    async fn custom_base_url_is_used() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_send_response()))
            .expect(1)
            .mount(&server)
            .await;

        // Trailing-slash variant should work too.
        let client = LettermintClient::builder()
            .api_token("test-token")
            .base_url(format!("{}/", server.uri()))
            .build();
        let resp = minimal_email().execute(&client).await.unwrap();

        assert_eq!(resp.message_id, "msg-123");
    }

    // -- untriggerable error scenarios --------------------------------------

    #[tokio::test]
    async fn error_403_is_authentication() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/send"))
            .respond_with(
                ResponseTemplate::new(403).set_body_json(json!({ "message": "Access denied" })),
            )
            .mount(&server)
            .await;

        let err = minimal_email()
            .execute(&mock_client(&server))
            .await
            .expect_err("should fail with 403");

        assert!(
            matches!(err, QueryError::Authentication { .. }),
            "expected Authentication error, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn error_429_is_rate_limit() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/send"))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_json(json!({ "message": "Rate limit exceeded" })),
            )
            .mount(&server)
            .await;

        let err = minimal_email()
            .execute(&mock_client(&server))
            .await
            .expect_err("should fail with 429");

        match err {
            QueryError::RateLimit { message, .. } => {
                assert_eq!(message.as_deref(), Some("Rate limit exceeded"));
            }
            other => panic!("expected RateLimit error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn error_500_is_api_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/send"))
            .respond_with(ResponseTemplate::new(500).set_body_json(
                json!({ "error_type": "InternalError", "message": "Something broke" }),
            ))
            .mount(&server)
            .await;

        let err = minimal_email()
            .execute(&mock_client(&server))
            .await
            .expect_err("should fail with 500");

        match err {
            QueryError::Api {
                status,
                error_type,
                message,
                ..
            } => {
                assert_eq!(status, http::StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(error_type.as_deref(), Some("InternalError"));
                assert_eq!(message.as_deref(), Some("Something broke"));
            }
            other => panic!("expected Api error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn non_json_error_body_handled() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/send"))
            .respond_with(ResponseTemplate::new(502).set_body_string("gateway timeout"))
            .mount(&server)
            .await;

        let err = minimal_email()
            .execute(&mock_client(&server))
            .await
            .expect_err("should fail with 502");

        match err {
            QueryError::Api {
                status,
                error_type,
                message,
                body,
                ..
            } => {
                assert_eq!(status, http::StatusCode::BAD_GATEWAY);
                assert_eq!(error_type, None);
                assert_eq!(message, None);
                assert_eq!(body.as_ref(), b"gateway timeout");
            }
            other => panic!("expected Api error, got: {other:?}"),
        }
    }
}
