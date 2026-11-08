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
///
/// On Windows this connects at identification level, so the server can tell who we
/// are but can't act as us. Pipe names are global, so whoever created a pipe first
/// would otherwise be able to impersonate everyone connecting to it, SYSTEM included.
pub async fn connect_stream(path: PlatformString) -> std::io::Result<LocalSocketStream> {
    let path = path.for_current();
    // Also rejects anything that isn't a pipe path on Windows, which CreateFileW
    // would otherwise happily open as a file.
    let name = path.as_str().to_fs_name::<GenericFilePath>()?;
    #[cfg(windows)]
    {
        let _ = name;
        crate::net::win_pipe::connect(&path).await
    }
    #[cfg(not(windows))]
    LocalSocketStream::connect(name).await
}
