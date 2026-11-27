//! Interactive login on a real Ubuntu VM booted through Incus: installs the
//! packaged debs, joins the domain and logs in with `/bin/login` on a pty. This
//! covers what the container tests can't: a real kernel, the packages' own
//! AppArmor and pam-auth-update setup, and the full PAM stack (auth + session).
//!
//! Needs an Incus daemon with KVM (socket access via `incus-admin`), the debs
//! under `bin/{sysd,nss,pam}/` and authentik reachable from the VM through the
//! Incus bridge. Run with
//! `cargo nextest run -p ak-platform-e2e --test vm_login --run-ignored only`.
#![cfg(unix)]

use std::time::{Duration, Instant};

use ak_platform_e2e::{
    authentik_creds, cleanup_hosts, local_authentik_url, lookup_repo_dir, test_init,
};
use eyre::{ContextCompat, Result, bail};
use incus_client::apis::configuration::Configuration;
use incus_client::apis::{instances_api, operations_api};
use incus_client::models::{
    InstanceExecPost, InstanceStatePut, InstancesPut202Response, Operation,
};
use incus_client::unix_socket::unix_socket_configuration;
use serde_json::{Value, json};

struct Vm {
    cfg: Configuration,
    name: String,
}

impl Vm {
    /// Creates and starts a VM from the public `images:` remote.
    async fn launch(name: String, alias: &str) -> Result<Self> {
        let cfg = unix_socket_configuration()?;
        // ponytail: raw request, incus-client 0.1.1's `instances_post` sends no body.
        let op: InstancesPut202Response = cfg
            .client
            .post(format!("{}/1.0/instances", cfg.base_path))
            .json(&json!({
                "name": name,
                "type": "virtual-machine",
                "start": true,
                "config": { "limits.cpu": "2", "limits.memory": "2GiB" },
                "source": {
                    "type": "image",
                    "alias": alias,
                    "server": "https://images.linuxcontainers.org",
                    "protocol": "simplestreams",
                },
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let vm = Vm { cfg, name };
        vm.wait(op).await?;
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

    /// Runs `script` as root, returning stdout+stderr; non-zero exit is an error.
    async fn sh(&self, script: &str, env: &[(&str, &str)]) -> Result<String> {
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
        let mut out = String::new();
        for fd in ["1", "2"] {
            if let Some(path) = meta
                .get("output")
                .and_then(|o| o.get(fd))
                .and_then(Value::as_str)
            {
                // ponytail: raw request, `instance_exec_output_get` discards the body.
                let url = format!("{}{path}", self.cfg.base_path);
                out += &self.cfg.client.get(url).send().await?.text().await?;
            }
        }
        match meta.get("return").and_then(Value::as_i64) {
            Some(0) => Ok(out),
            code => bail!("`{script}` exited with {code:?}:\n{out}"),
        }
    }

    /// Retries `script` until it succeeds or `timeout` passes.
    async fn wait_for(&self, script: &str, timeout: Duration) -> Result<String> {
        let start = Instant::now();
        loop {
            match self.sh(script, &[]).await {
                Ok(out) => return Ok(out),
                Err(e) if start.elapsed() > timeout => return Err(e),
                Err(_) => tokio::time::sleep(Duration::from_secs(3)).await,
            }
        }
    }

    async fn push(&self, src: &std::path::Path, dst: &str) -> Result<()> {
        // ponytail: raw request, `instance_files_post` sends no body.
        self.cfg
            .client
            .post(format!(
                "{}/1.0/instances/{}/files",
                self.cfg.base_path, self.name
            ))
            .query(&[("path", dst)])
            .header("X-Incus-type", "file")
            .header("X-Incus-write", "overwrite")
            .body(std::fs::read(src)?)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    async fn delete(self) -> Result<()> {
        let mut stop = InstanceStatePut::new();
        stop.action = Some("stop".into());
        stop.force = Some(true);
        let op = instances_api::instance_state_put(&self.cfg, &self.name, None, Some(stop)).await?;
        self.wait(op).await?;
        let op = instances_api::instance_delete(&self.cfg, &self.name, None).await?;
        self.wait(op).await?;
        Ok(())
    }
}

/// The single `.deb` in `bin/<dir>/`.
fn deb(dir: &str) -> Result<std::path::PathBuf> {
    let path = lookup_repo_dir(&format!("bin/{dir}"));
    for entry in std::fs::read_dir(&path)? {
        let p = entry?.path();
        if p.extension().is_some_and(|e| e == "deb") {
            return Ok(p);
        }
    }
    bail!("no .deb in {}", path.display())
}

/// Tests requiring an enterprise license; skipped by the `no-enterprise` nextest profile.
mod enterprise {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "needs incus with KVM; see module docs"]
    async fn vm_interactive_login() -> Result<()> {
        test_init();
        let version = std::env::var("AK_E2E_UBUNTU_VERSION").unwrap_or_else(|_| "24.04".into());
        let name = format!("ak-e2e-login-{}", std::process::id());
        let vm = Vm::launch(name, &format!("ubuntu/{version}")).await?;
        let result = login(&vm).await;
        if result.is_err() {
            let diag = vm
                .sh("journalctl -u ak-sysd --no-pager | tail -200", &[])
                .await;
            eprintln!("{}", diag.unwrap_or_else(|e| format!("{e:?}")));
        }
        vm.delete().await?;
        cleanup_hosts().await?;
        result
    }
}

async fn login(vm: &Vm) -> Result<()> {
    // Exec fails until the in-VM agent is up.
    vm.wait_for("true", Duration::from_secs(300)).await?;
    for d in ["sysd", "nss", "pam"] {
        let src = deb(d)?;
        let file = src.file_name().context("deb name")?.to_string_lossy();
        vm.push(&src, &format!("/root/{file}")).await?;
    }
    vm.wait_for("apt-get update", Duration::from_secs(120))
        .await?;
    vm.sh(
        "DEBIAN_FRONTEND=noninteractive apt-get install -y /root/*.deb",
        &[],
    )
    .await?;
    vm.wait_for("systemctl is-active ak-sysd", Duration::from_secs(60))
        .await?;

    // authentik runs on the host, which the VM reaches via its default gateway
    // (the Incus bridge).
    let port = url::Url::parse(&local_authentik_url())?
        .port_or_known_default()
        .context("authentik port")?;
    let gw = vm
        .sh("ip -4 route show default | awk '{print $3}'", &[])
        .await?;
    vm.sh(
        &format!("ak-sysd domains join ak -a http://{}:{port}", gw.trim()),
        &[("AK_SYS_INSECURE_ENV_TOKEN", "test-enroll-key")],
    )
    .await?;
    vm.wait_for("getent passwd akadmin", Duration::from_secs(60))
        .await?;

    // `login` reads from a terminal, so `script` gives it a pty. The marker is
    // only printed expanded by the logged-in shell, never by the input echo.
    let (_, password) = authentik_creds();
    let out = vm
        .sh(
            "{ sleep 3; printf '%s\\n' \"$AK_LOGIN_PW\"; sleep 20; \
               printf 'echo LOGGED_IN_AS_$(id -un); exit\\n'; } \
             | script -qefc 'login akadmin' /dev/null",
            &[("AK_LOGIN_PW", &password)],
        )
        .await?;
    if !out.contains("LOGGED_IN_AS_akadmin") {
        bail!("interactive login did not reach a shell:\n{out}");
    }
    Ok(())
}
