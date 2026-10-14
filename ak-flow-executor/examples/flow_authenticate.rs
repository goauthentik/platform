use ak_flow_executor::executor::FlowExecutor;
use ak_platform::{log::LogBuilder, string::PlatformString};

#[tokio::main]
async fn main() {
    LogBuilder::new(PlatformString::new())
        .force_stdout(true)
        .enable();
    let mut fe = FlowExecutor::builder()
        .flow("default-authentication-flow")
        .base_url("http://localhost:9000/api/v3")
        .with_answer("ak-stage-identification", "akadmin")
        .set_secrets("foo", false)
        .build()
        .await
        .unwrap();
    fe.execute().await.unwrap();
}
