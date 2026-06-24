include!(concat!(env!("OUT_DIR"), "/api_commands.rs"));

#[cfg(test)]
mod tests {
    // Smoke tests for the generated CLI commands.
    // The primary test is that the crate compiles — if the generator emits invalid Rust,
    // the build step fails before these tests run. These tests add runtime assertions.

    #[test]
    fn api_modules_non_empty() {
        assert!(
            !super::API_MODULES.is_empty(),
            "expected at least one API module"
        );
    }

    #[test]
    fn core_module_present() {
        assert!(
            super::API_MODULES.contains(&"core"),
            "core module missing from API_MODULES"
        );
    }

    #[test]
    fn admin_module_present() {
        assert!(
            super::API_MODULES.contains(&"admin"),
            "admin module missing from API_MODULES"
        );
    }

    #[test]
    fn flows_module_present() {
        assert!(
            super::API_MODULES.contains(&"flows"),
            "flows module missing from API_MODULES"
        );
    }

    #[test]
    fn function_count_reasonable() {
        assert!(
            super::API_FUNCTION_COUNT > 100,
            "expected more than 100 API functions, found {}",
            super::API_FUNCTION_COUNT
        );
    }
}
