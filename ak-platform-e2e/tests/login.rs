#![cfg(unix)]

use std::time::Duration;

use ak_platform_e2e::{
    TestMachine, assert_no_apparmor_denials, authentik_creds, cleanup_hosts, exec_command,
    join_domain, must_exec, test_init,
};

/// Tests requiring an enterprise license; skipped by the `no-enterprise` nextest profile.
mod enterprise {
    use super::*;

    /// A full interactive login on the VM's serial console (getty + `login`):
    /// auth against authentik plus session setup, ending in a shell as the
    /// authentik user.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_interactive_login_shell() {
        test_init();
        let tm = TestMachine::new().await.expect("test machine");
        join_domain(&tm).await.expect("join domain");

        // Log in on the serial console, the way a person at the machine
        // would: getty prompts for the user, then `login` runs PAM. The
        // marker only appears expanded when the logged-in shell runs it,
        // never in the input echo.
        let (_, password) = authentik_creds();
        let wait = Duration::from_secs(60);
        let mut console = tm.console().await.expect("attach console");
        console.send("\r").await.expect("send");
        console
            .expect(&["login: "], wait)
            .await
            .expect("getty prompt");
        console.send("akadmin\r").await.expect("send");
        console
            .expect(&["assword:"], wait)
            .await
            .expect("password prompt");
        console.send(&format!("{password}\r")).await.expect("send");
        let outcome = console
            .expect(&["$ ", "# ", "Login incorrect"], wait)
            .await
            .expect("shell prompt");
        assert_ne!(outcome, 2, "login rejected:\n{}", console.transcript());
        console
            .send("echo LOGGED_IN_AS_$(id -un)\r")
            .await
            .expect("send");
        console
            .expect(&["LOGGED_IN_AS_akadmin"], wait)
            .await
            .expect("shell runs as akadmin");
        console.send("exit\r").await.expect("send");

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
