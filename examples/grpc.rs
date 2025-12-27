use authentik_sys::{generated::ping::ping_client::PingClient, grpc::grpc_request};

fn main() {
    let resp: String = grpc_request(async |ch| {
        return Ok(PingClient::new(ch).ping(()).await?);
    })
    .unwrap()
    .into_inner()
    .version;
    eprintln!("{}", resp);
}
