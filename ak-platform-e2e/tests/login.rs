use ak_tests::{
    authentik_creds, cleanup_hosts, exec_command, join_domain, must_exec, test_init, test_machine,
};

/// Verifies that a real local (non-SSH) login via the `login` PAM service is
/// authenticated against authentik, using `pamtester` to drive PAM without a
/// controlling TTY.
#[tokio::test]
async fn test_local_login_success() {
    test_init();
    let container = test_machine().await.expect("test machine");
    join_domain(&container).await.expect("join domain");

    let (_, password) = authentik_creds();
    let output = must_exec(
        &container,
        "printf '%s\\n' \"$AK_LOGIN_PW\" | pamtester login akadmin authenticate",
        &[("AK_LOGIN_PW", &password)],
    )
    .await
    .expect("pamtester authenticate");

    assert!(
        output.contains("successfully authenticated"),
        "expected successful local login, got: {output}"
    );

    cleanup_hosts().await.expect("cleanup");
}

/// Verifies that the `login` PAM service rejects an incorrect password,
/// proving authentik is actually consulted rather than the module trivially
/// succeeding.
#[tokio::test]
async fn test_local_login_wrong_password_fails() {
    test_init();
    let container = test_machine().await.expect("test machine");
    join_domain(&container).await.expect("join domain");

    let (exit_code, output) = exec_command(
        &container,
        "printf '%s\\n' \"$AK_LOGIN_PW\" | pamtester login akadmin authenticate",
        &[("AK_LOGIN_PW", "definitely-not-the-password")],
    )
    .await
    .expect("exec pamtester");

    assert_ne!(exit_code, 0, "expected auth failure, got: {output}");

    cleanup_hosts().await.expect("cleanup");
}
