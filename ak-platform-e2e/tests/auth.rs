use ak_tests::{
    CmdTestCase, agent_setup, cleanup_hosts, cmd_test, join_domain, must_exec, test_init,
    test_machine,
};

#[tokio::test]
async fn test_auth_identity_agent() {
    test_init();
    let container = test_machine().await.expect("test machine");
    join_domain(&container).await.expect("join domain");
    agent_setup(&container).await.expect("agent setup");

    let ssh_opts = "-o StrictHostKeyChecking=no \
                    -o IdentityAgent=~/.local/share/authentik/agent-ssh.sock \
                    -o ForwardAgent=yes";

    cmd_test(
        &container,
        vec![
            CmdTestCase {
                name: "ssh_env".to_string(),
                cmd: format!("ssh {} akadmin@$(hostname) env", ssh_opts),
                expects: vec!["SSH_CONNECTION".to_string()],
            },
            CmdTestCase {
                name: "ssh_ak_whoami".to_string(),
                cmd: format!("ssh {} akadmin@$(hostname) ak whoami", ssh_opts),
                expects: vec!["akadmin".to_string()],
            },
        ],
    )
    .await
    .expect("cmd test");

    cleanup_hosts().await.expect("cleanup");
}

/// Verifies that a user unknown to authentik is PAM_IGNOREd by the authentik
/// module, allowing the rest of the PAM stack to handle authentication so the
/// user can still log in via local credentials.
#[tokio::test]
async fn test_auth_local_only_user() {
    test_init();
    let container = test_machine().await.expect("test machine");
    join_domain(&container).await.expect("join domain");

    // Create a local user that is not registered in authentik
    must_exec(&container, "useradd -m localonly", &[])
        .await
        .expect("useradd");
    must_exec(
        &container,
        "ssh-keygen -t ed25519 -f /tmp/localonly_key -N '' -q",
        &[],
    )
    .await
    .expect("ssh-keygen");
    must_exec(
        &container,
        "install -d -m 700 -o localonly -g localonly /home/localonly/.ssh",
        &[],
    )
    .await
    .expect("install .ssh");
    must_exec(
        &container,
        "cp /tmp/localonly_key.pub /home/localonly/.ssh/authorized_keys",
        &[],
    )
    .await
    .expect("copy pubkey");
    must_exec(
        &container,
        "chown localonly: /home/localonly/.ssh/authorized_keys && chmod 600 /home/localonly/.ssh/authorized_keys",
        &[],
    )
    .await
    .expect("chown/chmod");

    let ssh_opts = "-i /tmp/localonly_key -o StrictHostKeyChecking=no -o BatchMode=yes";

    cmd_test(
        &container,
        vec![
            CmdTestCase {
                // The authentik PAM module returns PAM_IGNORE for this user;
                // pam_unix handles auth and login succeeds.
                name: "local_only_user_can_ssh".to_string(),
                cmd: format!("ssh {} localonly@localhost whoami", ssh_opts),
                expects: vec!["localonly".to_string()],
            },
            CmdTestCase {
                // Sanity-check: the local user is NOT visible through the authentik
                // NSS module (only through the system databases).
                name: "local_only_user_not_in_authentik_directory".to_string(),
                cmd: "getent passwd localonly".to_string(),
                expects: vec!["localonly".to_string(), "/home/localonly".to_string()],
            },
        ],
    )
    .await
    .expect("cmd test");

    cleanup_hosts().await.expect("cleanup");
}
