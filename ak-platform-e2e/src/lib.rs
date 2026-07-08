use std::{env, path::PathBuf, time::Duration};

use authentik_client::apis::{configuration::Configuration as AkConfig, endpoints_api};
use eyre::{Context, ContextCompat, Result, bail};
use oauth_device_flows::provider::GenericProviderConfig;
use oauth_device_flows::{DeviceFlow, DeviceFlowConfig, Provider};
use testcontainers::core::CmdWaitFor;
use testcontainers::{ContainerAsync, GenericImage, core::ExecCommand};
use url::Url;

pub mod test_machine;
pub use test_machine::TestMachine;

pub fn test_init() {
    ak_platform::log::init_log_interactive_with_filter(Some("warn,ak_platform_e2e=trace"));
    ak_platform::log::set_log_level(ak_platform::log::LevelFilter::Debug);
}

pub fn local_authentik_url() -> String {
    if env::var("CI").as_deref() == Ok("true") {
        env::var("AK_URL").unwrap_or_else(|_| "http://localhost:9000".to_string())
    } else {
        "http://host.docker.internal:9123".to_string()
    }
}

pub fn container_authentik_url() -> String {
    if env::var("CI").as_deref() == Ok("true") {
        "http://host.docker.internal:9000".to_string()
    } else {
        "http://host.docker.internal:9123".to_string()
    }
}

pub fn authentik_creds() -> (String, String) {
    if env::var("CI").as_deref() == Ok("true") {
        (
            "akadmin".to_string(),
            env::var("AK_PASSWORD").unwrap_or_default(),
        )
    } else {
        (
            "akadmin".to_string(),
            "this-password-is-for-testing-dont-use".to_string(),
        )
    }
}

pub fn authentik_token() -> String {
    if env::var("CI").as_deref() == Ok("true") {
        env::var("AK_TOKEN").unwrap_or_default()
    } else {
        "this-token-is-for-testing-dont-use".to_string()
    }
}

/// Returns a host-local path for a repo-relative directory, respecting LOCAL_WORKSPACE.
pub fn lookup_repo_dir(rel: &str) -> PathBuf {
    let rel_clean = rel.trim_start_matches('/');
    if let Ok(lw) = env::var("LOCAL_WORKSPACE") {
        return PathBuf::from(lw).join(rel_clean);
    }
    // CARGO_MANIFEST_DIR points to ak-platform-e2e/ at compile time; its parent
    // is the workspace root regardless of where cargo-nextest sets the runtime CWD.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().unwrap_or(&manifest);
    root.join(rel_clean)
}

/// Creates a reqwest::Client with an active authentik session cookie by
/// executing the default-authentication-flow step-by-step.
pub async fn authenticated_session() -> Result<reqwest::Client> {
    let base_url = local_authentik_url();
    let (username, password) = authentik_creds();

    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .wrap_err("failed to build HTTP client")?;

    let flow_url = format!(
        "{}/api/v3/flows/executor/default-authentication-flow/?format=json",
        base_url
    );

    let mut challenge = client
        .get(&flow_url)
        .send()
        .await
        .wrap_err("failed to start authentication flow")?
        .json::<serde_json::Value>()
        .await
        .wrap_err("failed to parse flow challenge")?;

    // Walk through the flow steps (identification → password → redirect)
    loop {
        let component = challenge["component"].as_str().unwrap_or("").to_string();
        let body = if component.contains("identification") {
            serde_json::json!({ "component": component, "uid_field": username })
        } else if component.contains("password") {
            serde_json::json!({ "component": component, "password": password })
        } else {
            break; // redirect or unknown — authentication complete
        };

        let resp = client
            .post(&flow_url)
            .json(&body)
            .send()
            .await
            .wrap_err("flow step failed")?;

        // A non-JSON redirect response means success
        let ct = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !ct.contains("json") {
            break;
        }

        challenge = resp.json::<serde_json::Value>().await?;
        let t = challenge["type"].as_str().unwrap_or("");
        let c = challenge["component"].as_str().unwrap_or("");
        if t == "redirect" || c == "xak-flow-redirect" {
            break;
        }
    }

    Ok(client)
}

