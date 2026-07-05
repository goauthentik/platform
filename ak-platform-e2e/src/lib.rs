use std::{env, fs, path::PathBuf, time::Duration};

use authentik_client::apis::{configuration::Configuration as AkConfig, endpoints_api};
use eyre::{Context, Result, bail};
use testcontainers::{
    ContainerAsync, GenericImage, ImageExt,
    core::{CgroupnsMode, ExecCommand, Host, Mount, logs::LogFrame},
    runners::AsyncRunner,
};
use uuid::Uuid;

pub fn test_init() {
    ak_platform::log::init_log_interactive();
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
    // Integration tests run from ak-tests/, so workspace root is one level up
    let cwd = env::current_dir().expect("cwd");
    let root = cwd.parent().unwrap_or(&cwd);
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
pub async fn agent_setup(container: &ContainerAsync<GenericImage>) -> Result<()> {
    let base_url = local_authentik_url();
    let auth_client = authenticated_session().await?;

    // Request a device code for the ak CLI application
    let device_resp = auth_client
        .post(format!("{}/application/o/device/", base_url))
        .form(&[
            ("client_id", "authentik-cli"),
            (
                "scope",
                "openid profile email offline_access goauthentik.io/api",
            ),
        ])
        .send()
        .await
        .wrap_err("device code request failed")?
        .json::<serde_json::Value>()
        .await
        .wrap_err("invalid device code response")?;

    let device_code = device_resp["device_code"]
        .as_str()
        .ok_or_else(|| eyre::eyre!("missing device_code"))?
        .to_string();

    let verification_uri = device_resp["verification_uri_complete"]
        .as_str()
        .or_else(|| device_resp["verification_uri"].as_str())
        .ok_or_else(|| eyre::eyre!("missing verification_uri"))?
        .to_string();

    // Visit the verification URI — this triggers the consent flow in authentik
    auth_client
        .get(&verification_uri)
        .send()
        .await
        .wrap_err("failed to visit verification URI")?;

    // Execute the implicit consent flow to auto-approve the device authorization
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

    // Poll the token endpoint until the device authorization is approved
    let mut access_token = String::new();
    let mut refresh_token = String::new();
    for _ in 0..12 {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let token_resp = auth_client
            .post(format!("{}/application/o/token/", base_url))
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", &device_code),
                ("client_id", "authentik-cli"),
            ])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        if let Some(at) = token_resp["access_token"].as_str() {
            access_token = at.to_string();
            refresh_token = token_resp["refresh_token"]
                .as_str()
                .unwrap_or("")
                .to_string();
            break;
        }
    }

    if access_token.is_empty() {
        bail!("device flow timed out — no tokens received");
    }

    // Configure the agent in the container using the obtained tokens
    let ak_url = container_authentik_url();
    let access_token_str = access_token.as_str();
    let refresh_token_str = refresh_token.as_str();
    must_exec(
        container,
        &format!("ak config setup -a {}", ak_url),
        &[
            ("AK_CLI_ACCESS_TOKEN", access_token_str),
            ("AK_CLI_REFRESH_TOKEN", refresh_token_str),
        ],
    )
    .await?;

    Ok(())
}

