use ak_macos_touchid::{AuthRequest, authenticate_with_touchid};

#[tokio::main]
async fn main() {
    let res = authenticate_with_touchid(AuthRequest {
        reason: "foo".to_owned(),
    });
    eprintln!("Result: {res}");
}
