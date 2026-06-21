use ak_macos_touchid::{AccessRequest, authenticate_with_touchid};

#[tokio::main]
async fn main() {
    let res = authenticate_with_touchid(AccessRequest {
        requesting_app: "iTermServer-3.6.11".to_owned(),
        profile_name: "Jane Doe".to_owned(),
        profile_email: "jane@goauthentik.io".to_owned(),
        profile_username: "jdoe".to_owned(),
        profile_groups: "Engineering, Admins".to_owned(),
        reason: "iTermServer-3.6.11 wants to access your profile".to_owned(),
        ..Default::default()
    });
    eprintln!("Result: {res}");
}
