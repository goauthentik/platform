use authentik_client::models::HardwareRequest;
use eyre::Result;
use serde::Deserialize;
use wmi::{COMLibrary, WMIConnection};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32ComputerSystem {
    manufacturer: Option<String>,
    model: Option<String>,
    number_of_logical_processors: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32Bios {
    serial_number: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32PhysicalMemory {
    capacity: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32Processor {
    name: Option<String>,
}

fn wmi_serial(con: &WMIConnection) -> Result<Option<String>> {
    let results: Vec<Win32Bios> = con.query()?;
    Ok(results
        .into_iter()
        .find_map(|b| b.serial_number)
        .filter(|s| !s.trim().is_empty()))
}

fn registry_machine_guid() -> Result<String> {
    let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    let key = hklm.open_subkey("SOFTWARE\\Microsoft\\Cryptography")?;
    Ok(key.get_value("MachineGuid")?)
}

pub fn serial() -> Result<String> {
    let con = WMIConnection::new(COMLibrary::new()?)?;
    if let Some(serial) = wmi_serial(&con)? {
        return Ok(serial);
    }
    registry_machine_guid()
}

pub fn gather() -> Result<HardwareRequest> {
    let con = WMIConnection::new(COMLibrary::new()?)?;

    let computer_system: Option<Win32ComputerSystem> = con.query()?.into_iter().next();
    let processor: Option<Win32Processor> = con.query()?.into_iter().next();
    let memory: Vec<Win32PhysicalMemory> = con.query()?;
    let memory_bytes: u64 = memory.iter().filter_map(|m| m.capacity).sum();

    let serial = match wmi_serial(&con)? {
        Some(serial) => serial,
        None => registry_machine_guid()?,
    };

    Ok(HardwareRequest {
        manufacturer: computer_system.as_ref().and_then(|c| c.manufacturer.clone()),
        model: computer_system.as_ref().and_then(|c| c.model.clone()),
        serial,
        cpu_name: processor.and_then(|c| c.name),
        cpu_count: computer_system.and_then(|c| c.number_of_logical_processors.map(|n| n as i32)),
        memory_bytes: (memory_bytes > 0).then_some(memory_bytes as i64),
    })
}
