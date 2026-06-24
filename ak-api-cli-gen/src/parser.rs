use std::{collections::HashSet, path::Path};
use syn::Item;

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
    RequiredModel(String),
    /// An optional request body (model type name ends with `Request`). Accepted as JSON.
    OptionalModel(String),
    /// An optional enum/filter param (model type that is NOT a `*Request`).
    /// Accepted as a plain string and parsed via serde.
    OptionalEnum(String),
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

/// Context-aware split: choose the resource/operation boundary by finding the longest
/// prefix `P` such that another function in the same module has suffix `P_{canonical_op}`.
///
/// This correctly handles custom actions like `users_set_password_create`:
/// because `users_list`, `users_retrieve`, etc. exist, `users` is identified as the resource
/// and `set_password_create` becomes the operation.
///
/// Falls back to [`split_resource_op`] when no sibling evidence is found.
pub fn split_with_context(suffix: &str, module_suffixes: &HashSet<String>) -> (String, String) {
    let words: Vec<&str> = suffix.split('_').collect();

    if words.len() == 1 {
        return (String::new(), words[0].to_owned());
    }

    // Try each prefix from longest to shortest. A prefix is accepted as the resource when
    // a *different* function in the same module has `{prefix}_{canonical_op}` as its suffix.
    for i in (1..words.len()).rev() {
        let prefix = words[..i].join("_");
        for &op in CANONICAL_OPS {
            let candidate = format!("{prefix}_{op}");
            if candidate != suffix && module_suffixes.contains(&candidate) {
                let operation = words[i..].join("_");
                return (prefix, operation);
            }
        }
    }

    split_resource_op(suffix)
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
    if words.len() >= 3 && &words[words.len() - 3..] == ["used", "by", "list"] {
        return (words[..words.len() - 3].join("_"), "used_by_list".to_owned());
    }

    // 2-word compound ops.
    if words.len() >= 2 && &words[words.len() - 2..] == ["partial", "update"] {
        return (words[..words.len() - 2].join("_"), "partial_update".to_owned());
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
}

pub fn parse_all_apis(
    apis_dir: &Path,
) -> Result<Vec<ApiFunction>, Box<dyn std::error::Error + Send + Sync>> {
    let entries = std::fs::read_dir(apis_dir)?;
    let mut paths: Vec<_> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
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
        let module_suffixes: HashSet<String> =
            raw_fns.iter().map(|f| f.suffix.clone()).collect();
        for raw in raw_fns {
            let (resource, operation) =
                split_with_context(&raw.suffix, &module_suffixes);
            all_fns.push(ApiFunction {
                module: raw.module.clone(),
                full_name: raw.full_name.clone(),
                resource,
                operation,
                params: raw.params.clone(),
                returns_unit: raw.returns_unit,
                returns_response: raw.returns_response,
            });
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
    })
}

/// Extract functions from a parsed file using only the fallback heuristic (no module context).
/// Suitable for unit tests; production code goes through `parse_all_apis`.
pub fn extract_functions(module: &str, file: &syn::File) -> Vec<ApiFunction> {
    collect_raw(module, file)
        .into_iter()
        .map(|raw| {
            let (resource, operation) = split_resource_op(&raw.suffix);
            ApiFunction {
                module: raw.module,
                full_name: raw.full_name,
                resource,
                operation,
                params: raw.params,
                returns_unit: raw.returns_unit,
                returns_response: raw.returns_response,
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
            Some(ApiParam { name, field_name, cli_flag, ty })
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
        "Option<i32>" | "Option<i64>" | "Option<u32>" | "Option<u64>" => {
            ParamType::OptionalInt
        }
        "Option<bool>" => ParamType::OptionalBool,
        "Option<Vec<String>>" => ParamType::OptionalVecStr,
        "Option<Vec<uuid::Uuid>>" => ParamType::OptionalVecUuid,
        "Option<Vec<i32>>" | "Option<Vec<i64>>" | "Option<Vec<u32>>" | "Option<Vec<u64>>" => {
            ParamType::OptionalVecInt
        }
        "Option<std::path::PathBuf>" => ParamType::OptionalFile,
        _ if s.starts_with("models::") => {
            let name = last_path_segment(s);
            ParamType::RequiredModel(name)
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
                ParamType::OptionalModel(inner.to_owned())
            } else {
                ParamType::OptionalEnum(inner.to_owned())
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
        "type" | "use" | "loop" | "for" | "if" | "else" | "match" | "fn" | "struct"
        | "enum" | "impl" | "trait" | "return" | "let" | "const" | "static" | "pub"
        | "mod" | "super" | "self" | "Self" | "where" | "async" | "await" | "move"
        | "in" | "while" | "break" | "continue" | "ref" | "mut" | "extern" | "crate"
        | "dyn" | "as" | "true" | "false" | "unsafe" | "yield" | "abstract"
        | "become" | "do" | "final" | "override" | "priv" | "typeof" | "unsized"
        | "virtual" | "try" => format!("{s}_field"),
        _ => s,
    }
}

fn to_cli_flag(name: &str) -> String {
    name.replace("__", "-").replace('_', "-")
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
        // users_set_password_create: "users" is the resource because users_list exists as sibling.
        let ctx = suffixes(&[
            "users_list",
            "users_retrieve",
            "users_create",
            "users_destroy",
            "users_set_password_create",
        ]);
        assert_eq!(
            split_with_context("users_set_password_create", &ctx),
            ("users".to_owned(), "set_password_create".to_owned())
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
        let ctx = suffixes(&["iterations_list", "iterations_list_latest", "iterations_list_open"]);
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
            matches!(&fns[0].params[0].ty, ParamType::RequiredModel(n) if n == "ApplicationRequest")
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
            matches!(&fns[0].params[1].ty, ParamType::OptionalModel(n) if n == "PatchedApplicationRequest")
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
