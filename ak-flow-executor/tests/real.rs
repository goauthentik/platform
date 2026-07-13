use ak_flow_executor::executor::FlowExecutor;
use ak_platform::log::init_log_interactive;
use authentik_client::apis::configuration::Configuration;

#[tokio::test]
async fn login() {
    init_log_interactive();
    let mut ref_config = Configuration::new();
    ref_config.base_path = "http://localhost:9000/api/v3".to_string();
    let mut fe = FlowExecutor::builder()
        .flow("default-authentication-flow")
        .reference_config(ref_config)
        .with_answer("ak-stage-identification", "akadmin")
        .set_secrets("foo", false)
        .build()
        .await
        .unwrap();
    fe.execute().await.unwrap();
}
