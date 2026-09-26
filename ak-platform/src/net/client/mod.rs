use hyper_util::rt::TokioIo;
use interprocess::local_socket::{
    GenericFilePath,
    tokio::{Stream as LocalSocketStream, prelude::*},
};

use crate::string::PlatformString;

pub async fn connect(path: PlatformString) -> std::io::Result<TokioIo<LocalSocketStream>> {
    Ok(TokioIo::new(connect_stream(path).await?))
}

/// Like [`connect`], but returns the raw local socket stream instead of a
/// hyper-oriented `TokioIo` wrapper, for callers that need a plain
/// `tokio::io::AsyncRead`/`AsyncWrite` (e.g. proxying a non-HTTP protocol).
pub async fn connect_stream(path: PlatformString) -> std::io::Result<LocalSocketStream> {
    let name = path.for_current().to_fs_name::<GenericFilePath>()?;
    LocalSocketStream::connect(name).await
}
