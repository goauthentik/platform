//! Clap command tree for the authentik API, generated at build time by `ak-api-cli-gen` from the
//! `authentik-client` sources.
//!
//! This lives in its own crate purely for compile time: the generated surface is ~70k lines with
//! ~2k clap derive expansions, and keeping it out of `ak-cli` means editing the hand-written CLI
//! doesn't re-expand and re-check all of it.

include!(concat!(env!("OUT_DIR"), "/api_commands.rs"));

#[cfg(test)]
mod tests {
    // Smoke tests for the generated CLI commands.
    // The primary test is that the crate compiles — if the generator emits invalid Rust,
    // the build step fails before these tests run. These tests add runtime assertions.

    #[test]
    fn api_modules_non_empty() {
        assert!(
            !crate::API_MODULES.is_empty(),
            "expected at least one API module"
        );
    }

    #[test]
    fn core_module_present() {
        assert!(
            crate::API_MODULES.contains(&"core"),
            "core module missing from API_MODULES"
        );
    }

    #[test]
    fn admin_module_present() {
        assert!(
            crate::API_MODULES.contains(&"admin"),
            "admin module missing from API_MODULES"
        );
    }

    #[test]
    fn flows_module_present() {
        assert!(
            crate::API_MODULES.contains(&"flows"),
            "flows module missing from API_MODULES"
        );
    }

    #[test]
    fn function_count_reasonable() {
        assert!(
            crate::API_FUNCTION_COUNT > 100,
            "expected more than 100 API functions, found {}",
            crate::API_FUNCTION_COUNT
        );
    }
}
