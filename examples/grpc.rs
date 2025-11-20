use authentik_sys::generated::{grpc_request, ping::ping_client::PingClient};

fn main() {
    let resp: String = grpc_request(async |ch| {
        return Ok(PingClient::new(ch).ping(()).await?);
    })
    .unwrap()
    .into_inner()
    .version;
    eprintln!("{}", resp);
}
