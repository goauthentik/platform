use ak_platform::client::sysd::Client;
use ak_platform::paths::SysdSocketID;
use eyre::Result;

pub async fn print_version() -> Result<()> {
    let client = Client::new(SysdSocketID::Default).await?;
    let remote = client.ping().ping(()).await?.into_inner();

    println!("authentik System Agent: {}", ak_meta::full_version());
    println!("System: {}", remote.version);
    Ok(())
}
