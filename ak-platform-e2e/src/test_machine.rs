use std::time::Duration;

use eyre::{Context, ContextCompat, Result, bail};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::vm::{VIRTUAL_MACHINE, Vm, image_alias_exists};
use crate::{is_ci, local_authentik_url, lookup_repo_dir, package_file};

/// Where instrumented binaries and PAM/NSS modules write `.profraw` in the VM.
/// Not under /tmp, which may be a tmpfs and is cleared on boot.
const PROFILE_DIR: &str = "/var/lib/ak-coverage/rs";
const PACKAGES: [&str; 5] = ["sysd", "cli", "agent", "nss", "pam"];

/// A fresh Ubuntu VM (via Incus) with the packaged debs installed, booted from
/// a base image that is built once per set of debs.
pub struct TestMachine {
    vm: Option<Vm>,
    /// authentik as reachable from inside the VM.
    pub authentik_url: String,
}

impl TestMachine {
    pub async fn new() -> Result<Self> {
        let alias = base_image().await.wrap_err("failed to build base image")?;
        let name = format!("ak-e2e-{}", &Uuid::new_v4().simple().to_string()[..12]);
        let vm = Vm::launch(
            name,
            VIRTUAL_MACHINE,
            json!({ "type": "image", "alias": alias }),
        )
        .await?;

        // authentik runs on the host, reachable via the Incus bridge gateway.
        // NetworkManager (desktop images) may not have a route yet right after boot.
        let (code, gw, stderr) = vm
            .exec(
                "for i in $(seq 60); do \
                   gw=$(ip -4 route show default | awk '{print $3; exit}'); \
                   [ -n \"$gw\" ] && echo \"$gw\" && exit 0; sleep 2; \
                 done; ip -4 route; exit 1",
                &[],
            )
            .await?;
        if code != 0 {
            bail!("no default route in VM after 120s: {gw}{stderr}");
        }
        let port = url::Url::parse(&local_authentik_url())?
            .port_or_known_default()
            .context("authentik port")?;
        let tm = TestMachine {
            vm: Some(vm),
            authentik_url: format!("http://{}:{port}", gw.trim()),
        };

        // The image ships without host keys so each machine gets its own.
        tm.must_exec(
            "ssh-keygen -A && systemctl restart ssh ak-sysd ak-agent",
            &[],
        )
        .await?;
        for attempt in 0..20 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
            if tm.exec("/usr/bin/ak-sysd version", &[]).await?.0 == 0 {
                return Ok(tm);
            }
        }
        bail!("ak-sysd not ready after 60 seconds")
    }

    fn vm(&self) -> Result<&Vm> {
        self.vm.as_ref().context("test machine already torn down")
    }

    /// Runs `cmd` via `sh -c` as root, returning (exit code, stdout + stderr).
    /// Every exec reports coverage, so `ak`/`ak-sysd` CLI calls and the
    /// PAM/NSS modules loaded by the command are all counted.
    pub async fn exec(&self, cmd: &str, env: &[(&str, &str)]) -> Result<(i64, String)> {
        if !is_ci() {
            tracing::info!("[exec] {}", cmd);
        }
        let profile = format!("{PROFILE_DIR}/default_%m_%p.profraw");
        let mut env = env.to_vec();
        env.push(("LLVM_PROFILE_FILE", &profile));
        let (code, stdout, stderr) = self
            .vm()?
            .exec(cmd, &env)
            .await
            .wrap_err(format!("exec failed: '{cmd}'"))?;

        if is_ci() {
            eprintln!("::group::{cmd} (Exit code {code})");
        } else {
            tracing::info!("[exec] {} exit={}", cmd, code);
        }
        stdout
            .lines()
            .for_each(|l| tracing::info!("[stdout] {}", l));
        stderr
            .lines()
            .for_each(|l| tracing::warn!("[stderr] {}", l));
        if is_ci() {
            eprintln!("::endgroup::");
        }
        Ok((code, format!("{stdout}{stderr}")))
    }

    /// Like [`exec`](Self::exec), but a non-zero exit is an error.
    pub async fn must_exec(&self, cmd: &str, env: &[(&str, &str)]) -> Result<String> {
        let (code, output) = self.exec(cmd, env).await?;
        if code != 0 {
            bail!("command '{cmd}' exited with code {code}: {output}");
        }
        Ok(output)
    }

    /// Stops the services so they flush their profiles, then copies every
    /// `.profraw` to where `make test-e2e-convert` picks them up.
    async fn collect_coverage(&self) -> Result<()> {
        self.must_exec("systemctl stop ak-sysd ak-agent", &[])
            .await?;
        let vm = self.vm()?;
        let local = lookup_repo_dir("ak-platform-e2e/coverage/rs");
        std::fs::create_dir_all(&local)?;
        for file in self
            .must_exec(&format!("ls {PROFILE_DIR}"), &[])
            .await?
            .lines()
        {
            let data = vm.pull(&format!("{PROFILE_DIR}/{file}")).await?;
            std::fs::write(local.join(format!("{}-{file}", vm.name)), data)?;
        }
        Ok(())
    }

    async fn teardown(&mut self) -> Result<()> {
        // Logged for debugging failed tests.
        self.exec("journalctl --no-pager", &[]).await?;
        let coverage = self.collect_coverage().await;
        if let Some(vm) = self.vm.take() {
            vm.delete().await?;
        }
        coverage
    }
}

