use std::{collections::HashSet, path::Path};
use syn::Item;

#[derive(Debug, Clone)]
pub struct ModelField {
    pub json_key: String,        // the serde rename value (JSON key)
    pub cli_flag: String,        // kebab-case flag name
    pub rust_ident: String,      // sanitized Rust identifier for the struct field
    pub help: Option<String>,    // extracted from /// doc comments
    pub type_hint: &'static str, // e.g. "STRING", "NUMBER", "BOOL", "UUID"
}

#[derive(Debug, Clone)]
pub struct ApiFunction {
    pub module: String,
    pub full_name: String,
    /// The resource portion of the function name, e.g. `"users"` or `"application_entitlements"`.
    /// Empty string for functions with no resource segment (e.g. `schema_retrieve`).
    pub resource: String,
    /// The operation portion, e.g. `"retrieve"`, `"list"`, `"partial_update"`.
    pub operation: String,
    pub params: Vec<ApiParam>,
    pub returns_unit: bool,
    /// True when the function returns `reqwest::Response` (raw stream, not serializable).
    pub returns_response: bool,
    /// Doc comment extracted from the `///` lines above the function, if any.
    pub doc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ApiParam {
    pub name: String,
    pub field_name: String,
    pub cli_flag: String,
    pub ty: ParamType,
}

#[derive(Debug, Clone)]
pub enum ParamType {
    RequiredStr,
    OptionalStr,
    RequiredInt,
    OptionalInt,
    OptionalBool,
    /// A required request body (model type name ends with `Request`). Accepted as JSON.
    RequiredModel(String, Vec<ModelField>),
    /// An optional request body (model type name ends with `Request`). Accepted as JSON.
    OptionalModel(String, Vec<ModelField>),
    /// An optional enum/filter param (model type that is NOT a `*Request`).
    /// Accepted as a plain string and parsed via serde.
    /// The Vec holds the serde-rename values from each enum variant.
    OptionalEnum(String, Vec<String>),
    RequiredVecStr,
    RequiredVecInt,
    OptionalVecStr,
    OptionalVecInt,
    OptionalVecUuid,
    OptionalVecModel(String),
    OptionalChronoStr,
    RequiredFile,
    OptionalFile,
}

/// Canonical operations that, when found as a sibling suffix, prove a prefix is a resource.
const CANONICAL_OPS: &[&str] = &[
    "list",
    "create",
    "retrieve",
    "destroy",
    "update",
    "partial_update",
    "used_by_list",
];

/// Standard CRUD verbs used by the fallback heuristic.
const CRUD_VERBS: &[&str] = &["list", "create", "destroy", "retrieve", "update"];

/// Context-aware split: find the longest prefix `P` that is a "real resource", then return
/// `(P, normalize_operation(rest))`.
///
/// A prefix qualifies as a real resource when **either**:
/// - a sibling `{P}_list` exists (the clearest indicator), **or**
/// - ≥ 2 distinct sibling canonical ops exist (e.g. both `_retrieve` and `_create`).
///
/// Requiring a list or plurality prevents custom actions like `users_impersonate_create`
/// (whose only sibling is `users_impersonate_end_retrieve`) from falsely promoting
/// `users_impersonate` to a resource.
///
/// The operation is additionally normalized to drop a trailing CRUD verb from multi-word
/// custom action names (`impersonate_create` → `impersonate`, `health_list` → `health`).
pub fn split_with_context(suffix: &str, module_suffixes: &HashSet<String>) -> (String, String) {
    let (resource, operation) = raw_context_split(suffix, module_suffixes);
    (resource, normalize_operation(&operation))
}

fn raw_context_split(suffix: &str, module_suffixes: &HashSet<String>) -> (String, String) {
    let words: Vec<&str> = suffix.split('_').collect();

    if words.len() == 1 {
        return (String::new(), words[0].to_owned());
    }

    for i in (1..words.len()).rev() {
        let prefix = words[..i].join("_");
        if is_real_resource(&prefix, suffix, module_suffixes) {
            let operation = words[i..].join("_");
            return (prefix, operation);
        }
    }

    split_resource_op(suffix)
}

/// Returns `true` when `prefix` is a genuine resource: it has a `_list` sibling (the current
/// function IS allowed to be that sibling), or ≥ 2 distinct non-self canonical-op siblings.
///
/// Allowing the current function to be its own `_list` evidence is intentional: it ensures
/// that `permissions_assigned_by_roles_list` is anchored to the `permissions_assigned_by_roles`
/// resource rather than falling through to the shorter `permissions` prefix, which would cause a
/// struct naming collision between the resource wrapper and a function args struct.
fn is_real_resource(prefix: &str, current_suffix: &str, module_suffixes: &HashSet<String>) -> bool {
    let list_key = format!("{prefix}_list");
    if module_suffixes.contains(&list_key) {
        return true;
    }
    let mut count = 0usize;
    for &op in CANONICAL_OPS {
        let candidate = format!("{prefix}_{op}");
        if candidate != current_suffix && module_suffixes.contains(&candidate) {
            count += 1;
            if count >= 2 {
                return true;
            }
        }
    }
    false
}

/// Operations that must keep their trailing CRUD verb even though they are multi-word.
const KEEP_OPERATION_AS_IS: &[&str] = &["partial_update", "used_by_list"];

/// Strip the trailing CRUD verb from a multi-word custom action name.
///
/// Examples:
/// - `"impersonate_create"` → `"impersonate"`
/// - `"health_list"` → `"health"`
/// - `"check_access_retrieve"` → `"check_access"`
/// - `"list_latest"` → `"list_latest"` (last word `latest` is not a CRUD verb)
/// - `"create"` → `"create"` (single word, unchanged)
/// - `"partial_update"` → `"partial_update"` (in the keep list)
pub fn normalize_operation(op: &str) -> String {
    if KEEP_OPERATION_AS_IS.contains(&op) {
        return op.to_owned();
    }
    let words: Vec<&str> = op.split('_').collect();
    if words.len() > 1
        && let Some(&last) = words.last()
        && CRUD_VERBS.contains(&last)
    {
        return words[..words.len() - 1].join("_");
    }
    op.to_owned()
}

/// Fallback heuristic split used when no module context is available.
///
/// Examples:
/// - `"users_retrieve"` → `("users", "retrieve")`
/// - `"application_entitlements_partial_update"` → `("application_entitlements", "partial_update")`
/// - `"iterations_list_latest"` → `("iterations", "list_latest")`
/// - `"retrieve"` (single word) → `("", "retrieve")`
pub fn split_resource_op(suffix: &str) -> (String, String) {
    if suffix.is_empty() {
        return (String::new(), String::new());
    }

    let words: Vec<&str> = suffix.split('_').collect();

    if words.len() == 1 {
        return (String::new(), words[0].to_owned());
    }

    // 3-word compound ops.
    if words.len() >= 3 && words[words.len() - 3..] == ["used", "by", "list"] {
        return (
            words[..words.len() - 3].join("_"),
            "used_by_list".to_owned(),
        );
    }

    // 2-word compound ops.
    if words.len() >= 2 && words[words.len() - 2..] == ["partial", "update"] {
        return (
            words[..words.len() - 2].join("_"),
            "partial_update".to_owned(),
        );
    }

    // Rightmost standard CRUD verb.
    for i in (0..words.len()).rev() {
        if CRUD_VERBS.contains(&words[i]) {
            return (words[..i].join("_"), words[i..].join("_"));
        }
    }

    let n = words.len();
    (words[..n - 1].join("_"), words[n - 1].to_owned())
}

/// Intermediate representation used during the first parsing pass.
struct RawFn {
    module: String,
    full_name: String,
    suffix: String,
    params: Vec<ApiParam>,
    returns_unit: bool,
    returns_response: bool,
    doc: Option<String>,
}

pub fn parse_all_apis(
    apis_dir: &Path,
) -> Result<Vec<ApiFunction>, Box<dyn std::error::Error + Send + Sync>> {
    let entries = std::fs::read_dir(apis_dir)?;
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();

    // First pass: collect raw function data, grouped by module.
    let mut by_module: std::collections::BTreeMap<String, Vec<RawFn>> =
        std::collections::BTreeMap::new();

    for path in &paths {
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(s) => s.to_owned(),
            None => continue,
        };
        if !filename.ends_with("_api.rs") {
            continue;
        }
        let module = match filename.strip_suffix("_api.rs") {
            Some(m) => m.to_owned(),
            None => continue,
        };
        let content = std::fs::read_to_string(path)?;
        let file = syn::parse_file(&content)?;
        by_module
            .entry(module.clone())
            .or_default()
            .extend(collect_raw(&module, &file));
    }

