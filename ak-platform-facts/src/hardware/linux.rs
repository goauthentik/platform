use authentik_client::models::HardwareRequest;
use eyre::{Result, bail};

fn dmi(name: &str) -> Option<String> {
    std::fs::read_to_string(format!("/sys/class/dmi/id/{name}"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn serial() -> Result<String> {
    if let Some(serial) = dmi("product_serial") {
        return Ok(serial);
    }
    let machine_id = std::fs::read_to_string("/etc/machine-id")
        .map_err(|e| eyre::eyre!("product_serial unavailable and /etc/machine-id unreadable: {e}"))?;
    let machine_id = machine_id.trim();
    if machine_id.is_empty() {
        bail!("/etc/machine-id is empty");
    }
    Ok(machine_id.to_string())
}

pub fn gather() -> Result<HardwareRequest> {
    let serial = serial()?;
    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_all();
    sys.refresh_memory();
    Ok(HardwareRequest {
        manufacturer: dmi("sys_vendor"),
        model: dmi("product_name"),
        serial,
        cpu_name: sys.cpus().first().map(|c| c.brand().to_string()),
        cpu_count: Some(sys.cpus().len() as i32),
        memory_bytes: Some(sys.total_memory() as i64),
    })
}