impl Drop for TestMachine {
    fn drop(&mut self) {
        tokio::task::block_in_place(|| {
            tracing::info!("Test machine cleanup");
            // Don't panic here: this may run while already unwinding from a
            // failed test, and a panic during unwind aborts the process.
            if let Err(err) = tokio::runtime::Handle::current().block_on(self.teardown()) {
                tracing::warn!("test machine teardown failed: {:#}", err);
            }
        });
    }
}

/// Returns the alias of a local image with the current debs installed,
/// building it first if needed. Keyed by the debs' content, so rebuilt
/// packages get a new image.
///
/// ponytail: no locking; relies on e2e tests running one at a time
/// (`--test-threads 1`). Old images are left behind locally.
async fn base_image() -> Result<String> {
    let version = std::env::var("AK_E2E_UBUNTU_VERSION").unwrap_or_else(|_| "24.04".into());
    // `cloud` or `desktop` (GNOME, no cloud-init); see images.linuxcontainers.org.
    let variant = std::env::var("AK_E2E_UBUNTU_VARIANT").unwrap_or_else(|_| "cloud".into());
    let mut hash = Sha256::new();
    let mut debs = Vec::new();
    for pkg in PACKAGES {
        let path = package_file(pkg, "deb")?;
        let data = std::fs::read(&path)?;
        hash.update(&data);
        let file = path
            .file_name()
            .context("deb name")?
            .to_string_lossy()
            .into_owned();
        debs.push((file, data));
    }
    let alias = format!(
        "ak-e2e-{}-{variant}-{}",
        version.replace('.', ""),
        &hex::encode(hash.finalize())[..12]
    );
    if image_alias_exists(&alias).await? {
        return Ok(alias);
    }

    tracing::info!(alias, "building base image");
    let builder = Vm::launch(
        format!("{alias}-build"),
        VIRTUAL_MACHINE,
        json!({
            "type": "image",
            "alias": format!("ubuntu/{version}/{variant}"),
            "server": "https://images.linuxcontainers.org",
            "protocol": "simplestreams",
        }),
    )
    .await?;
    let result = provision(&builder, debs).await;
    if result.is_ok() {
        // A clean shutdown, so everything provisioned is on disk before publishing.
        builder.stop(false).await?;
        builder.publish(&alias).await?;
    }
    builder.delete().await?;
    result.map(|_| alias)
}

/// Same setup as the former e2e container image, as separate logged steps so
/// a slow or stuck one is visible in the test output.
async fn provision(vm: &Vm, debs: Vec<(String, Vec<u8>)>) -> Result<()> {
    for (file, data) in debs {
        vm.push(data, &format!("/root/{file}")).await?;
    }
    let steps = [
        // Fresh cloud and desktop images run cloud-init, the apt timers,
        // unattended-upgrades and (desktop) packagekit on first boot, all of
        // which hold the apt lock. Stop them, then wait for whatever is
        // already running to let go.
        (
            "quiesce package managers",
            "if command -v cloud-init >/dev/null; then cloud-init status --wait || true; fi\n\
             systemctl mask --now apt-daily.timer apt-daily-upgrade.timer \
               apt-daily.service apt-daily-upgrade.service unattended-upgrades.service \
               packagekit.service 2>/dev/null || true\n\
             while fuser /var/lib/dpkg/lock-frontend /var/lib/apt/lists/lock >/dev/null 2>&1; do sleep 2; done"
                .to_string(),
        ),
        // Drop-ins go in before install, so the postinst restart already uses
        // them. `%%p` is systemd's escape for a literal `%p` (the PID, for LLVM).
        (
            "coverage drop-ins",
            format!(
                "mkdir -p -m 1777 {PROFILE_DIR}\n\
                 for unit in ak-sysd ak-agent; do\n\
                   mkdir -p /etc/systemd/system/$unit.service.d\n\
                   printf '[Service]\\nEnvironment=LLVM_PROFILE_FILE={PROFILE_DIR}/%s-%%%%p.profraw\\n' $unit \
                     > /etc/systemd/system/$unit.service.d/coverage.conf\n\
                 done\n\
                 printf '[Service]\\nEnvironment=HOME=/root\\n' > /etc/systemd/system/ak-agent.service.d/home.conf"
            ),
        ),
        ("apt update", "apt-get -o Acquire::Retries=3 update".to_string()),
        (
            "install packages",
            "DEBIAN_FRONTEND=noninteractive apt-get -o Acquire::Retries=3 install -y pamtester openssh-server /root/*.deb"
                .to_string(),
        ),
        (
            "configure",
            "set -e\n\
             cp /etc/systemd/user/ak-agent.service /usr/lib/systemd/system\n\
             systemctl daemon-reload\n\
             systemctl enable ak-agent\n\
             sed -i 's/false/true/g' /etc/authentik/config.json\n\
             sed -i 's/pam_authentik.so/pam_authentik.so debug/g' /etc/pam.d/*"
                .to_string(),
        ),
        (
            "clean up",
            format!(
                "systemctl stop ak-sysd ak-agent\n\
                 rm -f /root/*.deb {PROFILE_DIR}/* /etc/ssh/ssh_host_*\n\
                 truncate -s 0 /etc/machine-id"
            ),
        ),
    ];
    for (name, script) in steps {
        tracing::info!(vm = vm.name, step = name, "provisioning");
        let start = std::time::Instant::now();
        let (code, stdout, stderr) = vm.exec(&script, &[]).await?;
        tracing::info!(
            vm = vm.name,
            step = name,
            code,
            secs = start.elapsed().as_secs(),
            "provisioning step done"
        );
        if code != 0 {
            bail!("provisioning step '{name}' exited with {code}:\n{stdout}{stderr}");
        }
    }
    Ok(())
}
