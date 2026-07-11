use authentik_client::models::HardwareRequest;
use eyre::{Context, Result};
use serde::Deserialize;

use crate::util::run;

#[derive(Deserialize, Default)]
struct SpHardwareEntry {
    serial_number: Option<String>,
    machine_model: Option<String>,
}

#[derive(Deserialize, Default)]
struct SpHardwareDataType {
    #[serde(rename = "SPHardwareDataType", default)]
    entries: Vec<SpHardwareEntry>,
}

fn profile() -> Result<SpHardwareEntry> {
    let out = run(std::process::Command::new("system_profiler").args(["-json", "SPHardwareDataType"]))?;
    let parsed: SpHardwareDataType =
        serde_json::from_str(&out).wrap_err("failed to parse system_profiler output")?;
    parsed
        .entries
        .into_iter()
        .next()
        .ok_or_else(|| eyre::eyre!("system_profiler returned no SPHardwareDataType entries"))
}

pub fn serial() -> Result<String> {
    if let Some(serial) = super::static_serial() {
        return Ok(serial);
    }
    profile()?
        .serial_number
        .ok_or_else(|| eyre::eyre!("serial_number missing from system_profiler output"))
}

pub fn gather() -> Result<HardwareRequest> {
    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_all();
    sys.refresh_memory();

    // model/cpu_name stay unset here — they're only ever populated from
    // the system_profiler response below, which this path skips.
    if let Some(serial) = super::static_serial() {
        return Ok(HardwareRequest {
            manufacturer: Some("Apple Inc.".to_string()),
            model: None,
            serial,
            cpu_name: None,
            cpu_count: Some(sys.cpus().len() as i32),
            memory_bytes: Some(sys.total_memory() as i64),
        });
    }

    let profile = profile()?;
    Ok(HardwareRequest {
        manufacturer: Some("Apple Inc.".to_string()),
        model: profile.machine_model,
        serial: profile
            .serial_number
            .ok_or_else(|| eyre::eyre!("serial_number missing from system_profiler output"))?,
        cpu_name: sys.cpus().first().map(|c| c.brand().to_string()),
        cpu_count: Some(sys.cpus().len() as i32),
        memory_bytes: Some(sys.total_memory() as i64),
    })
}