/// Sets up the ak CLI agent in a container.
///
/// Uses the OAuth device flow with the authenticated session auto-approving the
/// consent, then sets AK_CLI_ACCESS_TOKEN / AK_CLI_REFRESH_TOKEN and runs
/// `ak config setup` in the container.
pub async fn agent_setup(tm: &TestMachine) -> Result<()> {
    let base_url = local_authentik_url();
    let mut base = Url::parse(&base_url).wrap_err("invalid authentik URL")?;
    if !base.path().ends_with('/') {
        base.set_path(&format!("{}/", base.path()));
    }

    let config = DeviceFlowConfig::new()
        .client_id("authentik-cli")
        .scopes(vec![
            "openid",
            "profile",
            "email",
            "offline_access",
            "goauthentik.io/api",
        ])
        .poll_interval(Duration::from_secs(5))
        .generic_provider(GenericProviderConfig::new(
            base.join("application/o/device/")
                .wrap_err("invalid device URL")?,
            base.join("application/o/token/")
                .wrap_err("invalid token URL")?,
            "authentik".to_owned(),
        ))
        .max_attempts(12);

    let mut device_flow =
        DeviceFlow::new(Provider::Generic, config).wrap_err("failed to create device flow")?;

    let verification_uri = {
        let auth_response = device_flow
            .initialize()
            .await
            .wrap_err("device flow initialization failed")?;
        auth_response
            .verification_uri_complete()
            .unwrap_or_else(|| auth_response.verification_uri())
            .clone()
    };

    // Auto-approve: visit the verification URI with an authenticated session,
    // then submit the implicit consent form.
    let auth_client = authenticated_session().await?;
    auth_client
        .get(verification_uri.as_str())
        .send()
        .await
        .wrap_err("failed to visit verification URI")?;

    let consent_url = format!(
        "{}/api/v3/flows/executor/default-provider-authorization-implicit-consent/?format=json",
        base_url
    );
    if let Ok(resp) = auth_client.get(&consent_url).send().await
        && let Ok(challenge) = resp.json::<serde_json::Value>().await
    {
        let component = challenge["component"].as_str().unwrap_or("").to_string();
        let _ = auth_client
            .post(&consent_url)
            .json(&serde_json::json!({ "component": component }))
            .send()
            .await;
    }

    let token_response = device_flow
        .poll_for_token()
        .await
        .wrap_err("device flow polling failed")?;

    let ak_url = container_authentik_url();
    let access_token = token_response.access_token().to_owned();
    let refresh_token = token_response.refresh_token().unwrap_or("").to_owned();
    must_exec(
        &tm.container,
        &format!("ak config setup -a {}", ak_url),
        &[
            ("AK_CLI_ACCESS_TOKEN", &access_token),
            ("AK_CLI_REFRESH_TOKEN", &refresh_token),
        ],
    )
    .await?;

    Ok(())
}

/// Enrolls a container in the authentik domain and waits for the akadmin user
/// to appear in the NSS database.
///
/// Call `cleanup_hosts().await` in your test's cleanup to remove the enrolled device.
pub async fn join_domain(tm: &TestMachine) -> Result<()> {
    let ak_url = container_authentik_url();
    must_exec(
        &tm.container,
        &format!("ak-sysd domains join ak -a {}", ak_url),
        &[("AK_SYS_INSECURE_ENV_TOKEN", "test-enroll-key")],
    )
    .await
    .wrap_err("ak-sysd domains join failed")?;

    // Retry until akadmin appears in the NSS database (NSS module may take a moment)
    for attempt in 0..20 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        let (rc1, out1) = exec_command(&tm.container, "getent passwd", &[]).await?;
        if rc1 == 0 && out1.contains("akadmin") {
            let (rc2, out2) = exec_command(&tm.container, "getent passwd akadmin", &[]).await?;
            if rc2 == 0 && out2.contains("akadmin") {
                return Ok(());
            }
        }
    }

    bail!("akadmin not found in passwd after 20 attempts (~40 s)")
}

