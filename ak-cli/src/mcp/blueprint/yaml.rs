//! A minimal tag-preserving YAML AST built over `saphyr-parser`'s event stream.
//!
//! The validator's whole boundary depends on *seeing* YAML tags (`!Find`,
//! `!Env`, …), which a serde-style loader discards. We therefore walk the raw
//! event stream and keep the tag on every node, plus a plain-value projection
//! (the equivalent of the TS `yaml` lib's `.toJSON()`) for value checks.

use saphyr_parser::{Event, Parser, ScalarStyle, Tag};

/// A parsed YAML node that preserves its explicit tag (if any).
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Scalar {
        value: String,
        /// True only for YAML plain style; quoted/block scalars are always strings.
        plain: bool,
        tag: Option<String>,
    },
    Seq {
        tag: Option<String>,
        items: Vec<Node>,
    },
    Map {
        tag: Option<String>,
        pairs: Vec<(Node, Node)>,
    },
    /// A YAML alias (`*anchor`). Unsupported in blueprints; kept opaque.
    Alias,
}

/// A plain, tag-free projection of a node — the analogue of `toJSON()`.
#[derive(Debug, Clone, PartialEq)]
pub enum Plain {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Seq(Vec<Plain>),
    Map(Vec<(String, Plain)>),
}

impl serde::Serialize for Plain {
    /// Serialize to natural JSON (a bare value), not a tag-wrapped enum, so a
    /// flagged attribute's value reads as the value the operator wrote.
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{SerializeMap, SerializeSeq};
        match self {
            Plain::Null => s.serialize_none(),
            Plain::Bool(b) => s.serialize_bool(*b),
            Plain::Int(i) => s.serialize_i64(*i),
            Plain::Float(f) => s.serialize_f64(*f),
            Plain::Str(v) => s.serialize_str(v),
            Plain::Seq(items) => {
                let mut seq = s.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Plain::Map(pairs) => {
                let mut map = s.serialize_map(Some(pairs.len()))?;
                for (k, v) in pairs {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }
    }
}

impl Node {
    /// The explicit tag on this node, normalized (`!Find`, `!KeyOf`, …), or
    /// `None` for an untagged node.
    pub fn tag(&self) -> Option<&str> {
        match self {
            Node::Scalar { tag, .. } | Node::Seq { tag, .. } | Node::Map { tag, .. } => {
                tag.as_deref()
            }
            Node::Alias => None,
        }
    }

    /// Project to a plain value, discarding tags (mirrors `pdoc.toJSON()`).
    pub fn to_plain(&self) -> Plain {
        match self {
            Node::Scalar { value, plain, .. } => resolve_scalar(value, *plain),
            Node::Seq { items, .. } => Plain::Seq(items.iter().map(Node::to_plain).collect()),
            Node::Map { pairs, .. } => Plain::Map(
                pairs
                    .iter()
                    .map(|(k, v)| (scalar_key(k), v.to_plain()))
                    .collect(),
            ),
            Node::Alias => Plain::Null,
        }
    }

    /// Look up a value node by key in a mapping (untagged string key match).
    pub fn get<'a>(&'a self, key: &str) -> Option<&'a Node> {
        if let Node::Map { pairs, .. } = self {
            for (k, v) in pairs {
                if let Node::Scalar { value, .. } = k
                    && value == key
                {
                    return Some(v);
                }
            }
        }
        None
    }

    /// The string value of a scalar node, or `None` for non-scalars.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Node::Scalar { value, .. } => Some(value),
            _ => None,
        }
    }
}

/// Resolve a scalar's plain-value type. Only plain-style scalars get YAML core
/// schema resolution; quoted/block scalars are always strings.
fn resolve_scalar(value: &str, plain: bool) -> Plain {
    if !plain {
        return Plain::Str(value.to_string());
    }
    match value {
        "" | "~" | "null" | "Null" | "NULL" => Plain::Null,
        "true" | "True" | "TRUE" => Plain::Bool(true),
        "false" | "False" | "FALSE" => Plain::Bool(false),
        _ => {
            if let Ok(i) = value.parse::<i64>() {
                Plain::Int(i)
            } else if value.parse::<f64>().is_ok()
                && value.chars().any(|c| c == '.' || c == 'e' || c == 'E')
            {
                // Only treat as float when it actually looks like one, so an
                // all-digit string that overflows i64 doesn't silently become a
                // float; such a value stays a string.
                match value.parse::<f64>() {
                    Ok(f) => Plain::Float(f),
                    Err(_) => Plain::Str(value.to_string()),
                }
            } else {
                Plain::Str(value.to_string())
            }
        }
    }
}

/// True when a node is a scalar whose plain projection is a string (not a
/// number/bool/null). Used to enforce `!Find`'s "plain untagged scalar" shape.
pub fn is_string_scalar(node: &Node) -> bool {
    matches!(node, Node::Scalar { .. }) && matches!(node.to_plain(), Plain::Str(_))
}

fn scalar_key(k: &Node) -> String {
    match k {
        Node::Scalar { value, .. } => value.clone(),
        _ => String::new(),
    }
}

fn norm_tag(tag: Option<&Tag>) -> Option<String> {
    tag.map(|t| {
        if t.handle == "!" {
            format!("!{}", t.suffix)
        } else {
            // Secondary/named/verbatim handles: fall back to Display. These are
            // never in the permitted set, so exact form only needs to differ
            // from "!Find"/"!KeyOf".
            format!("{t}")
        }
    })
}

