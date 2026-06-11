pub mod client;
pub mod server;

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_stream::StreamExt;

    use crate::net::server::SocketPermMode;
    use crate::net::{client, server};
    use crate::string::PlatformString;

    #[cfg(unix)]
    #[tokio::test]
    async fn unix_roundtrip() {
        let path = PlatformString::new_with_default("/tmp/ak-test-net-roundtrip.sock");

        let mut listener = server::listen(path.clone(), SocketPermMode::Everyone)
            .await
            .unwrap();

        let server_task = tokio::spawn(async move {
            let mut conn = listener.next().await.unwrap().unwrap();
            let mut buf = [0u8; 16];
            let n = conn.read(&mut buf).await.unwrap();
            conn.write_all(&buf[..n]).await.unwrap();
        });

        let mut stream = client::connect(path).await.unwrap().into_inner();
        stream.write_all(b"hello").await.unwrap();
        let mut buf = [0u8; 16];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"hello");

        server_task.await.unwrap();
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn windows_roundtrip() {
        let path = PlatformString::new_with_default(r"\\.\pipe\ak-test-roundtrip");

        let mut listener = server::listen(path.clone(), SocketPermMode::Everyone)
            .await
            .unwrap();

        let server_task = tokio::spawn(async move {
            let mut conn = listener.next().await.unwrap().unwrap();
            let mut buf = [0u8; 16];
            let n = conn.read(&mut buf).await.unwrap();
            conn.write_all(&buf[..n]).await.unwrap();
        });

        let mut stream = client::connect(path).await.unwrap().into_inner();
        stream.write_all(b"hello").await.unwrap();
        let mut buf = [0u8; 16];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"hello");

        server_task.await.unwrap();
    }
}
