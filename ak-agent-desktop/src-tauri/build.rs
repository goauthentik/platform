fn main() {
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");

    windows_exe_info::icon::icon_ico("../../vpkg/windows/resources/icon.ico");
    if let Err(e) = windows_exe_info::versioninfo::VersionInfo::from_cargo_env_ex(
        Some("authentik Platform Agent"),
        Some("Authentik Security Inc."),
        Some("2026 Authentik Security Inc."),
        None,
    )
    .link()
    {
        println!("cargo::error={}", e);
    }

    tauri_build::build()
}
