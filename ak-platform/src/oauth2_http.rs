//! Adapts a workspace `reqwest::Client` to oauth2's `AsyncHttpClient`.
//!
//! oauth2's own reqwest integration (behind its `reqwest` default feature) vendors
//! its own pinned reqwest version, which duplicates and can drift from the
//! workspace's reqwest version. This adapter relies on oauth2's blanket impl of
//! `AsyncHttpClient` for `Fn(HttpRequest) -> Future<Output = Result<HttpResponse, E>>`
//! so callers can keep using the workspace's reqwest client instead.

use std::future::Future;
use std::pin::Pin;

type HttpFuture = Pin<
    Box<
        dyn Future<Output = Result<oauth2::HttpResponse, oauth2::HttpClientError<reqwest::Error>>>
            + Send,
    >,
>;

/// Builds an `AsyncHttpClient` for oauth2 backed by the given `reqwest::Client`.
pub fn adapter(client: reqwest::Client) -> impl Fn(oauth2::HttpRequest) -> HttpFuture {
    move |request| {
        let client = client.clone();
        Box::pin(async move {
            let mut builder = client.request(request.method().clone(), request.uri().to_string());
            for (name, value) in request.headers() {
                builder = builder.header(name.clone(), value.clone());
            }
            let response = builder
                .body(request.into_body())
                .send()
                .await
                .map_err(|e| oauth2::HttpClientError::Reqwest(Box::new(e)))?;

            let mut resp_builder = oauth2::http::Response::builder().status(response.status());
            for (name, value) in response.headers() {
                resp_builder = resp_builder.header(name.clone(), value.clone());
            }
            let body = response
                .bytes()
                .await
                .map_err(|e| oauth2::HttpClientError::Reqwest(Box::new(e)))?
                .to_vec();
            resp_builder
                .body(body)
                .map_err(oauth2::HttpClientError::Http)
        })
    }
}
