use crate::Endpoint;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Ping the Lettermint API to verify connectivity and credentials.
///
/// ```
/// # use lettermint::api::ping::PingRequest;
/// let req = PingRequest;
/// // resp will be PingResponse { status: 200 }
/// ```
pub struct PingRequest;

#[doc(hidden)]
#[derive(Debug, Serialize)]
pub struct EmptyBody;

/// Response from the ping endpoint.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct PingResponse {
    pub status: u16,
}

impl Endpoint for PingRequest {
    type Request = EmptyBody;
    type Response = PingResponse;

    fn endpoint(&self) -> Cow<'static, str> {
        "ping".into()
    }

    fn body(&self) -> &Self::Request {
        static BODY: EmptyBody = EmptyBody;
        &BODY
    }

    fn method(&self) -> http::Method {
        http::Method::GET
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_path_and_method() {
        let req = PingRequest;
        assert_eq!(req.endpoint(), "ping");
        assert_eq!(req.method(), http::Method::GET);
    }

    #[test]
    fn deserialize_response() {
        let resp: PingResponse = serde_json::from_str("200").unwrap();
        assert_eq!(resp.status, 200);
    }
}
