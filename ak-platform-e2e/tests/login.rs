#![cfg(unix)]

use ak_platform_e2e::{
    TestMachine, assert_no_apparmor_denials, authentik_creds, cleanup_hosts, exec_command,
    join_domain, must_exec, test_init,
};

/// Tests requiring an enterprise license; skipped by the `no-enterprise` nextest profile.
mod enterprise {
    use super::*;

    /// A full interactive `/bin/login` on a pty: auth against authentik plus
    /// session setup, ending in a shell as the authentik user.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_interactive_login_shell() {
        test_init();
        let tm = TestMachine::new().await.expect("test machine");
        join_domain(&tm).await.expect("join domain");

        // `script` gives `login` a pty. The marker only appears expanded when
        // the logged-in shell runs it, never in the input echo.
        let (_, password) = authentik_creds();
        let output = must_exec(
            &tm,
            "{ sleep 3; printf '%s\\n' \"$AK_LOGIN_PW\"; sleep 20; \
               printf 'echo LOGGED_IN_AS_$(id -un); exit\\n'; } \
             | script -qefc 'login akadmin' /dev/null",
            &[("AK_LOGIN_PW", &password)],
        )
        .await
        .expect("exec login");

        assert!(
            output.contains("LOGGED_IN_AS_akadmin"),
            "interactive login did not reach a shell: {output}"
        );
        assert_no_apparmor_denials(&tm)
            .await
            .expect("no apparmor denials");

        cleanup_hosts().await.expect("cleanup");
    }

    /// Verifies that a real local (non-SSH) login via the `login` PAM service is
    /// authenticated against authentik, using `pamtester` to drive PAM without a
    /// controlling TTY.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_local_login_success() {
        test_init();
        let tm = TestMachine::new().await.expect("test machine");
        join_domain(&tm).await.expect("join domain");

        let (_, password) = authentik_creds();
        let output = must_exec(
            &tm,
            "printf '%s\\n' \"$AK_LOGIN_PW\" | pamtester login akadmin authenticate",
            &[("AK_LOGIN_PW", &password)],
        )
        .await
        .expect("exec pamtester");

        assert!(
            output.contains("successfully authenticated"),
            "expected successful local login, got: {output}"
        );

        assert_no_apparmor_denials(&tm)
            .await
            .expect("no apparmor denials");

        cleanup_hosts().await.expect("cleanup");
    }
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
        &tm,
        "printf '%s\\n' \"$AK_LOGIN_PW\" | pamtester login akadmin authenticate",
        &[("AK_LOGIN_PW", "definitely-not-the-password")],
    )
    .await
    .expect("exec pamtester");

    assert_ne!(exit_code, 0, "expected auth failure, got: {output}");

    cleanup_hosts().await.expect("cleanup");
}
