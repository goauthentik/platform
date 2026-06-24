use std::{env, path::PathBuf};

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    // Re-run when the dependency version changes.
    println!("cargo:rerun-if-changed=../Cargo.lock");

    let apis_dir = find_authentik_client_apis()?;
    let code = ak_api_cli_gen::generate(&apis_dir).map_err(|e| anyhow::anyhow!("{e}"))?;

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    std::fs::write(out_dir.join("api_commands.rs"), code)?;

    Ok(())
}

fn find_authentik_client_apis() -> anyhow::Result<PathBuf> {
    let meta = cargo_metadata::MetadataCommand::new()
        .exec()
        .map_err(|e| anyhow::anyhow!("cargo metadata failed: {e}"))?;

    let pkg = meta
        .packages
        .iter()
        .find(|p| p.name == "authentik-client")
        .ok_or_else(|| anyhow::anyhow!("authentik-client not found in dependency graph"))?;

    let manifest_dir = pkg
        .manifest_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid manifest_path for authentik-client"))?;

    Ok(manifest_dir.join("src/apis").into())
}
