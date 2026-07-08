use ak_platform_e2e::{CmdTestCase, TestMachine, agent_setup, cmd_test, test_init};

#[tokio::test(flavor = "multi_thread")]
async fn test_agent_whoami() {
    test_init();
    let tm = TestMachine::new().await.expect("test machine");
    agent_setup(&tm).await.expect("agent setup");

    cmd_test(
        &tm,
        vec![CmdTestCase {
            name: "whoami".to_string(),
            cmd: "ak whoami".to_string(),
            expects: vec!["authentik Default Admin".to_string()],
        }],
    )
    .await
    .expect("cmd test");
}
