extern crate windows_exe_info;

use std::env;

fn main() {
    windows_exe_info::icon::icon_ico("../vpkg/windows/resources/icon.ico");
    if let Err(e) = windows_exe_info::versioninfo::VersionInfo::from_cargo_env_ex(
        Some("authentik Platform System Service"),
        Some("Authentik Security Inc."),
        Some("2026 Authentik Security Inc."),
        None,
    )
    .link()
    {
        println!("cargo::error={}", e);
    }

    // ak-sysd links two full copies of sqlite3: osquery-sys force-loads its own
    // vendored copy (`--whole-archive`/`/WHOLEARCHIVE:`), and rusqlite's
    // `libsqlite3-sys` (feature `bundled`) links a second. That's a duplicate-symbol
    // link error on rust-lld (Linux) and MSVC link.exe (Windows); macOS's ld already
    // tolerates it, keeping the first (osquery's) copy — which rusqlite runs against
    // fine (see state::test::test_state). Tell the other two linkers to do the same.
    // Emitted via rustc-link-arg so it also covers this package's test binaries.
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    match (target_os.as_str(), target_env.as_str()) {
        ("linux", _) => println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition"),
        ("windows", "msvc") => println!("cargo:rustc-link-arg=/FORCE:MULTIPLE"),
        _ => {} // macOS already links (warns only); nothing to do
    }
}
