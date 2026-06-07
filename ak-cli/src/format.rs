use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;

// Styles — use functions instead of lazy statics for composability
pub fn key_style() -> Style {
    Style::default().fg(Color::Cyan) // ANSI color 6
}

pub fn value_style() -> Style {
    Style::default().fg(Color::Green) // ANSI color 2
}

pub fn box_style() -> Style {
    Style::default()
        .fg(Color::Rgb(250, 250, 250))
        .bg(Color::Rgb(253, 75, 45))
        .add_modifier(Modifier::BOLD)
}

pub fn inline_style() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

// A simple tree node that owns its rendered lines
#[derive(Debug)]
pub struct TreeNode {
    pub label: String,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            children: Vec::new(),
        }
    }

    pub fn child(mut self, node: TreeNode) -> Self {
        self.children.push(node);
        self
    }

    pub fn push_child(&mut self, node: TreeNode) {
        self.children.push(node);
    }

    /// Render the tree into a Vec of styled Lines (ratatui-ready).
    /// `prefix` tracks the indentation/connector string for the current level.
    pub fn render(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        lines.push(Line::from(Span::raw(self.label.clone())));
        self.render_children(&mut lines, "");
        lines
    }

    fn render_children(&self, lines: &mut Vec<Line<'static>>, prefix: &str) {
        let count = self.children.len();
        for (i, child) in self.children.iter().enumerate() {
            let is_last = i == count - 1;
            let connector = if is_last { "╰── " } else { "├── " };
            let child_prefix = if is_last { "    " } else { "│   " };

            lines.push(Line::from(Span::raw(format!(
                "{}{}{}",
                prefix, connector, child.label
            ))));

            child.render_children(lines, &format!("{}{}", prefix, child_prefix));
        }
    }
}

/// Build a TreeNode from a `serde_json::Value`, mirroring `AddNodeToTree`.
pub fn value_to_tree_node(label: &str, value: &Value) -> TreeNode {
    match value {
        Value::Object(map) => {
            let mut node = TreeNode::new(styled_key(label));
            // BTreeMap gives us sorted keys for free
            let sorted: BTreeMap<_, _> = map.iter().collect();
            for (k, v) in sorted {
                node.push_child(value_to_tree_node(k, v));
            }
            node
        }

        Value::Array(arr) => {
            let mut node = TreeNode::new(styled_key(label));
            for (i, item) in arr.iter().enumerate() {
                let index_label = styled_key(&format!("[{i}]"));
                node.push_child(value_to_tree_node(&index_label, item));
            }
            node
        }

        Value::String(s) => {
            // Mirror the Go behaviour: if the string is itself valid JSON, recurse
            if let Ok(inner) = serde_json::from_str::<Value>(s)
                && matches!(inner, Value::Object(_) | Value::Array(_)) {
                    return value_to_tree_node(label, &inner);
                }
            leaf_node(label, s)
        }

        Value::Number(n) => leaf_node(label, &n.to_string()),
        Value::Bool(b) => leaf_node(label, &b.to_string()),
        Value::Null => leaf_node(label, "null"),
    }
}

/// Render a `serde_json::Value` (expected to be an Object) as a tree string,
/// mirroring `RenderMapAsTree`.
pub fn render_map_as_tree(data: &Value, root_title: &str) -> Vec<Line<'static>> {
    let mut root = TreeNode::new(root_title.to_string());

    if let Value::Object(map) = data {
        let sorted: BTreeMap<_, _> = map.iter().collect();
        for (k, v) in sorted {
            root.push_child(value_to_tree_node(k, v));
        }
    }

    root.render()
}

pub fn render_json(raw: String, title: &str, json: bool) -> Result<(), Box<dyn Error>> {
    if json {
        println!("{}", raw);
    } else {
        let body: Value = serde_json::from_str(&raw)?;
        let lines = render_map_as_tree(&body, title);
        for line in &lines {
            println!("{}", line.spans.iter().map(|s| s.content.as_ref()).collect::<String>());
        }
    }
    Ok(())
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Wrap a key string in the ANSI cyan color (matches `KeyStyle.Render(key)`).
fn styled_key(s: &str) -> String {
    ansi_color(s, 6)
}

/// Wrap a value string in the ANSI green color (matches `ValueStyle.Render(v)`).
fn styled_value(s: &str) -> String {
    ansi_color(s, 2)
}

/// Produce a leaf node: `key: value` with the value styled.
fn leaf_node(label: &str, value: &str) -> TreeNode {
    TreeNode::new(format!("{}: {}", styled_key(label), styled_value(value)))
}

/// Minimal ANSI 256-color wrapper — keeps parity with lipgloss colour ints.
fn ansi_color(s: &str, code: u8) -> String {
    format!("\x1b[38;5;{code}m{s}\x1b[0m")
}

// ── example / smoke-test ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn renders_without_panic() {
        let data = json!({
            "name": "authentik",
            "version": "2024.1",
            "providers": ["oauth2", "saml", "ldap"],
            "config": {
                "debug": false,
                "port": 9000
            }
        });

        let lines = render_map_as_tree(&data, "authentik config");
        for line in &lines {
            println!("{}", line.spans.iter().map(|s| s.content.as_ref()).collect::<String>());
        }
        assert!(!lines.is_empty());
    }
}
