use eyre::Error;
use reqwest::Response;

pub trait ReqwestResponseError {
    fn error_with_response(self) -> impl std::future::Future<Output = Result<Self, Error>> + Send
    where
        Self: Sized;
}

impl ReqwestResponseError for Response {
    async fn error_with_response(self) -> Result<Self, Error> {
        let status = self.status();
        if status.is_client_error() || status.is_server_error() {
            let body = self.text().await?;
            Err(eyre::eyre!("Error in HTTP request: {} {}", status, body))
        } else {
            Ok(self)
        }
    }
}
