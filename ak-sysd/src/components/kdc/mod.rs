//! Local KDC that lets `ee/wcp` log a Windows user on after an authentik
//! sign-in without touching the local account's password. Windows is pointed
//! at it as an MIT realm with `ksetup`, the same arrangement FreeIPA documents
//! for Windows clients, with every realm user mapped onto the local account of
//! the same name.
//!
//! Each user gets a stable random Kerberos password, so DPAPI keeps working
//! across logons, but the KDC only accepts it within [`proto::ARM_WINDOW`] of
//! a successful `TokenAuth` for that user.

use std::sync::{Arc, Mutex};

use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use eyre::Result;
use rand::Rng;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UdpSocket};
use tokio_util::sync::CancellationToken;

use crate::components::{Component, SysdContext};
use ak_platform::paths::SysdSocketID;

mod proto;

/// Not a real DNS domain (`.local` is mDNS-only), so Windows finds the KDC
/// through the `ksetup /addkdc` mapping rather than SRV lookups.
pub const REALM: &str = "AUTHENTIK.LOCAL";
const LISTEN: &str = "127.0.0.1:88";
/// Requests and replies here are a few hundred bytes; anything near this is
/// not Kerberos.
const MAX_MESSAGE: usize = 64 * 1024;

pub struct KdcComponent {
    ctx: SysdContext,
    running: Mutex<Option<(Arc<proto::Kdc>, CancellationToken)>>,
}

impl KdcComponent {
    pub fn new(ctx: SysdContext) -> Self {
        Self {
            ctx,
            running: Mutex::new(None),
        }
    }

    /// Clears `username` for one Kerberos logon and returns the realm and
    /// password to submit it with. `None` while the KDC is not running.
    pub async fn issue(&self, username: &str) -> Result<Option<(String, String)>> {
        let Some(kdc) = self
            .running
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|(kdc, _)| Arc::clone(kdc))
        else {
            return Ok(None);
        };
        let password = self
            .secret(&format!("user:{}", username.to_lowercase()))
            .await?;
        kdc.arm(username, &password, chrono::Utc::now());
        Ok(Some((kdc.realm.clone(), password)))
    }

    /// Get-or-create a random secret persisted under `key`.
    async fn secret(&self, key: &str) -> Result<String> {
        let kv = self.ctx.state.component_kv(Self::id());
        if let Some(secret) = kv.get(key).await? {
            return Ok(secret);
        }
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let secret = BASE64_URL_SAFE_NO_PAD.encode(bytes);
        kv.set(key, &secret).await?;
        Ok(secret)
    }
}

#[tonic::async_trait]
impl Component for KdcComponent {
    fn id() -> &'static str {
        "kdc"
    }

    async fn start(&self) -> Result<()> {
        let machine_password = self.secret("machine_password").await?;
        #[cfg(windows)]
        match self.configure_windows(&machine_password).await {
            Ok(true) => {}
            // Handing out realm credentials Windows cannot use yet would fail
            // every sign-in, where not starting falls back to the local
            // password.
            Ok(false) => {
                tracing::warn!("kdc: Kerberos realm takes effect after a reboot; not starting");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("kdc: could not configure the Kerberos realm, not starting: {e:?}");
                return Ok(());
            }
        }

        let kdc = Arc::new(proto::Kdc::new(REALM.to_string(), machine_password));
        let cancel = self.ctx.cancel.child_token();
        let udp = UdpSocket::bind(LISTEN).await?;
        let tcp = TcpListener::bind(LISTEN).await?;
        tokio::spawn(serve_udp(udp, Arc::clone(&kdc), cancel.clone()));
        tokio::spawn(serve_tcp(tcp, Arc::clone(&kdc), cancel.clone()));
        tracing::info!(realm = REALM, "kdc: listening on {LISTEN}");

        if let Some((_, old)) = self
            .running
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .replace((kdc, cancel))
        {
            old.cancel();
        }
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        if let Some((_, cancel)) = self
            .running
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            cancel.cancel();
        }
        Ok(())
    }

    fn register(
        self: Arc<Self>,
        _socket: SysdSocketID,
        _routes: &mut tonic::service::RoutesBuilder,
    ) {
    }
}

#[cfg(windows)]
impl KdcComponent {
    /// Points Windows at this KDC once, returning whether that has taken
    /// effect yet, which takes a reboot. `ksetup` itself refuses on a
    /// domain-joined machine, which is where this has to stop.
    async fn configure_windows(&self, machine_password: &str) -> Result<bool> {
        let kv = self.ctx.state.component_kv(Self::id());
        let now = chrono::Utc::now().timestamp();
        if let Some(configured_at) = kv.get("configured_at").await? {
            let uptime = unsafe { windows::Win32::System::SystemInformation::GetTickCount64() };
            let booted_at = now - (uptime / 1000) as i64;
            return Ok(booted_at > configured_at.parse::<i64>()?);
        }
        let kdc_host = LISTEN.split(':').next().unwrap_or("127.0.0.1");
        for args in [
            vec!["/setrealm", REALM],
            vec!["/addkdc", REALM, kdc_host],
            vec!["/mapuser", "*", "*"],
            vec!["/setcomputerpassword", machine_password],
        ] {
            let out = tokio::process::Command::new("ksetup")
                .args(&args)
                .output()
                .await?;
            if !out.status.success() {
                eyre::bail!(
                    "ksetup {} failed: {}",
                    args[0],
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
        }
        kv.set("configured_at", &now.to_string()).await?;
        tracing::info!(realm = REALM, "kdc: Kerberos realm configured");
        Ok(false)
    }
}

async fn serve_udp(socket: UdpSocket, kdc: Arc<proto::Kdc>, cancel: CancellationToken) {
    let mut buf = vec![0u8; MAX_MESSAGE];
    loop {
        let (len, peer) = tokio::select! {
            _ = cancel.cancelled() => return,
            r = socket.recv_from(&mut buf) => match r {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("kdc: udp receive failed: {e}");
                    continue;
                }
            },
        };
        let reply = kdc.handle(&buf[..len], chrono::Utc::now());
        if let Err(e) = socket.send_to(&reply, peer).await {
            tracing::warn!("kdc: udp reply failed: {e}");
        }
    }
}

async fn serve_tcp(listener: TcpListener, kdc: Arc<proto::Kdc>, cancel: CancellationToken) {
    loop {
        let mut stream = tokio::select! {
            _ = cancel.cancelled() => return,
            r = listener.accept() => match r {
                Ok((stream, _)) => stream,
                Err(e) => {
                    tracing::warn!("kdc: tcp accept failed: {e}");
                    continue;
                }
            },
        };
        let kdc = Arc::clone(&kdc);
        tokio::spawn(async move {
            // RFC 4120 7.2.2: each message is prefixed with its 4-byte length.
            while let Ok(len) = stream.read_u32().await {
                if len as usize > MAX_MESSAGE {
                    return;
                }
                let mut req = vec![0u8; len as usize];
                if stream.read_exact(&mut req).await.is_err() {
                    return;
                }
                let reply = kdc.handle(&req, chrono::Utc::now());
                if stream.write_u32(reply.len() as u32).await.is_err()
                    || stream.write_all(&reply).await.is_err()
                {
                    return;
                }
            }
        });
    }
}
