use ak_macos_touchid::authenticate_with_touchid;

#[tokio::main]
async fn main() {
    let res = authenticate_with_touchid("foo".to_owned());
    eprintln!("Result: {res}");
}
