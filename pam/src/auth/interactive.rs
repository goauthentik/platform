use authentik_sys::generated::{
    grpc_request,
    pam::{InteractiveResponse, pam_client::PamClient},
};
use pam::{constants::PamResultCode, conv::Conv};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Request;

pub fn auth_interactive(_username: String, _password: &str, _conv: &Conv<'_>) -> PamResultCode {
    match grpc_request(async |ch| {
        let (tx, rx) = mpsc::channel(128);
        let request_stream = ReceiverStream::new(rx);

        // Start the bidirectional stream
        let response = PamClient::new(ch)
            .interactive_auth(Request::new(request_stream))
            .await?;
        let mut response_stream = response.into_inner();

        // Spawn a task to send requests
        let sender_handle = tokio::spawn(async move {
            for i in 0..10 {
                let request = InteractiveResponse {
                    init: false,
                    values: vec![],
                };

                if tx.send(request).await.is_err() {
                    break;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        // Handle incoming responses
        loop {
            let challenge = match response_stream.message().await {
                Ok(m) => match m {
                    Some(m) => m,
                    None => break,
                },
                Err(e) => {
                    log::warn!("failed to get challenge: {e}");
                    return Err(Box::from(e));
                }
            };
            println!("Received: {:?}", challenge);
        }

        // Wait for sender to complete
        sender_handle.await?;
        Ok(())
    }) {
        Ok(_) => PamResultCode::PAM_SUCCESS,
        Err(e) => {
            log::warn!("failed to authenticate: {e}");
            PamResultCode::PAM_AUTH_ERR
        }
    }
}
