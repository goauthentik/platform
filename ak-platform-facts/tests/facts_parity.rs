//! Cross-checks the Go (`goauthentik.io/platform/pkg/platform/facts`, run via
//! `ak-sysd troubleshoot facts`) and Rust (this crate, run via the
//! `facts_print` example) device-facts gatherers against each other on the
//! same host. They're independent implementations (different libraries per
//! platform), so this only asserts on fields meant to be identical --
//! process/user/disk/software lists are point-in-time snapshots from two
//! separate processes and aren't expected to match exactly.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("ak-platform-facts is a workspace member, not the repo root")
        .to_path_buf()
}

fn run_json(mut cmd: Command, label: &str) -> Value {
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {label}: {e}"));
    assert!(
        output.status.success(),
        "{label} exited with {}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "{label} did not print valid JSON: {e}\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

#[test]
fn go_and_rust_facts_agree() {
    let root = repo_root();

    let mut go_cmd = Command::new("go");
    go_cmd
        .args(["run", "-v", "./cmd/agent_system/", "troubleshoot", "facts"])
        .current_dir(&root);
    let go_facts = run_json(go_cmd, "go troubleshoot facts");

    let mut rust_cmd = Command::new("cargo");
    rust_cmd
        .args([
            "run",
            "--quiet",
            "-p",
            "ak-platform-facts",
            "--example",
            "facts_print",
        ])
        .current_dir(&root);
    let rust_facts = run_json(rust_cmd, "cargo run --example facts_print");

    assert_eq!(
        go_facts["network"]["hostname"], rust_facts["network"]["hostname"],
        "hostname mismatch"
    );
    assert_eq!(
        go_facts["os"]["family"], rust_facts["os"]["family"],
        "os family mismatch"
    );
    assert_eq!(
        go_facts["os"]["arch"], rust_facts["os"]["arch"],
        "os arch mismatch (ak_platform_facts::os::go_style_arch should keep these in sync)"
    );

    // Hardware facts require querying the platform's real serial/DMI data,
    // which virtualized CI runners frequently don't expose. Go tolerates
    // that with an empty `serial` string; Rust's hardware::gather() treats
    // a missing serial as a hard error, so the whole `hardware` section is
    // absent there instead (see ak-platform-facts/src/hardware.rs). Only
    // compare when both sides actually produced a hardware section.
    let go_hw = go_facts.get("hardware").filter(|v| !v.is_null());
    let rust_hw = rust_facts.get("hardware").filter(|v| !v.is_null());
    if let (Some(go_hw), Some(rust_hw)) = (go_hw, rust_hw) {
        assert_eq!(
            go_hw["cpu_count"], rust_hw["cpu_count"],
            "logical cpu count mismatch"
        );

        if let (Some(go_mem), Some(rust_mem)) = (
            go_hw["memory_bytes"].as_i64(),
            rust_hw["memory_bytes"].as_i64(),
        ) {
            // Different query mechanisms per platform (sysctl/WMI/procfs on
            // the Go side, osquery's system_info on the Rust side) can round
            // "total installed memory" slightly differently -- allow some
            // slack rather than requiring a bit-exact match.
            let diff = (go_mem - rust_mem).abs() as f64;
            let tolerance = go_mem as f64 * 0.1;
            assert!(
                diff <= tolerance,
                "total memory mismatch: go={go_mem} rust={rust_mem}"
            );
        }

        let go_serial = go_hw["serial"].as_str().filter(|s| !s.is_empty());
        let rust_serial = rust_hw["serial"].as_str().filter(|s| !s.is_empty());
        if let (Some(go_serial), Some(rust_serial)) = (go_serial, rust_serial) {
            assert_eq!(go_serial, rust_serial, "hardware serial mismatch");
        }
    }
}