/// An owned, lifetime-free projection of a saphyr event.
enum Ev {
    Scalar {
        value: String,
        plain: bool,
        tag: Option<String>,
    },
    SeqStart(Option<String>),
    SeqEnd,
    MapStart(Option<String>),
    MapEnd,
    DocStart,
    DocEnd,
    Alias,
    Other,
}

fn own_event(ev: Event<'_>) -> Ev {
    match ev {
        Event::Scalar(value, style, _anchor, tag) => Ev::Scalar {
            value: value.into_owned(),
            plain: matches!(style, ScalarStyle::Plain),
            tag: norm_tag(tag.as_deref()),
        },
        Event::SequenceStart(_, tag) => Ev::SeqStart(norm_tag(tag.as_deref())),
        Event::SequenceEnd => Ev::SeqEnd,
        Event::MappingStart(_, tag) => Ev::MapStart(norm_tag(tag.as_deref())),
        Event::MappingEnd => Ev::MapEnd,
        Event::DocumentStart(_) => Ev::DocStart,
        Event::DocumentEnd => Ev::DocEnd,
        Event::Alias(_) => Ev::Alias,
        _ => Ev::Other,
    }
}

struct Builder {
    evs: Vec<Ev>,
    pos: usize,
}

impl Builder {
    fn peek(&self) -> Option<&Ev> {
        self.evs.get(self.pos)
    }

    fn next(&mut self) -> Option<&Ev> {
        let e = self.evs.get(self.pos);
        if e.is_some() {
            self.pos += 1;
        }
        e
    }

    /// Build the node beginning at the cursor. Returns `None` at a structural
    /// terminator (end of a collection or document).
    fn build(&mut self) -> Option<Node> {
        match self.peek() {
            Some(Ev::Scalar { .. }) => {
                if let Some(Ev::Scalar { value, plain, tag }) = self.next() {
                    Some(Node::Scalar {
                        value: value.clone(),
                        plain: *plain,
                        tag: tag.clone(),
                    })
                } else {
                    None
                }
            }
            Some(Ev::Alias) => {
                self.pos += 1;
                Some(Node::Alias)
            }
            Some(Ev::SeqStart(_)) => {
                let tag = if let Some(Ev::SeqStart(t)) = self.next() {
                    t.clone()
                } else {
                    None
                };
                let mut items = Vec::new();
                loop {
                    match self.peek() {
                        Some(Ev::SeqEnd) => {
                            self.pos += 1;
                            break;
                        }
                        None => break,
                        _ => {
                            if let Some(n) = self.build() {
                                items.push(n);
                            } else {
                                break;
                            }
                        }
                    }
                }
                Some(Node::Seq { tag, items })
            }
            Some(Ev::MapStart(_)) => {
                let tag = if let Some(Ev::MapStart(t)) = self.next() {
                    t.clone()
                } else {
                    None
                };
                let mut pairs = Vec::new();
                loop {
                    match self.peek() {
                        Some(Ev::MapEnd) => {
                            self.pos += 1;
                            break;
                        }
                        None => break,
                        _ => {
                            let key = match self.build() {
                                Some(k) => k,
                                None => break,
                            };
                            let val = match self.build() {
                                Some(v) => v,
                                None => Node::Scalar {
                                    value: String::new(),
                                    plain: true,
                                    tag: None,
                                },
                            };
                            pairs.push((key, val));
                        }
                    }
                }
                Some(Node::Map { tag, pairs })
            }
            _ => None,
        }
    }
}

/// Parse a single YAML document into a tag-preserving AST. Returns `Ok(None)`
/// for an empty document, `Err` with a message on a scan error. Never panics.
pub fn parse_document(content: &str) -> Result<Option<Node>, String> {
    let mut evs = Vec::new();
    for item in Parser::new_from_str(content) {
        match item {
            Ok((event, _span)) => evs.push(own_event(event)),
            Err(e) => return Err(format!("{e}")),
        }
    }
    let mut b = Builder { evs, pos: 0 };
    // Advance to the first document's content.
    while let Some(ev) = b.peek() {
        match ev {
            Ev::DocStart => {
                b.pos += 1;
                return Ok(b.build());
            }
            Ev::DocEnd => break,
            _ => b.pos += 1,
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scalar_map_and_seq() {
        let n = parse_document("a: 1\nb: [x, y]").unwrap().unwrap();
        assert_eq!(n.get("a").unwrap().to_plain(), Plain::Int(1));
        assert_eq!(
            n.get("b").unwrap().to_plain(),
            Plain::Seq(vec![Plain::Str("x".into()), Plain::Str("y".into())])
        );
    }

    #[test]
    fn preserves_local_tags() {
        let n = parse_document("x: !Find [m, [a, b]]").unwrap().unwrap();
        assert_eq!(n.get("x").unwrap().tag(), Some("!Find"));
    }

    #[test]
    fn plain_bool_and_quoted_string_differ() {
        let n = parse_document("a: false\nb: \"false\"").unwrap().unwrap();
        assert_eq!(n.get("a").unwrap().to_plain(), Plain::Bool(false));
        assert_eq!(n.get("b").unwrap().to_plain(), Plain::Str("false".into()));
    }

    #[test]
    fn numeric_scalar_is_not_a_string() {
        let n = parse_document("a: 999\nb: scope-x").unwrap().unwrap();
        assert!(!is_string_scalar(n.get("a").unwrap()));
        assert!(is_string_scalar(n.get("b").unwrap()));
    }

    #[test]
    fn empty_input_is_none_not_panic() {
        assert_eq!(parse_document("").unwrap(), None);
        assert_eq!(parse_document("   \n# just a comment").unwrap(), None);
    }
}
