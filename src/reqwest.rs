use std::time::Duration;

use crate::{Client, Endpoint, LETTERMINT_API_URL, Query, QueryError};
use bon::Builder;
use bytes::Bytes;
use http::{Request, Response};
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const USER_AGENT: &str = concat!("Lettermint/", env!("CARGO_PKG_VERSION"), " (Rust)");

/// A reqwest-based Lettermint API client.
///
/// ```
/// # use lettermint::reqwest::LettermintClient;
/// let client = LettermintClient::builder()
///     .api_token("your-api-token")
///     .build();
/// ```
///
/// With a custom base URL:
/// ```
/// # use lettermint::reqwest::LettermintClient;
/// let client = LettermintClient::builder()
///     .api_token("your-api-token")
///     .base_url("https://custom.api/v1/")
///     .build();
/// ```
///
/// With a pre-configured reqwest client:
/// ```
/// # use lettermint::reqwest::LettermintClient;
/// let http_client = reqwest::Client::builder()
///     .timeout(std::time::Duration::from_secs(60))
///     .build()
///     .unwrap();
/// let client = LettermintClient::builder()
///     .api_token("your-api-token")
///     .client(http_client)
///     .build();
/// ```
#[derive(Clone, Builder)]
pub struct LettermintClient {
    #[builder(into)]
    api_token: SecretString,
    #[builder(into, default = String::from(LETTERMINT_API_URL))]
    base_url: String,
    #[builder(default = default_reqwest_client())]
    client: ::reqwest::Client,
}

fn default_reqwest_client() -> ::reqwest::Client {
    ::reqwest::Client::builder()
        .timeout(DEFAULT_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .expect("default reqwest client should build")
}

impl LettermintClient {
    /// Execute an endpoint request and return the deserialized response.
    ///
    /// # Errors
    ///
    /// Returns [`QueryError`] if the request fails due to network, authentication,
    /// validation, rate limiting, or other API errors.
    pub async fn execute_endpoint<T>(
        &self,
        request: T,
    ) -> Result<T::Response, QueryError<LettermintClientError>>
    where
        T: Endpoint + Send + Sync,
    {
        request.execute(self).await
    }
}

impl std::fmt::Debug for LettermintClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LettermintClient")
            .field("api_token", &"***")
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

#[derive(Error, Debug)]
pub enum LettermintClientError {
    #[error("error setting auth header: {}", source)]
    AuthError {
        #[from]
        source: http::header::InvalidHeaderValue,
    },
    #[error("communication with lettermint: {}", source)]
    Communication {
        #[from]
        source: ::reqwest::Error,
    },
    #[error("http error: {}", source)]
    Http {
        #[from]
        source: http::Error,
    },
    #[error("invalid uri: {}", source)]
    InvalidUri {
        #[from]
        source: http::uri::InvalidUri,
    },
}

impl Client for LettermintClient {
    type Error = LettermintClientError;

    #[tracing::instrument(name = "lettermint.http", skip_all, fields(url))]
    async fn execute(&self, mut req: Request<Bytes>) -> Result<Response<Bytes>, Self::Error> {
        req.headers_mut().append(
            "x-lettermint-token",
            self.api_token.expose_secret().try_into()?,
        );

        // Build URL by joining base_url and the endpoint path, avoiding Url::join
        // pitfalls with leading slashes and missing trailing slashes.
        let path = req
            .uri()
            .path_and_query()
            .map_or("", http::uri::PathAndQuery::as_str);
        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        tracing::Span::current().record("url", &url);

        *req.uri_mut() = url.parse()?;

        let reqwest_req: ::reqwest::Request = req.try_into()?;
        let reqwest_rsp = self.client.execute(reqwest_req).await?;

        tracing::debug!(status = reqwest_rsp.status().as_u16(), "HTTP response");

        let mut rsp = Response::builder()
            .status(reqwest_rsp.status())
            .version(reqwest_rsp.version());

        let headers = rsp
            .headers_mut()
            .expect("response builder should have headers");
        for (k, v) in reqwest_rsp.headers() {
            headers.insert(k, v.clone());
        }

        Ok(rsp.body(reqwest_rsp.bytes().await?)?)
    }
}
