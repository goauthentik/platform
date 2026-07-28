use ak_platform_e2e::{
    TestMachine, authentik_creds, cleanup_hosts, exec_command, join_domain, test_init,
};

/// Verifies that a real local (non-SSH) login via the `login` PAM service is
/// authenticated against authentik, using `pamtester` to drive PAM without a
/// controlling TTY.
#[tokio::test(flavor = "multi_thread")]
async fn test_local_login_success() {
    test_init();
    let tm = TestMachine::new().await.expect("test machine");
    join_domain(&tm).await.expect("join domain");

    let (_, password) = authentik_creds();
    let (exit_code, output) = exec_command(
        &tm.container,
        "printf '%s\\n' \"$AK_LOGIN_PW\" | pamtester login akadmin authenticate",
        &[("AK_LOGIN_PW", &password)],
    )
    .await
    .expect("exec pamtester");

    if exit_code != 0 {
        // ak-sysd logs to the systemd journal, but ak-pam (the client-side PAM
        // module loaded into pamtester's own process) logs separately via
        // syslog under its own identifier, not the ak-sysd unit — a
        // client-side failure after a successful server response would be
        // invisible if we only dumped the former, so grab both.
        let (_, sysd_journal) = exec_command(
            &tm.container,
            "journalctl -u ak-sysd --no-pager -n 200",
            &[],
        )
        .await
        .unwrap_or_default();
        let (_, pam_journal) = exec_command(
            &tm.container,
            "journalctl -t libpam-authentik --no-pager -n 200",
            &[],
        )
        .await
        .unwrap_or_default();
        panic!(
            "pamtester authenticate failed: {output}\n--- ak-sysd journal ---\n{sysd_journal}\n--- libpam-authentik journal ---\n{pam_journal}"
        );
    }

    assert!(
        output.contains("successfully authenticated"),
        "expected successful local login, got: {output}"
    );

    cleanup_hosts().await.expect("cleanup");
}

/// Verifies that the `login` PAM service rejects an incorrect password,
/// proving authentik is actually consulted rather than the module trivially
/// succeeding.
#[tokio::test(flavor = "multi_thread")]
async fn test_local_login_wrong_password_fails() {
    test_init();
    let tm = TestMachine::new().await.expect("test machine");
    join_domain(&tm).await.expect("join domain");

    let (exit_code, output) = exec_command(
        &tm.container,
        "printf '%s\\n' \"$AK_LOGIN_PW\" | pamtester login akadmin authenticate",
        &[("AK_LOGIN_PW", "definitely-not-the-password")],
    )
    .await
    .expect("exec pamtester");

    assert_ne!(exit_code, 0, "expected auth failure, got: {output}");

    cleanup_hosts().await.expect("cleanup");
}
