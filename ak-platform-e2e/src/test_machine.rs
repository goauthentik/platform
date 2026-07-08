use std::{env, fs, path::PathBuf, time::Duration};

use eyre::{Context, Result, bail};
use testcontainers::{
    ContainerAsync, GenericImage, ImageExt,
    core::{CgroupnsMode, Host, Mount, logs::LogFrame},
    runners::AsyncRunner,
};
use uuid::Uuid;

use crate::{exec_command, lookup_repo_dir, must_exec};

pub struct TestMachine {
    pub container: ContainerAsync<GenericImage>,
}

impl TestMachine {
    pub async fn new() -> Result<Self> {
        let host_coverage_dir = lookup_repo_dir("ak-platform-e2e/coverage");
        // Use the compile-time manifest dir for local fs ops; cwd.parent() breaks when
        // cargo-nextest sets CWD to the workspace root rather than the package directory.
        let local_coverage_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("coverage");

        for sub in &["cli", "ak-sysd", "ak-agent", "rs"] {
            let fd = local_coverage_dir.join(sub);
            tracing::debug!(
                path = fd.to_string_lossy().to_string(),
                "Creating coverage dir"
            );
            fs::create_dir_all(fd)
                .wrap_err(format!("failed to create coverage subdir '{}'", sub))?;
        }

        let hostname = format!("test-machine-{}", Uuid::new_v4());
        let host_coverage_str = host_coverage_dir.to_string_lossy().into_owned();

        let container = GenericImage::new("xghcr.io/goauthentik/platform-e2e", "local")
            .with_env_var("GOCOVERDIR", "/tmp/ak-coverage/ak-sysd")
            .with_env_var(
                "LLVM_PROFILE_FILE",
                "/tmp/ak-coverage/rs/default_%m_%p.profraw",
            )
            .with_hostname(hostname)
            .with_user("root")
            .with_privileged(true)
            .with_cgroupns_mode(CgroupnsMode::Host)
            .with_mount(Mount::bind_mount("/sys/fs/cgroup", "/sys/fs/cgroup"))
            .with_mount(Mount::bind_mount(&host_coverage_str, "/tmp/ak-coverage"))
            .with_host("host.docker.internal", Host::HostGateway)
            .with_log_consumer(|frame: &LogFrame| {
                let line = String::from_utf8_lossy(frame.bytes());
                tracing::debug!("[e2e] {}", line.trim());
            })
            .start()
            .await
            .wrap_err("failed to start e2e container")?;

        let tm = Self { container };
        // Wait for ak-sysd to be healthy (mirrors Go's wait.ForExec)
        for attempt in 0..20 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
            let (exit_code, _) =
                exec_command(&tm.container, "/usr/bin/ak-sysd version", &[]).await?;
            if exit_code == 0 {
                return Ok(tm);
            }
        }

        bail!("ak-sysd not ready after 60 seconds")
    }
}

impl std::ops::Deref for TestMachine {
    type Target = ContainerAsync<GenericImage>;

    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

impl Drop for TestMachine {
    fn drop(&mut self) {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                for cmd in [
                    "journalctl -u ak-sysd",
                    "journalctl -u ak-agent",
                    "journalctl -u ssh",
                    "systemctl stop ak-sysd",
                    "systemctl stop ak-agent",
                ] {
                    // Don't panic here: this may run while already unwinding
                    // from a failed test, and a panic during unwind aborts
                    // the process instead of just failing the test.
                    if let Err(err) = must_exec(&self.container, cmd, &[]).await {
                        tracing::warn!("teardown command '{}' failed: {:#}", cmd, err);
                    }
                }
            });
        });
    }
}
