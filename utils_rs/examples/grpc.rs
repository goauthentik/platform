
fn main() {
    let resp: String = authentik_sys::generated::grpc_request(async |ch| {
        return Ok(authentik_sys::generated::ping::ping_client::PingClient::new(ch)
            .ping(())
            .await?);
    }).unwrap().into_inner().version;
    eprintln!("{}", resp);
}