    // Second pass: use each module's full suffix set to split resource/operation accurately.
    let mut all_fns = Vec::new();
    for raw_fns in by_module.values() {
        let module_suffixes: HashSet<String> = raw_fns.iter().map(|f| f.suffix.clone()).collect();
        for raw in raw_fns {
            let (resource, operation) = split_with_context(&raw.suffix, &module_suffixes);
            all_fns.push(ApiFunction {
                module: raw.module.clone(),
                full_name: raw.full_name.clone(),
                resource,
                operation,
                params: raw.params.clone(),
                returns_unit: raw.returns_unit,
                returns_response: raw.returns_response,
                doc: raw.doc.clone(),
            });
        }
    }

    // Derive models directory from apis directory (sibling of apis/)
    let models_dir_opt: Option<std::path::PathBuf> = apis_dir.parent().map(|p| p.join("models"));

    if let Some(ref models_dir) = models_dir_opt {
        for f in &mut all_fns {
            for param in &mut f.params {
                match param.ty.clone() {
                    ParamType::RequiredModel(type_name, _) => {
                        let fields = parse_model_fields(models_dir, &type_name);
                        param.ty = ParamType::RequiredModel(type_name, fields);
                    }
                    ParamType::OptionalModel(type_name, _) => {
                        let fields = parse_model_fields(models_dir, &type_name);
                        param.ty = ParamType::OptionalModel(type_name, fields);
                    }
                    ParamType::OptionalEnum(type_name, _) => {
                        let values = parse_enum_values(models_dir, &type_name);
                        param.ty = ParamType::OptionalEnum(type_name, values);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(all_fns)
}

fn collect_raw(module: &str, file: &syn::File) -> Vec<RawFn> {
    file.items
        .iter()
        .filter_map(|item| raw_from_item(module, item))
        .collect()
}

fn raw_from_item(module: &str, item: &Item) -> Option<RawFn> {
    let f = match item {
        Item::Fn(f) => f,
        _ => return None,
    };
    if !matches!(f.vis, syn::Visibility::Public(_)) || f.sig.asyncness.is_none() {
        return None;
    }
    let full_name = f.sig.ident.to_string();
    let prefix = format!("{module}_");
    let suffix = full_name.strip_prefix(&prefix)?.to_owned();
    Some(RawFn {
        module: module.to_owned(),
        full_name,
        suffix,
        params: extract_params(&f.sig.inputs),
        returns_unit: check_returns_unit(&f.sig.output),
        returns_response: check_returns_response(&f.sig.output),
        doc: get_doc_comment(&f.attrs),
    })
}

/// Extract functions from a parsed file using only the fallback heuristic (no module context).
/// Suitable for unit tests; production code goes through `parse_all_apis`.
pub fn extract_functions(module: &str, file: &syn::File) -> Vec<ApiFunction> {
    collect_raw(module, file)
        .into_iter()
        .map(|raw| {
            let (resource, op) = split_resource_op(&raw.suffix);
            ApiFunction {
                module: raw.module,
                full_name: raw.full_name,
                resource,
                operation: normalize_operation(&op),
                params: raw.params,
                returns_unit: raw.returns_unit,
                returns_response: raw.returns_response,
                doc: raw.doc,
            }
        })
        .collect()
}

fn check_returns_unit(output: &syn::ReturnType) -> bool {
    match output {
        syn::ReturnType::Default => true,
        syn::ReturnType::Type(_, ty) => compact_type_str(ty).contains("Result<(),"),
    }
}

fn check_returns_response(output: &syn::ReturnType) -> bool {
    match output {
        syn::ReturnType::Default => false,
        syn::ReturnType::Type(_, ty) => compact_type_str(ty).contains("reqwest::Response"),
    }
}

fn extract_params(
    inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::Token![,]>,
) -> Vec<ApiParam> {
    inputs
        .iter()
        .filter_map(|arg| {
            let pat_type = match arg {
                syn::FnArg::Typed(pt) => pt,
                _ => return None,
            };
            let raw_name = match &*pat_type.pat {
                syn::Pat::Ident(pi) => pi.ident.to_string(),
                _ => return None,
            };
            // proc_macro2 preserves the `r#` prefix for raw identifiers (e.g. r#type → "r#type").
            // Strip it to get the plain name so downstream code can form valid identifiers.
            let name = raw_name.strip_prefix("r#").unwrap_or(&raw_name).to_owned();
            if name == "configuration" {
                return None;
            }
            let ty = classify_type(&pat_type.ty);
            let field_name = sanitize_field_name(&name);
            let cli_flag = to_cli_flag(&name);
            Some(ApiParam {
                name,
                field_name,
                cli_flag,
                ty,
            })
        })
        .collect()
}

fn compact_type_str(ty: &syn::Type) -> String {
    quote::quote!(#ty)
        .to_string()
        .chars()
        .filter(|&c| c != ' ')
        .collect()
}

fn classify_type(ty: &syn::Type) -> ParamType {
    classify_compact(&compact_type_str(ty))
}

fn classify_compact(s: &str) -> ParamType {
    match s {
        "&str" => ParamType::RequiredStr,
        "i32" | "i64" | "u32" | "u64" | "usize" => ParamType::RequiredInt,
        "bool" => ParamType::OptionalBool,
        "std::path::PathBuf" => ParamType::RequiredFile,
        "Vec<String>" => ParamType::RequiredVecStr,
        "Vec<i32>" | "Vec<i64>" | "Vec<u32>" | "Vec<u64>" => ParamType::RequiredVecInt,
        "Option<&str>" => ParamType::OptionalStr,
        "Option<i32>" | "Option<i64>" | "Option<u32>" | "Option<u64>" => ParamType::OptionalInt,
        "Option<bool>" => ParamType::OptionalBool,
        "Option<Vec<String>>" => ParamType::OptionalVecStr,
        "Option<Vec<uuid::Uuid>>" => ParamType::OptionalVecUuid,
        "Option<Vec<i32>>" | "Option<Vec<i64>>" | "Option<Vec<u32>>" | "Option<Vec<u64>>" => {
            ParamType::OptionalVecInt
        }
        "Option<std::path::PathBuf>" => ParamType::OptionalFile,
        _ if s.starts_with("models::") => {
            let name = last_path_segment(s);
            ParamType::RequiredModel(name, Vec::new())
        }
        _ if s.starts_with("Option<Vec<models::") => {
            let inner = s
                .strip_prefix("Option<Vec<models::")
                .and_then(|t| t.strip_suffix(">>"))
                .unwrap_or(s);
            ParamType::OptionalVecModel(inner.to_owned())
        }
        _ if s.starts_with("Option<models::") => {
            let inner = s
                .strip_prefix("Option<models::")
                .and_then(|t| t.strip_suffix('>'))
                .unwrap_or(s);
            // Types ending in "Request" are JSON request bodies.
            // All others (enums, modes, etc.) are accepted as plain strings.
            if inner.ends_with("Request") {
                ParamType::OptionalModel(inner.to_owned(), Vec::new())
            } else {
                ParamType::OptionalEnum(inner.to_owned(), Vec::new())
            }
        }
        _ if s.starts_with("Option<chrono::") => ParamType::OptionalChronoStr,
        _ if s.starts_with("Option<Vec<") => ParamType::OptionalVecStr,
        _ if s.starts_with("Option<") => ParamType::OptionalStr,
        _ => ParamType::OptionalStr,
    }
}

fn last_path_segment(s: &str) -> String {
    s.split("::").last().unwrap_or(s).to_owned()
}

fn sanitize_field_name(name: &str) -> String {
    let s = name.replace("__", "_");
    match s.as_str() {
        "type" | "use" | "loop" | "for" | "if" | "else" | "match" | "fn" | "struct" | "enum"
        | "impl" | "trait" | "return" | "let" | "const" | "static" | "pub" | "mod" | "super"
        | "self" | "Self" | "where" | "async" | "await" | "move" | "in" | "while" | "break"
        | "continue" | "ref" | "mut" | "extern" | "crate" | "dyn" | "as" | "true" | "false"
        | "unsafe" | "yield" | "abstract" | "become" | "do" | "final" | "override" | "priv"
        | "typeof" | "unsized" | "virtual" | "try" => format!("{s}_field"),
        _ => s,
    }
}

fn to_cli_flag(name: &str) -> String {
    name.replace("__", "-").replace('_', "-")
}

pub fn parse_model_fields(models_dir: &Path, type_name: &str) -> Vec<ModelField> {
    let filename = format!("{}.rs", pascal_to_snake(type_name));
    let path = models_dir.join(&filename);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let file = match syn::parse_file(&content) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    for item in &file.items {
        let s = match item {
            syn::Item::Struct(s) if s.ident == type_name => s,
            _ => continue,
        };
        let named = match &s.fields {
            syn::Fields::Named(f) => &f.named,
            _ => continue,
        };
        return named
            .iter()
            .filter_map(|f| {
                let field_name = f.ident.as_ref()?.to_string();
                let json_key = get_serde_rename(&f.attrs).unwrap_or_else(|| field_name.clone());
                let cli_flag = to_cli_flag(&field_name);
                let rust_ident = sanitize_field_name(&field_name);
                if rust_ident == "body" {
                    return None;
                }
                // Compact type string for hint/enum detection.
                let field_ty = &f.ty;
                let compact_ty: String = quote::quote!(#field_ty)
                    .to_string()
                    .chars()
                    .filter(|&c| c != ' ')
                    .collect();
                let type_hint = field_type_hint(&compact_ty);

                // Extract doc comment, then optionally append enum possible values.
                let mut help = get_doc_comment(&f.attrs);
                if let Some(enum_name) = enum_type_name(&compact_ty) {
                    let values = parse_enum_values(models_dir, enum_name);
                    if !values.is_empty() {
                        let values_str = format!("Possible values: {}", values.join(", "));
                        help = Some(match help {
                            Some(doc) => format!("{doc} {values_str}"),
                            None => values_str,
                        });
                    }
                }

                Some(ModelField {
                    json_key,
                    cli_flag,
                    rust_ident,
                    help,
                    type_hint,
                })
            })
            .collect();
    }
    Vec::new()
}

/// Extract serde-rename strings from every variant of an enum model file.
pub fn parse_enum_values(models_dir: &Path, type_name: &str) -> Vec<String> {
    let filename = format!("{}.rs", pascal_to_snake(type_name));
    let content = match std::fs::read_to_string(models_dir.join(&filename)) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let file = match syn::parse_file(&content) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    for item in &file.items {
        let e = match item {
            syn::Item::Enum(e) if e.ident == type_name => e,
            _ => continue,
        };
        return e
            .variants
            .iter()
            .filter_map(|v| get_serde_rename(&v.attrs))
            .collect();
    }
    Vec::new()
}

/// If the compact type string contains an optional or direct enum reference
/// (i.e. `Option<models::Foo>` where `Foo` doesn't end with `Request`),
/// return the enum type name so callers can look up its values.
fn enum_type_name(compact: &str) -> Option<&str> {
    let inner = compact
        .strip_prefix("Option<models::")
        .and_then(|s| s.strip_suffix('>'))
        .or_else(|| compact.strip_prefix("models::"))?;
    // Only consider non-Request types (those are body models, not enums).
    if inner.ends_with("Request") {
        None
    } else {
        Some(inner)
    }
}

/// Map a compact type string to a short type hint shown as the Clap value_name.
fn field_type_hint(compact: &str) -> &'static str {
    match compact {
        "String" | "Option<String>" => "STRING",
        "i32" | "i64" | "u32" | "u64" | "Option<i32>" | "Option<i64>" | "Option<u32>"
        | "Option<u64>" => "NUMBER",
        "bool" | "Option<bool>" => "BOOL",
        "uuid::Uuid" | "Option<uuid::Uuid>" => "UUID",
        "Option<Vec<String>>" | "Vec<String>" => "STRING...",
        "Option<Vec<i32>>" | "Option<Vec<i64>>" | "Vec<i32>" | "Vec<i64>" => "NUMBER...",
        "Option<Vec<uuid::Uuid>>" | "Vec<uuid::Uuid>" => "UUID...",
        _ if compact.starts_with("Option<chrono::") || compact.starts_with("chrono::") => {
            "DATETIME"
        }
        _ if compact.starts_with("Option<Vec<models::") || compact.starts_with("Vec<models::") => {
            "JSON..."
        }
        // Enum or nested model – both are passed as a string at the CLI level.
        _ => "STRING",
    }
}

fn get_serde_rename(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let compact: String = quote::quote!(#attr)
            .to_string()
            .chars()
            .filter(|&c| c != ' ')
            .collect();
        // compact looks like: `#[serde(rename="open_in_new_tab",skip_serializing_if="Option::is_none")]`
        if let Some(idx) = compact.find("rename=\"") {
            let rest = &compact[idx + 8..];
            if let Some(end) = rest.find('"') {
                return Some(rest[..end].to_owned());
            }
        }
    }
    None
}

/// Extract concatenated `///` doc comment lines from a field's attributes.
fn get_doc_comment(attrs: &[syn::Attribute]) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let syn::Meta::NameValue(ref nv) = attr.meta
            && let syn::Expr::Lit(ref el) = nv.value
            && let syn::Lit::Str(ref ls) = el.lit
        {
            let text = ls.value();
            let trimmed = text.trim().to_owned();
            if !trimmed.is_empty() {
                lines.push(trimmed);
            }
        }
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join(" "))
    }
}

fn pascal_to_snake(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn parse(src: &str) -> syn::File {
        syn::parse_str(src).unwrap()
    }

    fn suffixes(list: &[&str]) -> HashSet<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn split_with_context_custom_action() {
        // users_set_password_create: "users" is the resource because users_list exists.
        // The trailing _create is stripped by normalize_operation.
        let ctx = suffixes(&[
            "users_list",
            "users_retrieve",
            "users_create",
            "users_destroy",
            "users_set_password_create",
        ]);
        assert_eq!(
            split_with_context("users_set_password_create", &ctx),
            ("users".to_owned(), "set_password".to_owned())
        );
    }

    #[test]
    fn split_with_context_impersonate_pair() {
        // Both impersonate_create and impersonate_end_retrieve should land under "users",
        // not under a fake "users_impersonate" resource (which would only have 1 canonical op).
        let ctx = suffixes(&[
            "users_list",
            "users_retrieve",
            "users_create",
            "users_destroy",
            "users_set_password_create",
            "users_impersonate_create",
            "users_impersonate_end_retrieve",
        ]);
        assert_eq!(
            split_with_context("users_impersonate_create", &ctx),
            ("users".to_owned(), "impersonate".to_owned())
        );
        assert_eq!(
            split_with_context("users_impersonate_end_retrieve", &ctx),
            ("users".to_owned(), "impersonate_end".to_owned())
        );
    }

    #[test]
    fn split_with_context_plain_crud() {
        let ctx = suffixes(&["users_list", "users_retrieve", "users_create"]);
        assert_eq!(
            split_with_context("users_list", &ctx),
            ("users".to_owned(), "list".to_owned())
        );
    }

    #[test]
    fn split_with_context_partial_update() {
        let ctx = suffixes(&[
            "application_entitlements_list",
            "application_entitlements_create",
            "application_entitlements_partial_update",
        ]);
        assert_eq!(
            split_with_context("application_entitlements_partial_update", &ctx),
            (
                "application_entitlements".to_owned(),
                "partial_update".to_owned()
            )
        );
    }

    #[test]
    fn split_with_context_list_variant() {
        // iterations_list_latest: "iterations" has iterations_list sibling, so resource=iterations.
        let ctx = suffixes(&[
            "iterations_list",
            "iterations_list_latest",
            "iterations_list_open",
        ]);
        assert_eq!(
            split_with_context("iterations_list_latest", &ctx),
            ("iterations".to_owned(), "list_latest".to_owned())
        );
    }

    #[test]
    fn split_with_context_falls_back_to_heuristic() {
        // No siblings → fall back to rightmost-CRUD-verb.
        let ctx = suffixes(&["config_retrieve"]);
        assert_eq!(
            split_with_context("config_retrieve", &ctx),
            ("config".to_owned(), "retrieve".to_owned())
        );
    }

    #[test]
    fn split_simple() {
        assert_eq!(
            split_resource_op("users_retrieve"),
            ("users".to_owned(), "retrieve".to_owned())
        );
        assert_eq!(
            split_resource_op("applications_list"),
            ("applications".to_owned(), "list".to_owned())
        );
    }

    #[test]
    fn split_compound_ops() {
        assert_eq!(
            split_resource_op("application_entitlements_partial_update"),
            (
                "application_entitlements".to_owned(),
                "partial_update".to_owned()
            )
        );
        assert_eq!(
            split_resource_op("application_entitlements_used_by_list"),
            (
                "application_entitlements".to_owned(),
                "used_by_list".to_owned()
            )
        );
    }

    #[test]
    fn split_rightmost_verb() {
        // list is rightmost CRUD verb → resource = iterations, op = list_latest
        assert_eq!(
            split_resource_op("iterations_list_latest"),
            ("iterations".to_owned(), "list_latest".to_owned())
        );
        assert_eq!(
            split_resource_op("applications_check_access_retrieve"),
            (
                "applications_check_access".to_owned(),
                "retrieve".to_owned()
            )
        );
    }

    #[test]
    fn split_single_word() {
        assert_eq!(
            split_resource_op("retrieve"),
            (String::new(), "retrieve".to_owned())
        );
    }

    #[test]
    fn list_fn_no_extra_params() {
        let file = parse(
            r#"pub async fn admin_apps_list(
                configuration: &configuration::Configuration,
            ) -> Result<Vec<models::App>, Error<AdminAppsListError>> { unimplemented!() }"#,
        );
        let fns = extract_functions("admin", &file);
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].full_name, "admin_apps_list");
        assert_eq!(fns[0].resource, "apps");
        assert_eq!(fns[0].operation, "list");
        assert!(fns[0].params.is_empty());
        assert!(!fns[0].returns_unit);
    }

