use ak_platform::client::sysd::Client;
use ak_platform::generated::sys_ctrl::TroubleshootInspectResponse;
use ak_platform::paths::SysdSocketID;
use ak_platform::tui::{TreeNode, value_to_tree_node};
use eyre::Result;
use std::io::IsTerminal;

pub async fn check() -> Result<()> {
    crate::check::run_checks().await
}

pub async fn facts() -> Result<()> {
    let facts = ak_platform_facts::gather();
    if std::io::stdout().is_terminal() {
        let value = serde_json::to_value(&facts)?;
        for line in ak_platform::tui::render_map_as_tree(&value, "Facts:") {
            println!("{line}");
        }
    } else {
        println!("{}", serde_json::to_string(&facts)?);
    }
    Ok(())
}

/// Recursively renders `TroubleshootInspectResponse.children` as nested
/// subtrees, mirroring Go's `renderInspectAsTree`
/// (`pkg/agent_system/cli/troubleshoot_inspect.go`).
fn inspect_to_tree_node(r: &TroubleshootInspectResponse) -> TreeNode {
    let mut node = TreeNode::new(r.bucket.clone());
    let mut kv: Vec<_> = r.kv.iter().collect();
    kv.sort_by(|a, b| a.0.cmp(b.0));
    for (k, v) in kv {
        node.push_child(value_to_tree_node(k, &serde_json::Value::String(v.clone())));
    }
    for child in &r.children {
        node.push_child(inspect_to_tree_node(child));
    }
    node
}

pub async fn inspect() -> Result<()> {
    let ctrl_client = Client::new(SysdSocketID::CTRL).await?;
    let inspect = ctrl_client
        .ctrl()
        .troubleshoot_inspect(())
        .await?
        .into_inner();

    let mut root = inspect_to_tree_node(&inspect);

    // Unlike Go (which fetches Capabilities over the same ctrl-socket client
    // — that RPC is only ever registered on the default socket, so that call
    // would fail at runtime), dial the default socket separately here.
    match Client::new(SysdSocketID::Default).await {
        Ok(default_client) => match default_client.ping().capabilities(()).await {
            Ok(caps) => {
                let value = serde_json::to_value(caps.into_inner())?;
                root.push_child(value_to_tree_node("Capabilities", &value));
            }
            Err(e) => tracing::warn!("failed to fetch capabilities: {e:?}"),
        },
        Err(e) => tracing::warn!("failed to connect to default socket: {e:?}"),
    }

    for line in root.render() {
        println!("{line}");
    }
    Ok(())
}