/// Enrolls a container in the authentik domain and waits for the akadmin user
/// to appear in the NSS database.
///
/// Call `cleanup_hosts().await` in your test's cleanup to remove the enrolled device.
pub async fn join_domain(container: &ContainerAsync<GenericImage>) -> Result<()> {
    let ak_url = container_authentik_url();
    must_exec(
        container,
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
        let (rc1, out1) = exec_command(container, "getent passwd", &[]).await?;
        if rc1 == 0 && out1.contains("akadmin") {
            let (rc2, out2) = exec_command(container, "getent passwd akadmin", &[]).await?;
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
    let mut config = AkConfig::new();
    config.base_path = format!("{}/api/v3", base_url);
    config.bearer_access_token = Some(authentik_token());

    let result =
        endpoints_api::endpoints_devices_list(&config, None, None, None, None, Some(100), None)
            .await
            .wrap_err("failed to list devices")?;

    let count = result.results.len();
    for device in result.results {
        if let Some(uuid) = device.device_uuid {
            endpoints_api::endpoints_devices_destroy(&config, &uuid.to_string())
                .await
                .wrap_err("failed to destroy device")?;
        }
    }
    eprintln!("Deleted {} devices", count);

    Ok(())
}

/// Creates and starts the e2e test machine container.
///
/// Waits for `ak-sysd version` to succeed before returning, matching the
/// Go harness's `wait.ForExec` condition.
pub async fn test_machine() -> Result<ContainerAsync<GenericImage>> {
    let host_coverage_dir = lookup_repo_dir("ak-platform-e2e/coverage");
    let cwd = env::current_dir().expect("cwd");
    let local_coverage_dir = cwd
        .parent()
        .unwrap_or(&cwd)
        .join("../ak-platform-e2e/coverage");

    for sub in &["cli", "ak-sysd", "ak-agent", "rs"] {
        let fd = local_coverage_dir.join(sub);
        tracing::debug!(
            path = fd.to_string_lossy().to_string(),
            "Creating coverage dir"
        );
        fs::create_dir_all(fd).wrap_err(format!("failed to create coverage subdir '{}'", sub))?;
    }

    let hostname = format!("test-machine-{}", Uuid::new_v4());
    let host_coverage_str = host_coverage_dir.to_string_lossy().into_owned();

    let container = GenericImage::new("xghcr.io/goauthentik/platform-e2e", "local")
        .with_env_var("GOCOVERDIR", "/tmp/ak-coverage/cli")
        .with_env_var(
            "LLVM_PROFILE_FILE",
            "/tmp/ak-coverage/rs/default_%m_%p.profraw",
        )
        .with_hostname(hostname)
        .with_user("root")
        .with_privileged(true)
        .with_cgroupns_mode(CgroupnsMode::Host)
        .with_mount(Mount::bind_mount("/sys/fs/cgroup", "/sys/fs/cgroup"))
        .with_mount(Mount::bind_mount(&host_coverage_str, "/tmp/ak-coverage"))
        .with_host("host.docker.internal", Host::HostGateway)
        .with_log_consumer(|frame: &LogFrame| {
            let line = String::from_utf8_lossy(frame.bytes());
            eprint!("[e2e] {}", line);
        })
        .start()
        .await
        .wrap_err("failed to start e2e container")?;

    // Wait for ak-sysd to be healthy (mirrors Go's wait.ForExec)
    for attempt in 0..20 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        let (exit_code, _) = exec_command(&container, "/usr/bin/ak-sysd version", &[]).await?;
        if exit_code == 0 {
            return Ok(container);
        }
    }

    bail!("ak-sysd not ready after 60 seconds")
}

/// Executes a shell command in a container, returning (exit_code, stdout).
pub async fn exec_command(
    container: &ContainerAsync<GenericImage>,
    cmd: &str,
    env_vars: &[(&str, &str)],
) -> Result<(i64, String)> {
    eprintln!("[exec] {}", cmd);
    let exec_cmd = ExecCommand::new(["sh", "-c", cmd])
        .with_env_vars(env_vars.iter().map(|(k, v)| (k.to_string(), v.to_string())));

    let mut result = container
        .exec(exec_cmd)
        .await
        .wrap_err(format!("exec failed: '{}'", cmd))?;

    let stdout_bytes = result.stdout_to_vec().await?;
    let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
    let exit_code = result.exit_code().await?.unwrap_or(-1);
    eprintln!("[exec] exit={} output={:?}", exit_code, stdout.trim());

    Ok((exit_code, stdout))
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
pub async fn cmd_test(
    container: &ContainerAsync<GenericImage>,
    cases: Vec<CmdTestCase>,
) -> Result<()> {
    for case in cases {
        let output = must_exec(container, &case.cmd, &[]).await?;
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
