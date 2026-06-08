use std::error::Error;

use crate::{
    models::{Message, Response},
    path_handler::PathHandler,
};

impl PathHandler {
    pub async fn handle_ping(&self, msg: Message) -> Result<Response, Box<dyn Error>> {
        let mut res = Response::in_response_to(msg);
        res.data.insert(
            "ping".to_owned(),
            serde_json::Value::String("pong".to_owned()),
        );
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use hyper_util::rt::TokioIo;
    use tokio::io::{DuplexStream, duplex};
    use tonic::transport::{Channel, Endpoint};
    use tower::service_fn;

    use super::*;

    async fn mock_channel() -> Channel {
        let (client_io, _server_io) = duplex(1024);
        let mut client_io = Some(client_io);
        Endpoint::try_from("http://[::]:50051")
            .unwrap()
            .connect_with_connector(service_fn(move |_| {
                let io = client_io.take().unwrap();
                async move { Ok::<TokioIo<DuplexStream>, Infallible>(TokioIo::new(io)) }
            }))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_ping() {
        let handler = PathHandler {
            system_channel: mock_channel().await,
            user_channel: None,
        };
        let res = handler.handle_ping(Message::test_msg()).await.unwrap();
        assert_eq!(res.data.get("ping").unwrap().as_str().unwrap(), "pong");
    }
}
