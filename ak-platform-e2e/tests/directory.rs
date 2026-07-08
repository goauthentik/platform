use ak_platform_e2e::{
    CmdTestCase, TestMachine, cleanup_hosts, cmd_test, join_domain, must_exec, test_init,
};
use testcontainers::{ContainerAsync, GenericImage};

/// Returns the third colon-separated field (UID or GID) for the given key
/// from `getent` style output.
async fn getent_lookup(container: &ContainerAsync<GenericImage>, cmd: &str, key: &str) -> String {
    let output = must_exec(container, cmd, &[]).await.expect("getent");
    for line in output.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.first().copied() == Some(key) && parts.len() > 2 {
            return parts[2].to_string();
        }
    }
    panic!("key '{}' not found in '{}' output", key, cmd);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_directory_user() {
    test_init();
    let tm = TestMachine::new().await.expect("test machine");
    join_domain(&tm).await.expect("join domain");

    let uid = getent_lookup(&tm, "getent passwd akadmin", "akadmin").await;

    cmd_test(
        &tm,
        vec![
            CmdTestCase {
                name: "getent_user_all".to_string(),
                cmd: "getent passwd".to_string(),
                expects: vec!["akadmin".to_string(), "authentik Default Admin".to_string()],
            },
            CmdTestCase {
                name: "getent_user_by_name".to_string(),
                cmd: "getent passwd akadmin".to_string(),
                expects: vec![
                    "akadmin".to_string(),
                    "authentik Default Admin".to_string(),
                    uid.clone(),
                ],
            },
            CmdTestCase {
                name: "getent_user_by_id".to_string(),
                cmd: format!("getent passwd {}", uid),
                expects: vec![
                    "akadmin".to_string(),
                    "authentik Default Admin".to_string(),
                    uid.clone(),
                ],
            },
        ],
    )
    .await
    .expect("cmd test");

    cleanup_hosts().await.expect("cleanup");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_directory_group() {
    test_init();
    let tm = TestMachine::new().await.expect("test machine");
    join_domain(&tm).await.expect("join domain");

    let gid = getent_lookup(&tm, "getent group akadmin", "akadmin").await;

    cmd_test(
        &tm,
        vec![
            CmdTestCase {
                name: "getent_group_all".to_string(),
                cmd: "getent group".to_string(),
                expects: vec!["akadmin".to_string()],
            },
            CmdTestCase {
                name: "getent_group_by_name".to_string(),
                cmd: "getent group akadmin".to_string(),
                expects: vec!["akadmin".to_string(), gid.clone()],
            },
            CmdTestCase {
                name: "getent_group_by_id".to_string(),
                cmd: format!("getent group {}", gid),
                expects: vec!["akadmin".to_string(), gid.clone()],
            },
        ],
    )
    .await
    .expect("cmd test");

    cleanup_hosts().await.expect("cleanup");
}
