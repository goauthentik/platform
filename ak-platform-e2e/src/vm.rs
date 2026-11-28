//! Minimal Incus VM client over the local unix socket.
//!
//! incus-client 0.1.1 is generated from the API spec and some bindings are
//! broken: `instances_post`, `instance_files_post` and `images_post` send no
//! body, and `instance_exec_output_get`/`instance_files_get` discard the
//! response. Those go through the crate's configured `client` directly.

use std::time::{Duration, Instant};

use eyre::{ContextCompat, Result, bail};
use futures_util::{SinkExt, StreamExt};
use incus_client::apis::configuration::Configuration;
use incus_client::apis::{instances_api, operations_api};
use incus_client::models::{
    InstanceExecPost, InstanceStatePut, InstancesPut202Response, Operation,
};
use incus_client::unix_socket::{resolve_socket_path, unix_socket_configuration};
use serde_json::{Value, json};
use tokio::net::UnixStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{WebSocketStream, client_async};

pub struct Vm {
    cfg: Configuration,
    pub name: String,
}

/// Upper bound for any single Incus operation, including a multi-GB desktop
/// image download and the longest provisioning step.
const OP_TIMEOUT_SECS: i32 = 1200;

/// Incus instance types.
pub const VIRTUAL_MACHINE: &str = "virtual-machine";
/// For images only published as containers (e.g. Amazon Linux).
pub const CONTAINER: &str = "container";

