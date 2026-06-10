use std::error::Error;
use std::time::Duration;

use hyper_util::rt::TokioIo;
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::time;

use crate::net::client::StreamType;

pub async fn connect(path: String) -> Result<TokioIo<StreamType>, Box<dyn Error + Send + Sync>> {
    let client = loop {
        match ClientOptions::new().open(&path) {
            Ok(client) => break client,
            Err(e) if e.raw_os_error() == Some(231) => (),
            Err(e) => return Err(e.into()),
        }

        time::sleep(Duration::from_millis(50)).await;
    };

    Ok(TokioIo::new(StreamType::Windows(client)))
}
