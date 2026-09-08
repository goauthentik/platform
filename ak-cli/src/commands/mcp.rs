use rmcp::{ServiceExt as _, transport::stdio};

use crate::{App, mcp::AuthentikMcp};

pub async fn mcp(app: App) -> eyre::Result<()> {
    let service = AuthentikMcp::new(app)
        .serve(stdio())
        .await
        .inspect_err(|e| tracing::error!("serving error: {e:?}"))?;
    service.waiting().await?;
    Ok(())
}