impl Vm {
    /// Creates and starts an instance of `kind` from `source` (an Incus
    /// instance source object).
    pub async fn launch(name: String, kind: &str, source: Value) -> Result<Self> {
        let cfg = unix_socket_configuration()?;
        let op: InstancesPut202Response = cfg
            .client
            .post(format!("{}/1.0/instances", cfg.base_path))
            .json(&json!({
                "name": name,
                "type": kind,
                "start": true,
                // 4GiB so desktop images can run their session alongside the tests.
                "config": { "limits.cpu": "2", "limits.memory": "4GiB" },
                // Some VM images (e.g. Rocky, Alma) don't ship incus-agent and
                // install it from this disk; without it exec never works.
                "devices": if kind == VIRTUAL_MACHINE {
                    json!({ "agent": { "type": "disk", "source": "agent:config" } })
                } else {
                    json!({})
                },
                "source": source,
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let vm = Vm { cfg, name };
        match vm.wait_ready(op).await {
            Ok(()) => Ok(vm),
            // The instance exists even if it failed to start; don't leave it
            // behind to collide with the next launch of the same name.
            Err(e) => {
                let _ = vm.delete().await;
                Err(e)
            }
        }
    }

    async fn wait_ready(&self, op: InstancesPut202Response) -> Result<()> {
        self.wait(op).await?;
        // Exec fails until the in-VM agent is up.
        let start = Instant::now();
        while self.exec("true", &[]).await.is_err() {
            if start.elapsed() > Duration::from_secs(300) {
                bail!("incus-agent in {} not ready after 300s", self.name);
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        Ok(())
    }

    /// Waits for an Incus operation (image download, exec, stop, publish)
    /// for at most [`OP_TIMEOUT_SECS`], so a stuck step fails instead of
    /// hanging the test run.
    async fn wait(&self, op: InstancesPut202Response) -> Result<Operation> {
        let id = op.metadata.and_then(|m| m.id).context("no operation id")?;
        let res = operations_api::operation_wait_get(&self.cfg, &id, Some(OP_TIMEOUT_SECS)).await?;
        let op = *res.metadata.context("no operation metadata")?;
        if op.status.as_deref() != Some("Success") {
            bail!(
                "operation {id} ({:?}) ended as {:?} within {OP_TIMEOUT_SECS}s: {:?}",
                op.description,
                op.status,
                op.err
            );
        }
        Ok(op)
    }

    /// Runs `script` as root, returning (exit code, stdout, stderr).
    pub async fn exec(&self, script: &str, env: &[(&str, &str)]) -> Result<(i64, String, String)> {
        let mut exec = InstanceExecPost::new();
        exec.command = Some(vec!["sh".into(), "-c".into(), script.into()]);
        exec.environment = Some(
            env.iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        );
        exec.record_output = Some(true);
        exec.wait_for_websocket = Some(false);
        exec.interactive = Some(false);
        let op = instances_api::instance_exec_post(&self.cfg, &self.name, None, Some(exec)).await?;
        let meta = self.wait(op).await?.metadata.unwrap_or_default();
        let mut out = [String::new(), String::new()];
        for (fd, buf) in ["1", "2"].into_iter().zip(out.iter_mut()) {
            if let Some(path) = meta
                .get("output")
                .and_then(|o| o.get(fd))
                .and_then(Value::as_str)
            {
                let url = format!("{}{path}", self.cfg.base_path);
                *buf = self.cfg.client.get(url).send().await?.text().await?;
            }
        }
        let code = meta.get("return").and_then(Value::as_i64).unwrap_or(-1);
        let [stdout, stderr] = out;
        Ok((code, stdout, stderr))
    }

    fn files_url(&self) -> String {
        format!("{}/1.0/instances/{}/files", self.cfg.base_path, self.name)
    }

    pub async fn push(&self, data: Vec<u8>, dst: &str) -> Result<()> {
        self.cfg
            .client
            .post(self.files_url())
            .query(&[("path", dst)])
            .header("X-Incus-type", "file")
            .header("X-Incus-write", "overwrite")
            .body(data)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn pull(&self, src: &str) -> Result<Vec<u8>> {
        let resp = self
            .cfg
            .client
            .get(self.files_url())
            .query(&[("path", src)])
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.bytes().await?.to_vec())
    }

    /// Stops the instance. `force` kills it without a guest shutdown, so
    /// unflushed writes are lost; only use it for throwaway instances.
    pub async fn stop(&self, force: bool) -> Result<()> {
        let mut stop = InstanceStatePut::new();
        stop.action = Some("stop".into());
        stop.force = Some(force);
        stop.timeout = Some(120);
        let op = instances_api::instance_state_put(&self.cfg, &self.name, None, Some(stop)).await?;
        self.wait(op).await?;
        Ok(())
    }

    /// Publishes the stopped VM as a local image under `alias`.
    pub async fn publish(&self, alias: &str) -> Result<()> {
        let op: InstancesPut202Response = self
            .cfg
            .client
            .post(format!("{}/1.0/images", self.cfg.base_path))
            .json(&json!({
                "source": { "type": "instance", "name": self.name },
                "aliases": [{ "name": alias }],
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        self.wait(op).await?;
        Ok(())
    }

    /// Attaches to the VM's serial console, like `incus console`.
    pub async fn console(&self) -> Result<Console> {
        let op: Value = self
            .cfg
            .client
            .post(format!(
                "{}/1.0/instances/{}/console",
                self.cfg.base_path, self.name
            ))
            .json(&json!({ "type": "console", "width": 160, "height": 50, "force": true }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let id = op["metadata"]["id"]
            .as_str()
            .context("no console operation id")?;
        let fds = &op["metadata"]["metadata"]["fds"];
        let connect = async |fd: &str| -> Result<WebSocketStream<UnixStream>> {
            let secret = fds[fd].as_str().context("no console secret")?;
            let stream = UnixStream::connect(resolve_socket_path(None).0).await?;
            let url = format!("ws://unix.socket/1.0/operations/{id}/websocket?secret={secret}");
            Ok(client_async(url, stream).await?.0)
        };
        // Incus only starts forwarding once both data and control are attached.
        let data = connect("0").await?;
        let control = connect("control").await?;
        Ok(Console {
            data,
            _control: control,
            transcript: String::new(),
            pos: 0,
        })
    }

    pub async fn delete(self) -> Result<()> {
        // Fails if already stopped (e.g. after `publish`), which is fine.
        let _ = self.stop(true).await;
        let op = instances_api::instance_delete(&self.cfg, &self.name, None).await?;
        self.wait(op).await?;
        Ok(())
    }
}

pub async fn image_alias_exists(alias: &str) -> Result<bool> {
    let cfg = unix_socket_configuration()?;
    let resp = cfg
        .client
        .get(format!("{}/1.0/images/aliases/{alias}", cfg.base_path))
        .send()
        .await?;
    Ok(resp.status().is_success())
}

/// A VM serial console session with expect-style matching.
pub struct Console {
    data: WebSocketStream<UnixStream>,
    _control: WebSocketStream<UnixStream>,
    /// Everything received so far, for matching and failure reports.
    transcript: String,
    /// Start of the not yet matched part of `transcript`.
    pos: usize,
}

impl Console {
    pub async fn send(&mut self, input: &str) -> Result<()> {
        self.data
            .send(Message::binary(input.as_bytes().to_vec()))
            .await?;
        Ok(())
    }

    /// Waits for the first of `patterns` in output not matched yet and
    /// returns its index.
    pub async fn expect(&mut self, patterns: &[&str], timeout: Duration) -> Result<usize> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let unmatched = &self.transcript[self.pos..];
            let found = patterns
                .iter()
                .enumerate()
                .filter_map(|(i, p)| unmatched.find(p).map(|at| (i, at + p.len())))
                .min_by_key(|&(_, end)| end);
            if let Some((i, end)) = found {
                self.pos += end;
                return Ok(i);
            }
            match tokio::time::timeout_at(deadline, self.data.next()).await {
                Err(_) => bail!(
                    "console: none of {patterns:?} within {timeout:?}, output:\n{}",
                    self.transcript
                ),
                Ok(None) => bail!(
                    "console closed waiting for {patterns:?}, output:\n{}",
                    self.transcript
                ),
                // ponytail: lossy per frame, so a UTF-8 char split across
                // frames turns into U+FFFD; fine for ASCII prompts.
                Ok(Some(msg)) => match msg? {
                    Message::Binary(b) => self.transcript.push_str(&String::from_utf8_lossy(&b)),
                    Message::Text(t) => self.transcript.push_str(t.as_str()),
                    _ => {}
                },
            }
        }
    }

    pub fn transcript(&self) -> &str {
        &self.transcript
    }
}
