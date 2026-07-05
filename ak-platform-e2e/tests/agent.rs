use ak_tests::{CmdTestCase, agent_setup, cmd_test, test_init, test_machine};

#[tokio::test]
async fn test_agent_whoami() {
    test_init();
    let container = test_machine().await.expect("test machine");
    agent_setup(&container).await.expect("agent setup");

    cmd_test(
        &container,
        vec![CmdTestCase {
            name: "whoami".to_string(),
            cmd: "ak whoami".to_string(),
            expects: vec!["authentik Default Admin".to_string()],
        }],
    )
    .await
    .expect("cmd test");
}
