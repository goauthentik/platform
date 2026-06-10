use std::error::Error;

use hyper_util::rt::TokioIo;
use tokio::net::UnixStream;

use crate::net::client::StreamType;

pub async fn connect(path: String) -> Result<TokioIo<StreamType>, Box<dyn Error + Send + Sync>> {
    let client = match UnixStream::connect(path).await {
        Ok(c) => c,
        Err(e) => {
            return Err(e.into());
        }
    };
    Ok(TokioIo::new(StreamType::Unix(client)))
}
