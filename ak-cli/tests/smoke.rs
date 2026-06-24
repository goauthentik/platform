// Smoke tests for the generated CLI commands.
// The primary test is that the crate compiles — if the generator emits invalid Rust,
// the build step fails before these tests run. These tests add runtime assertions.

#[test]
fn api_modules_non_empty() {
    assert!(
        !ak_api_cli::API_MODULES.is_empty(),
        "expected at least one API module"
    );
}

#[test]
fn core_module_present() {
    assert!(
        ak_api_cli::API_MODULES.contains(&"core"),
        "core module missing from API_MODULES"
    );
}

#[test]
fn admin_module_present() {
    assert!(
        ak_api_cli::API_MODULES.contains(&"admin"),
        "admin module missing from API_MODULES"
    );
}

#[test]
fn flows_module_present() {
    assert!(
        ak_api_cli::API_MODULES.contains(&"flows"),
        "flows module missing from API_MODULES"
    );
}

#[test]
fn function_count_reasonable() {
    assert!(
        ak_api_cli::API_FUNCTION_COUNT > 100,
        "expected more than 100 API functions, found {}",
        ak_api_cli::API_FUNCTION_COUNT
    );
}
