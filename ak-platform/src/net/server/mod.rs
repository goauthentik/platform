use std::{io, pin::Pin, task::{Context, Poll}};

use tokio_stream::Stream;

#[cfg(unix)]
use tokio::net::UnixStream;
#[cfg(unix)]
use tokio_stream::wrappers::UnixListenerStream;
#[cfg(windows)]
use tokio::net::windows::named_pipe::NamedPipeServer;

use crate::platform::string::PlatformString;

#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

pub enum SocketPermMode {
    Owner,
    Everyone,
    Admin,
}

pub enum ListenerStream {
    #[cfg(unix)]
    Unix(UnixListenerStream),
    #[cfg(windows)]
    Windows(windows::NamedPipeListenerStream),
}

#[cfg(unix)]
impl Stream for ListenerStream {
    type Item = io::Result<UnixStream>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            ListenerStream::Unix(s) => Pin::new(s).poll_next(cx),
        }
    }
}

#[cfg(windows)]
impl Stream for ListenerStream {
    type Item = io::Result<NamedPipeServer>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            ListenerStream::Windows(s) => Pin::new(s).poll_next(cx),
        }
    }
}

pub async fn listen(
    path: PlatformString,
    perm: SocketPermMode,
) -> Result<ListenerStream, Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(unix)]
    return unix::listen(path.for_current(), perm).await;
    #[cfg(windows)]
    return windows::listen(path.for_current(), perm).await;
}
