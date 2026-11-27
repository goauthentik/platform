#![cfg(unix)]

//! Package install smoke tests across distros, each on a fresh cloud image.

use ak_platform_e2e::package_file;
use ak_platform_e2e::test_init;
use ak_platform_e2e::vm::{CONTAINER, VIRTUAL_MACHINE, Vm};
use eyre::{ContextCompat, Result, bail};
use serde_json::json;
use uuid::Uuid;

/// Installed one at a time, in this order.
const PACKAGES: [&str; 5] = ["cli", "agent", "sysd", "pam", "nss"];

/// Boots `alias` from images.linuxcontainers.org and runs `install <file>` for
/// each package.
async fn install_all(alias: &str, kind: &str, ext: &str, install: &str) -> Result<()> {
    let name = format!("ak-e2e-pkg-{}", &Uuid::new_v4().simple().to_string()[..12]);
    let source = json!({
        "type": "image",
        "alias": alias,
        "server": "https://images.linuxcontainers.org",
        "protocol": "simplestreams",
    });
    let vm = Vm::launch(name, kind, source).await?;
    let result = async {
        // cloud-init holds the package manager lock on first boot. Its exit
        // code is non-zero for recoverable warnings, so only wait on it.
        vm.exec(
            "command -v cloud-init >/dev/null && cloud-init status --wait",
            &[],
        )
        .await?;
        for pkg in PACKAGES {
            let src = package_file(pkg, ext)?;
            let file = src.file_name().context("package name")?.to_string_lossy();
            let dst = format!("/root/{file}");
            vm.push(std::fs::read(&src)?, &dst).await?;
            let (code, stdout, stderr) = vm.exec(&format!("{install} {dst}"), &[]).await?;
            if code != 0 {
                bail!("{install} {file} on {alias} exited with {code}:\n{stdout}{stderr}");
            }
        }
        Ok(())
    }
    .await;
    vm.delete().await?;
    result
}

#[tokio::test(flavor = "multi_thread")]
async fn test_packaging_deb() {
    test_init();
    for alias in [
        "ubuntu/24.04/cloud",
        "ubuntu/26.04/cloud",
        "debian/13/cloud",
    ] {
        install_all(alias, VIRTUAL_MACHINE, "deb", "dpkg -i")
            .await
            .unwrap_or_else(|e| panic!("{alias}: {e:?}"));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_packaging_rpm() {
    test_init();
    for (alias, kind) in [
        // Closest to RHEL/UBI, which isn't published on the image server.
        ("rockylinux/10/cloud", VIRTUAL_MACHINE),
        ("almalinux/10/cloud", VIRTUAL_MACHINE),
        // Only published as a container image.
        ("amazonlinux/2023", CONTAINER),
    ] {
        install_all(alias, kind, "rpm", "yum install -y")
            .await
            .unwrap_or_else(|e| panic!("{alias}: {e:?}"));
    }
}
