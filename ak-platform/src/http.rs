use eyre::Error;
use reqwest::{Response, header::HeaderMap};
use serde_json::Value;

/// Header in which authentik returns the ID it logs each request under.
pub const REQUEST_ID_HEADER: &str = "X-authentik-id";

pub trait ReqwestResponseError {
    fn error_with_response(self) -> impl std::future::Future<Output = Result<Self, Error>> + Send
    where
        Self: Sized;
}

impl ReqwestResponseError for Response {
    async fn error_with_response(self) -> Result<Self, Error> {
        let status = self.status();
        if status.is_client_error() || status.is_server_error() {
            let headers = self.headers().clone();
            let body = self.text().await?;
            Err(eyre::eyre!(
                "Error in HTTP request: {} {}",
                status,
                error_message(&headers, &body)
            ))
        } else {
            Ok(self)
        }
    }
}

/// Describe an error response for users.
///
/// OAuth errors (`error` and `error_description`) and API errors (`detail`) are shown as text
/// instead of raw JSON. If the response came from authentik, its request ID is added, so users
/// can pass it to their administrator, who can find the request in authentik's logs. Responses
/// from other servers are returned as they are.
pub fn error_message(headers: &HeaderMap, body: &str) -> String {
    let json = serde_json::from_str::<Value>(body).ok();
    let field = |name: &str| {
        json.as_ref()
            .and_then(|j| j.get(name))
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
    };
    let mut message = match (
        field("error"),
        field("error_description").or(field("detail")),
    ) {
        (Some(error), Some(description)) => format!("{error}: {description}"),
        (Some(error), None) => error.to_owned(),
        (None, Some(description)) => description.to_owned(),
        (None, None) => body.trim().to_owned(),
    };
    let request_id = field("request_id").or_else(|| {
        headers
            .get(REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty())
    });
    if let Some(id) = request_id {
        message.push_str(&format!(
            " (request ID {id}: give this to your authentik administrator)"
        ));
    }
    message
}

#[cfg(test)]
mod tests {
    use reqwest::header::HeaderValue;

    use super::*;

    #[test]
    fn oauth_error_with_request_id() {
        let body = r#"{"error": "invalid_dpop_proof", "error_description": "DPoP proof iat outside acceptable clock skew", "request_id": "abc123"}"#;
        assert_eq!(
            error_message(&HeaderMap::new(), body),
            "invalid_dpop_proof: DPoP proof iat outside acceptable clock skew \
             (request ID abc123: give this to your authentik administrator)"
        );
    }

    #[test]
    fn request_id_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert(REQUEST_ID_HEADER, HeaderValue::from_static("def456"));
        assert_eq!(
            error_message(&headers, r#"{"detail": "Not found."}"#),
            "Not found. (request ID def456: give this to your authentik administrator)"
        );
    }

    #[test]
    fn other_servers_unchanged() {
        assert_eq!(
            error_message(&HeaderMap::new(), "upstream unavailable\n"),
            "upstream unavailable"
        );
        assert_eq!(
            error_message(&HeaderMap::new(), r#"{"errors": ["permission denied"]}"#),
            r#"{"errors": ["permission denied"]}"#
        );
    }
}
