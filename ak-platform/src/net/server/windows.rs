use std::{error::Error, io};
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::mpsc;
use tokio_stream::Stream;

use super::{ListenerStream, SocketPermMode};

pub struct NamedPipeListenerStream {
    rx: mpsc::Receiver<io::Result<NamedPipeServer>>,
}

impl NamedPipeListenerStream {
    fn new(path: String, _perm: SocketPermMode) -> io::Result<Self> {
        // TODO: apply _perm as an SDDL security descriptor via windows-sys,
        // mirroring the Go implementation in socket_windows.go.
        let server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(&path)?;

        let (tx, rx) = mpsc::channel(1);

        tokio::spawn(async move {
            let mut current = server;
            loop {
                if let Err(e) = current.connect().await {
                    let _ = tx.send(Err(e)).await;
                    return;
                }
                // Create the next instance before handing off the current one so
                // there is no window where new connections would be rejected.
                let next = match ServerOptions::new().create(&path) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        return;
                    }
                };
                let accepted = std::mem::replace(&mut current, next);
                if tx.send(Ok(accepted)).await.is_err() {
                    return;
                }
            }
        });

        Ok(Self { rx })
    }
}

impl Stream for NamedPipeListenerStream {
    type Item = io::Result<NamedPipeServer>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

pub async fn listen(
    path: String,
    perm: SocketPermMode,
) -> Result<ListenerStream, Box<dyn Error + Send + Sync>> {
    let stream = NamedPipeListenerStream::new(path, perm)?;
    Ok(ListenerStream::Windows(stream))
}

#[cfg(test)]
mod tests {
    use tokio::net::windows::named_pipe::ClientOptions;

    use super::{listen, SocketPermMode};

    #[tokio::test]
    async fn listen_creates_pipe() {
        let path = r"\\.\pipe\ak-test-server-creates".to_string();
        let _listener = listen(path.clone(), SocketPermMode::Owner).await.unwrap();
        // If the pipe exists, a client can open it immediately.
        ClientOptions::new().open(&path).unwrap();
    }
}
