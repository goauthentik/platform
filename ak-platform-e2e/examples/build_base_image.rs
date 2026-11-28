//! Builds the base VM image ahead of the e2e tests (`make test-e2e-image`).
//! Without it, the first `TestMachine::new()` builds the image.

#[cfg(unix)]
#[tokio::main]
async fn main() -> eyre::Result<()> {
    ak_platform_e2e::test_init();
    let alias = ak_platform_e2e::test_machine::base_image().await?;
    println!("{alias}");
    Ok(())
}

#[cfg(not(unix))]
fn main() {}