/// Removes all enrolled devices from authentik via the admin API.
pub async fn cleanup_hosts() -> Result<()> {
    let base_url = local_authentik_url();
    let token = authentik_token();

    // Use raw reqwest to avoid generated-client serde failures on nullable UUID fields.
    let body: serde_json::Value = reqwest::Client::new()
        .get(format!("{}/api/v3/endpoints/devices/", base_url))
        .query(&[("page_size", "100")])
        .bearer_auth(&token)
        .send()
        .await
        .wrap_err("failed to list devices")?
        .error_for_status()
        .wrap_err("failed to list devices")?
        .json()
        .await
        .wrap_err("failed to parse device list")?;

    let results = body["results"]
        .as_array()
        .wrap_err("invalid device list response")?;
    let count = results.len();

    let mut config = AkConfig::new();
    config.base_path = format!("{}/api/v3", base_url);
    config.bearer_access_token = Some(token);

    for device in results {
        if let Some(uuid) = device["device_uuid"].as_str() {
            endpoints_api::endpoints_devices_destroy(&config, uuid)
                .await
                .wrap_err("failed to destroy device")?;
        }
    }
    tracing::info!("Deleted {} devices", count);

    Ok(())
}

/// Executes a shell command in a container, returning (exit_code, stdout).
pub async fn exec_command(
    container: &ContainerAsync<GenericImage>,
    cmd: &str,
    env_vars: &[(&str, &str)],
) -> Result<(i64, String)> {
    tracing::info!("[exec] {}", cmd);
    let exec_cmd = ExecCommand::new(["sh", "-c", cmd])
        .with_env_vars(env_vars.iter().map(|(k, v)| (k.to_string(), v.to_string())))
        .with_cmd_ready_condition(CmdWaitFor::Exit { code: None });

    let mut result = container
        .exec(exec_cmd)
        .await
        .wrap_err(format!("exec failed: '{}'", cmd))?;

    let exit_code = result.exit_code().await.unwrap().unwrap();
    tracing::info!("[exec] {} exit={}", cmd, exit_code);
    let stdout_str = String::from_utf8_lossy(&result.stdout_to_vec().await?).into_owned();
    let stderr_str = String::from_utf8_lossy(&result.stderr_to_vec().await?).into_owned();
    stdout_str
        .lines()
        .for_each(|l| tracing::info!("[stdout] {}", l));
    stderr_str
        .lines()
        .for_each(|l| tracing::warn!("[stderr] {}", l));

    let output = format!("{}{}", stdout_str, stderr_str);

    Ok((exit_code, output))
}

/// Executes a shell command in a container, returning stdout or an error on
/// non-zero exit code.
pub async fn must_exec(
    container: &ContainerAsync<GenericImage>,
    cmd: &str,
    env_vars: &[(&str, &str)],
) -> Result<String> {
    let (exit_code, output) = exec_command(container, cmd, env_vars).await?;
    if exit_code != 0 {
        bail!(
            "command '{}' exited with code {}: {}",
            cmd,
            exit_code,
            output
        );
    }
    Ok(output)
}

/// A single parameterized command test case.
pub struct CmdTestCase {
    pub name: String,
    pub cmd: String,
    pub expects: Vec<String>,
}

/// Runs a series of command test cases against a container, asserting that each
/// expected string is present in the command's output.
pub async fn cmd_test(tm: &TestMachine, cases: Vec<CmdTestCase>) -> Result<()> {
    for case in cases {
        let output = must_exec(&tm.container, &case.cmd, &[]).await?;
        for expected in &case.expects {
            assert!(
                output.contains(expected.as_str()),
                "test '{}' (cmd: {}): expected {:?} in output {:?}",
                case.name,
                case.cmd,
                expected,
                output,
            );
        }
    }
    Ok(())
}
