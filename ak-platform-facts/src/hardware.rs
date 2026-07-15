use std::sync::{LazyLock, RwLock};

use authentik_client::models::HardwareRequest;
use eyre::Result;
use serde::Deserialize;

use crate::query::{non_empty, query_named};

/// Lets an embedding host supply the serial directly, bypassing the
/// osquery-based lookup. Checked unconditionally on every platform, though
/// today only macOS embedding contexts actually set it.
static STATIC_SERIAL: LazyLock<RwLock<Option<String>>> = LazyLock::new(|| RwLock::new(None));

/// Pass `None` to clear the override.
pub fn set_static_serial(serial: Option<String>) {
    if let Ok(mut guard) = STATIC_SERIAL.write() {
        *guard = serial;
    }
}

pub(crate) fn static_serial() -> Option<String> {
    STATIC_SERIAL.read().ok().and_then(|guard| guard.clone())
}

#[derive(Deserialize)]
struct SystemInfoRow {
    #[serde(default)]
    hardware_vendor: String,
    #[serde(default)]
    hardware_model: String,
    #[serde(default)]
    hardware_serial: String,
    #[serde(default)]
    cpu_brand: String,
    #[serde(default)]
    cpu_logical_cores: String,
    #[serde(default)]
    physical_memory: String,
}

fn system_info_row() -> Result<SystemInfoRow> {
    query_named::<SystemInfoRow>("system_info")?
        .into_iter()
        .next()
        .ok_or_else(|| eyre::eyre!("system_info returned no rows"))
}

fn native_serial(row: &SystemInfoRow) -> Result<String> {
    non_empty(row.hardware_serial.clone())
        .ok_or_else(|| eyre::eyre!("hardware_serial missing or empty in system_info"))
}

pub fn gather() -> Result<HardwareRequest> {
    let row = system_info_row()?;
    let serial = match static_serial() {
        Some(serial) => serial,
        None => native_serial(&row)?,
    };

    Ok(HardwareRequest {
        manufacturer: non_empty(row.hardware_vendor),
        model: non_empty(row.hardware_model),
        serial,
        cpu_name: non_empty(row.cpu_brand),
        cpu_count: row.cpu_logical_cores.parse::<i32>().ok(),
        memory_bytes: row.physical_memory.parse::<i64>().ok(),
    })
}

/// Also used for enrollment and JWT `iss` claims, where a full [`gather`]
/// would be wasteful.
pub fn serial() -> Result<String> {
    if let Some(serial) = static_serial() {
        return Ok(serial);
    }
    native_serial(&system_info_row()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gather_produces_positive_memory_and_cpu_count() {
        let hw = gather().unwrap();
        assert!(hw.memory_bytes.is_some_and(|b| b > 0));
        assert!(hw.cpu_count.is_some_and(|c| c > 0));
        assert!(!hw.serial.is_empty());
    }
}
