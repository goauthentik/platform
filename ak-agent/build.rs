extern crate windows_exe_info;

fn main() {
    #[cfg(windows)]
    {
        windows_exe_info::icon::icon_ico("vpkg/windows/resources/icon.ico");
        windows_exe_info::versioninfo::VersionInfo::from_cargo_env_ex(
            Some("authentik Platform Agent"),
            Some("Authentik Security Inc."),
            Some("2026 Authentik Security Inc."),
            None,
        )
        .link()
        .unwrap();
    }
}
