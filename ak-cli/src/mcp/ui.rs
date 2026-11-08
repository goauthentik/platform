//! Interactive views served over the MCP Apps extension (`io.modelcontextprotocol/ui`). A tool
//! points at a view with `_meta.ui.resourceUri`; hosts that support the extension render it in a
//! sandboxed iframe, everyone else just sees the tool's text content.

use rmcp::{ErrorData as McpError, Peer, RoleServer, model::*};
use serde_json::json;

const EXTENSION: &str = "io.modelcontextprotocol/ui";
const MIME_TYPE: &str = "text/html;profile=mcp-app";

pub const GRANT_REQUEST: &str = "ui://authentik/grant-request.html";

/// View URI to its HTML.
const VIEWS: &[(&str, &str)] = &[(GRANT_REQUEST, include_str!("ui/grant-request.html"))];

/// Tool `_meta` linking the tool to the view at `uri`.
pub fn meta(uri: &str) -> MetaObject {
    MetaObject(
        json!({ "ui": { "resourceUri": uri } })
            .as_object()
            .cloned()
            .unwrap_or_default(),
    )
}

/// Whether the client renders views, i.e. declared the extension with our MIME type.
pub fn supported(peer: &Peer<RoleServer>) -> bool {
    peer.peer_info()
        .is_some_and(|info| client_supports(&info.capabilities))
}

fn client_supports(capabilities: &ClientCapabilities) -> bool {
    capabilities
        .extensions
        .as_ref()
        .and_then(|e| e.get(EXTENSION))
        .and_then(|ui| ui.get("mimeTypes"))
        .and_then(|m| m.as_array())
        .is_some_and(|m| m.iter().any(|t| t == MIME_TYPE))
}

pub fn resources() -> Vec<Resource> {
    VIEWS
        .iter()
        .map(|(uri, html)| {
            Resource::new(*uri, *uri)
                .with_mime_type(MIME_TYPE)
                .with_size(html.len() as u64)
        })
        .collect()
}

/// `None` if no view is served at `uri`.
pub fn read(uri: &str) -> Option<Result<ReadResourceResult, McpError>> {
    let (_, html) = VIEWS.iter().find(|(u, _)| *u == uri)?;
    Some(Ok(ReadResourceResult::new(vec![
        ResourceContents::text(*html, uri).with_mime_type(MIME_TYPE),
    ])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn views_are_served() {
        for (uri, _) in VIEWS {
            assert!(uri.starts_with("ui://"));
            assert!(read(uri).is_some_and(|r| r.is_ok()));
        }
        assert!(read("ui://nope").is_none());
        assert_eq!(meta(GRANT_REQUEST).0["ui"]["resourceUri"], GRANT_REQUEST);
    }

    #[test]
    fn client_support() {
        let caps = |v: serde_json::Value| {
            serde_json::from_value::<ClientCapabilities>(v).expect("capabilities should parse")
        };
        assert!(client_supports(&caps(json!({
            "extensions": { EXTENSION: { "mimeTypes": [MIME_TYPE] } }
        }))));
        assert!(!client_supports(&caps(json!({
            "extensions": { EXTENSION: { "mimeTypes": ["text/plain"] } }
        }))));
        assert!(!client_supports(&caps(json!({}))));
    }
}
