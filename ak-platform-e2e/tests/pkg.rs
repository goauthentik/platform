use ak_platform_e2e::{lookup_repo_dir, must_exec, test_init};
use testcontainers::{ContainerAsync, GenericImage, ImageExt, core::Mount, runners::AsyncRunner};

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
        ("docker.io/library/debian", "13"),
    ] {
        let container = pkg_container(image, tag, &bin_dir).await;

        for pkg in &[
            "/tmp/ak-bin/cli/*.deb",
            "/tmp/ak-bin/agent/*.deb",
            "/tmp/ak-bin/agent_system/*.deb",
            "/tmp/ak-bin/pam/*.deb",
            "/tmp/ak-bin/nss/*.deb",
        ] {
            must_exec(&container, &format!("dpkg -i {}", pkg), &[])
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
    ] {
        let container = pkg_container(image, tag, &bin_dir).await;

        for pkg in &[
            "/tmp/ak-bin/cli/*.rpm",
            "/tmp/ak-bin/agent/*.rpm",
            "/tmp/ak-bin/agent_system/*.rpm",
            "/tmp/ak-bin/nss/*.rpm",
            "/tmp/ak-bin/pam/*.rpm",
        ] {
            must_exec(&container, &format!("yum install -y {}", pkg), &[])
                .await
                .unwrap_or_else(|e| panic!("yum install {} on {}/{}: {}", pkg, image, tag, e));
        }
    }
}
