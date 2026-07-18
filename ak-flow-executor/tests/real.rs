use ak_flow_executor::executor::FlowExecutor;
use ak_platform::log::init_log_interactive;

#[tokio::test]
async fn login() {
    init_log_interactive();
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