    #[test]
    fn list_fn_optional_params() {
        let file = parse(
            r#"pub async fn core_applications_list(
                configuration: &configuration::Configuration,
                name: Option<&str>,
                page: Option<i32>,
                only_with_launch_url: Option<bool>,
            ) -> Result<models::PaginatedApplicationList, Error<E>> { unimplemented!() }"#,
        );
        let fns = extract_functions("core", &file);
        assert_eq!(fns[0].resource, "applications");
        assert_eq!(fns[0].operation, "list");
        assert_eq!(fns[0].params.len(), 3);
        assert!(matches!(fns[0].params[0].ty, ParamType::OptionalStr));
        assert!(matches!(fns[0].params[1].ty, ParamType::OptionalInt));
        assert!(matches!(fns[0].params[2].ty, ParamType::OptionalBool));
    }

    #[test]
    fn create_fn_required_model_body() {
        let file = parse(
            r#"pub async fn core_applications_create(
                configuration: &configuration::Configuration,
                application_request: models::ApplicationRequest,
            ) -> Result<models::Application, Error<E>> { unimplemented!() }"#,
        );
        let fns = extract_functions("core", &file);
        assert_eq!(fns[0].resource, "applications");
        assert_eq!(fns[0].operation, "create");
        assert_eq!(fns[0].params.len(), 1);
        assert!(
            matches!(&fns[0].params[0].ty, ParamType::RequiredModel(n, _) if n == "ApplicationRequest")
        );
    }

    #[test]
    fn destroy_fn_returns_unit() {
        let file = parse(
            r#"pub async fn core_applications_destroy(
                configuration: &configuration::Configuration,
                slug: &str,
            ) -> Result<(), Error<E>> { unimplemented!() }"#,
        );
        let fns = extract_functions("core", &file);
        assert_eq!(fns[0].resource, "applications");
        assert_eq!(fns[0].operation, "destroy");
        assert!(fns[0].returns_unit);
        assert!(matches!(fns[0].params[0].ty, ParamType::RequiredStr));
    }

    #[test]
    fn optional_model_body() {
        let file = parse(
            r#"pub async fn core_applications_partial_update(
                configuration: &configuration::Configuration,
                slug: &str,
                patched_application_request: Option<models::PatchedApplicationRequest>,
            ) -> Result<models::Application, Error<E>> { unimplemented!() }"#,
        );
        let fns = extract_functions("core", &file);
        assert_eq!(fns[0].resource, "applications");
        assert_eq!(fns[0].operation, "partial_update");
        assert!(matches!(fns[0].params[0].ty, ParamType::RequiredStr));
        assert!(
            matches!(&fns[0].params[1].ty, ParamType::OptionalModel(n, _) if n == "PatchedApplicationRequest")
        );
    }

    #[test]
    fn vec_str_param() {
        let file = parse(
            r#"pub async fn events_events_list(
                configuration: &configuration::Configuration,
                managed: Option<Vec<String>>,
            ) -> Result<(), Error<E>> { unimplemented!() }"#,
        );
        let fns = extract_functions("events", &file);
        assert_eq!(fns[0].resource, "events");
        assert_eq!(fns[0].operation, "list");
        assert!(matches!(fns[0].params[0].ty, ParamType::OptionalVecStr));
    }

    #[test]
    fn cli_flag_names() {
        assert_eq!(to_cli_flag("page_size"), "page-size");
        assert_eq!(to_cli_flag("name__iexact"), "name-iexact");
        assert_eq!(to_cli_flag("source__slug"), "source-slug");
        assert_eq!(to_cli_flag("managed__isnull"), "managed-isnull");
    }

    #[test]
    fn sanitize_double_underscore() {
        assert_eq!(sanitize_field_name("name__iexact"), "name_iexact");
        assert_eq!(sanitize_field_name("source__slug"), "source_slug");
    }

    #[test]
    fn skips_non_pub_async() {
        let file = parse(
            r#"
            fn private_fn(configuration: &configuration::Configuration) {}
            pub fn sync_pub_fn(configuration: &configuration::Configuration) -> () {}
            pub async fn admin_apps_list(configuration: &configuration::Configuration) -> Result<(), ()> { unimplemented!() }
            "#,
        );
        let fns = extract_functions("admin", &file);
        assert_eq!(fns.len(), 1);
    }

    #[test]
    fn skips_functions_without_module_prefix() {
        let file = parse(
            r#"
            pub async fn core_apps_list(configuration: &configuration::Configuration) -> Result<(), ()> { unimplemented!() }
            pub async fn other_apps_list(configuration: &configuration::Configuration) -> Result<(), ()> { unimplemented!() }
            "#,
        );
        let fns = extract_functions("core", &file);
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].full_name, "core_apps_list");
    }
}
