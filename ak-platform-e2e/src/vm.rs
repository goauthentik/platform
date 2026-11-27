//! Minimal Incus VM client over the local unix socket.
//!
//! incus-client 0.1.1 is generated from the API spec and some bindings are
//! broken: `instances_post`, `instance_files_post` and `images_post` send no
//! body, and `instance_exec_output_get`/`instance_files_get` discard the
//! response. Those go through the crate's configured `client` directly.

use std::time::{Duration, Instant};

use eyre::{ContextCompat, Result, bail};
use incus_client::apis::configuration::Configuration;
use incus_client::apis::{instances_api, operations_api};
use incus_client::models::{
    InstanceExecPost, InstanceStatePut, InstancesPut202Response, Operation,
};
use incus_client::unix_socket::unix_socket_configuration;
use serde_json::{Value, json};

pub struct Vm {
    cfg: Configuration,
    pub name: String,
}

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
                "config": { "limits.cpu": "2", "limits.memory": "2GiB" },
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
        vm.wait(op).await?;
        // Exec fails until the in-VM agent is up.
        let start = Instant::now();
        while vm.exec("true", &[]).await.is_err() {
            if start.elapsed() > Duration::from_secs(300) {
                bail!("incus-agent in {} not ready after 300s", vm.name);
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        Ok(vm)
    }

    async fn wait(&self, op: InstancesPut202Response) -> Result<Operation> {
        let id = op.metadata.and_then(|m| m.id).context("no operation id")?;
        let res = operations_api::operation_wait_get(&self.cfg, &id, Some(-1)).await?;
        let op = *res.metadata.context("no operation metadata")?;
        if op.status.as_deref() != Some("Success") {
            bail!("operation {id} failed: {:?}", op.err);
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
