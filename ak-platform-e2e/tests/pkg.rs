#![cfg(unix)]

//! Package install smoke tests across distros. These stay on plain containers:
//! they only need a package manager, not systemd or a real kernel.

use ak_platform_e2e::{lookup_repo_dir, test_init};
use testcontainers::core::{CmdWaitFor, ExecCommand};
use testcontainers::{ContainerAsync, GenericImage, ImageExt, core::Mount, runners::AsyncRunner};

async fn must_exec(container: &ContainerAsync<GenericImage>, cmd: &str) -> Result<String, String> {
    let exec = ExecCommand::new(["sh", "-c", cmd])
        .with_cmd_ready_condition(CmdWaitFor::Exit { code: None });
    let mut result = container.exec(exec).await.map_err(|e| e.to_string())?;
    let stdout = result.stdout_to_vec().await.map_err(|e| e.to_string())?;
    let stderr = result.stderr_to_vec().await.map_err(|e| e.to_string())?;
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr)
    );
    match result.exit_code().await.map_err(|e| e.to_string())? {
        Some(0) => Ok(output),
        code => Err(format!("exited with {code:?}: {output}")),
    }
}

async fn pkg_container(image: &str, tag: &str, bin_dir: &str) -> ContainerAsync<GenericImage> {
    GenericImage::new(image, tag)
        .with_entrypoint("/bin/bash")
        .with_cmd(["-c", "sleep infinity"])
        .with_mount(Mount::bind_mount(bin_dir, "/tmp/ak-bin"))
        .start()
        .await
        .unwrap_or_else(|e| panic!("failed to start container for {}/{}: {}", image, tag, e))
}

#[tokio::test]
async fn test_packaging_deb() {
    test_init();
    let bin_dir = lookup_repo_dir("bin").to_string_lossy().into_owned();

    for (image, tag) in &[
        ("docker.io/library/ubuntu", "24.04"),
        ("docker.io/library/ubuntu", "25.10"),
        ("docker.io/library/ubuntu", "26.04"),
        ("docker.io/library/debian", "13"),
    ] {
        let container = pkg_container(image, tag, &bin_dir).await;

        for pkg in &[
            "/tmp/ak-bin/cli/*.deb",
            "/tmp/ak-bin/agent/*.deb",
            "/tmp/ak-bin/sysd/*.deb",
            "/tmp/ak-bin/pam/*.deb",
            "/tmp/ak-bin/nss/*.deb",
        ] {
            must_exec(&container, &format!("dpkg -i {}", pkg))
                .await
                .unwrap_or_else(|e| panic!("dpkg install {} on {}/{}: {}", pkg, image, tag, e));
        }
    }
}

#[tokio::test]
async fn test_packaging_rpm() {
    test_init();
    let bin_dir = lookup_repo_dir("bin").to_string_lossy().into_owned();

    for (image, tag) in &[
        ("docker.io/redhat/ubi10", "latest"),
        ("docker.io/library/almalinux", "10"),
        ("docker.io/library/amazonlinux", "2023"),
    ] {
        let container = pkg_container(image, tag, &bin_dir).await;

        for pkg in &[
            "/tmp/ak-bin/cli/*.rpm",
            "/tmp/ak-bin/agent/*.rpm",
            "/tmp/ak-bin/sysd/*.rpm",
            "/tmp/ak-bin/nss/*.rpm",
            "/tmp/ak-bin/pam/*.rpm",
        ] {
            must_exec(&container, &format!("yum install -y {}", pkg))
                .await
                .unwrap_or_else(|e| panic!("yum install {} on {}/{}: {}", pkg, image, tag, e));
        }
    }
}
