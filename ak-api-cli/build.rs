use std::{env, fs, path::Path, path::PathBuf};

fn main() -> eyre::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    // Re-run when the dependency version changes. A version bump moves the git checkout to a new
    // path, so no `rerun-if-changed` on the checkout itself would ever fire.
    println!("cargo:rerun-if-changed=../Cargo.lock");

    let apis_dir = find_authentik_client_apis()?;
    println!("cargo:rerun-if-changed={}", apis_dir.display());

    let generated = ak_api_cli_gen::generate(&apis_dir).map_err(|e| eyre::eyre!("{e}"))?;

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    write_if_changed(&out_dir.join("api_commands.rs"), &generated.root)?;
    for (file_name, contents) in generated.modules {
        write_if_changed(&out_dir.join(file_name), &contents)?;
    }

    Ok(())
}

/// Codegen is deterministic, so an over-eager re-run (any `Cargo.lock` change triggers one) usually
/// produces identical bytes. Leaving the files untouched in that case keeps their mtimes stable, so
/// the 70k+ generated lines this crate `include!`s stay cached instead of being recompiled.
fn write_if_changed(path: &Path, contents: &str) -> eyre::Result<()> {
    if fs::read_to_string(path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }
    fs::write(path, contents)?;
    Ok(())
}

fn find_authentik_client_apis() -> eyre::Result<PathBuf> {
    let meta = cargo_metadata::MetadataCommand::new()
        .exec()
        .map_err(|e| eyre::eyre!("cargo metadata failed: {e}"))?;

    let pkg = meta
        .packages
        .iter()
        .find(|p| p.name == "authentik-client")
        .ok_or_else(|| eyre::eyre!("authentik-client not found in dependency graph"))?;

    let manifest_dir = pkg
        .manifest_path
        .parent()
        .ok_or_else(|| eyre::eyre!("invalid manifest_path for authentik-client"))?;

    Ok(manifest_dir.join("src/apis").into())
}
