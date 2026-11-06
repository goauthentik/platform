use rmcp::{ServiceExt as _, transport::stdio};

use crate::{App, mcp::AuthentikMcp};

pub async fn mcp(app: App, allow_dangerous_api: bool) -> eyre::Result<()> {
    let service = AuthentikMcp::new(app, allow_dangerous_api)
        .serve(stdio())
        .await
        .inspect_err(|e| tracing::error!("serving error: {e:?}"))?;
    service.waiting().await?;
    Ok(())
}
