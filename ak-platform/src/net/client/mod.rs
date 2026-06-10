use std::error::Error;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper_util::rt::TokioIo;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[cfg(unix)]
use tokio::net::UnixStream;
#[cfg(windows)]
use tokio::net::windows::named_pipe::NamedPipeClient;

use crate::platform::string::PlatformString;

#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

pub enum StreamType {
    #[cfg(unix)]
    Unix(UnixStream),
    #[cfg(windows)]
    Windows(NamedPipeClient),
}

impl AsyncRead for StreamType {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            #[cfg(unix)]
            StreamType::Unix(s) => Pin::new(s).poll_read(cx, buf),
            #[cfg(windows)]
            StreamType::Windows(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for StreamType {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            #[cfg(unix)]
            StreamType::Unix(s) => Pin::new(s).poll_write(cx, buf),
            #[cfg(windows)]
            StreamType::Windows(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            #[cfg(unix)]
            StreamType::Unix(s) => Pin::new(s).poll_flush(cx),
            #[cfg(windows)]
            StreamType::Windows(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            #[cfg(unix)]
            StreamType::Unix(s) => Pin::new(s).poll_shutdown(cx),
            #[cfg(windows)]
            StreamType::Windows(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

pub async fn connect(path: PlatformString) -> Result<TokioIo<StreamType>, Box<dyn Error + Send + Sync>> {
    #[cfg(unix)]
    return unix::connect(path.for_current()).await;
    #[cfg(windows)]
    return windows::connect(path.for_current()).await;
}
