use hyper_util::rt::TokioIo;
use interprocess::local_socket::tokio::Stream as LocalSocketStream;
#[cfg(not(windows))]
use interprocess::local_socket::{GenericFilePath, tokio::prelude::*};

use crate::string::PlatformString;

pub async fn connect(path: PlatformString) -> std::io::Result<TokioIo<LocalSocketStream>> {
    Ok(TokioIo::new(connect_stream(path).await?))
}

/// Like [`connect`], but returns the raw local socket stream instead of a
/// hyper-oriented `TokioIo` wrapper, for callers that need a plain
/// `tokio::io::AsyncRead`/`AsyncWrite` (e.g. proxying a non-HTTP protocol).
///
/// On Windows this connects at identification level, so the server can tell who we
/// are but can't act as us. Pipe names are global, so whoever created a pipe first
/// would otherwise be able to impersonate everyone connecting to it, SYSTEM included.
pub async fn connect_stream(path: PlatformString) -> std::io::Result<LocalSocketStream> {
    #[cfg(windows)]
    return crate::net::win_pipe::connect(&path.for_current()).await;
    #[cfg(not(windows))]
    {
        let name = path.for_current().to_fs_name::<GenericFilePath>()?;
        LocalSocketStream::connect(name).await
    }
}
