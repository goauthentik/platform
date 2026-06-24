use crate::parser::{ApiFunction, ApiParam, ModelField, ParamType};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use std::collections::{BTreeMap, HashSet};

/// Top-level entry point: generate everything from a flat list of API functions.
///
/// The generated hierarchy is:
///   `ApiCommand` (module) → `{Module}Command` (resource) → `{Module}{Resource}Command` (operation)
///
/// Functions with an empty resource (e.g. `schema_retrieve`) skip the resource level and appear
/// directly under the module command enum.
pub fn generate_all(functions: &[ApiFunction]) -> TokenStream {
    // Group: module → resource → functions (BTreeMap for deterministic order).
    let mut by_module: BTreeMap<&str, BTreeMap<&str, Vec<&ApiFunction>>> = BTreeMap::new();
    for f in functions {
        by_module
            .entry(f.module.as_str())
            .or_default()
            .entry(f.resource.as_str())
            .or_default()
            .push(f);
    }

    let module_names: Vec<&str> = by_module.keys().copied().collect();
    let top_level_enum = gen_top_level_enum(&module_names);
    let top_level_impl = gen_top_level_impl(&module_names);
    let modules_const = gen_modules_const(&module_names);
    let fn_count = functions.len();
    let fn_count_const = quote! { pub const API_FUNCTION_COUNT: usize = #fn_count; };

    let module_items: Vec<TokenStream> = by_module
        .iter()
        .map(|(module, resources)| gen_module(module, resources))
        .collect();

    let helper_fn = quote! {
        fn __json_field_value(s: &str) -> serde_json::Value {
            serde_json::from_str(s).unwrap_or_else(|_| serde_json::Value::String(s.to_owned()))
        }
    };

    quote! {
        #helper_fn
        #top_level_enum
        #top_level_impl
        #modules_const
        #fn_count_const
        #(#module_items)*
    }
}

// ── Top-level ApiCommand enum ─────────────────────────────────────────────────

fn gen_top_level_enum(modules: &[&str]) -> TokenStream {
    let variants: Vec<TokenStream> = modules
        .iter()
        .map(|m| {
            let variant = module_variant_ident(m);
            let args = module_args_ident(m);
            let doc = format!("{} API", capitalize_first(m));
            quote! {
                #[doc = #doc]
                #variant(Box<#args>),
            }
        })
        .collect();

    quote! {
        #[derive(Debug, Clone, clap::Subcommand)]
        pub enum ApiCommand {
            #(#variants)*
        }
    }
}

fn gen_top_level_impl(modules: &[&str]) -> TokenStream {
    let arms: Vec<TokenStream> = modules
        .iter()
        .map(|m| {
            let variant = module_variant_ident(m);
            quote! { ApiCommand::#variant(args) => args.command.execute(config).await, }
        })
        .collect();

    quote! {
        impl ApiCommand {
            pub async fn execute(
                &self,
                config: &authentik_client::apis::configuration::Configuration,
            ) -> anyhow::Result<()> {
                match self {
                    #(#arms)*
                }
            }
        }
    }
}

fn gen_modules_const(modules: &[&str]) -> TokenStream {
    quote! { pub const API_MODULES: &[&str] = &[#(#modules),*]; }
}

// ── Module level ──────────────────────────────────────────────────────────────
//
// Each module command enum has two kinds of variants:
//   1. Resource variants  – point to `{Module}{Resource}Args { command: {Module}{Resource}Command }`.
//   2. Direct variants    – functions with empty resource, point straight to the args struct.

fn gen_module(module: &str, resources: &BTreeMap<&str, Vec<&ApiFunction>>) -> TokenStream {
    let args_ident = module_args_ident(module);
    let cmd_ident = module_command_ident(module);

    let mut variants: Vec<TokenStream> = Vec::new();
    let mut match_arms: Vec<TokenStream> = Vec::new();
    let mut items: Vec<TokenStream> = Vec::new();

    for (resource, fns) in resources {
        if resource.is_empty() {
            // Direct operations under the module (no resource sub-level).
            for f in fns {
                let variant = op_variant_ident(&f.operation);
                let args_struct = fn_args_struct_ident(f);
                let doc = format!("`{}`", f.full_name);
                variants.push(quote! {
                    #[doc = #doc]
                    #variant(Box<#args_struct>),
                });
                match_arms.push(quote! {
                    #cmd_ident::#variant(args) => args.execute(config).await,
                });
                items.push(gen_fn_item(f));
            }
        } else {
            // Resource sub-level.
            let res_variant = resource_variant_ident(resource);
            let res_args = resource_args_ident(module, resource);
            let doc = format!("`{}` operations", resource.replace('_', "-"));
            variants.push(quote! {
                #[doc = #doc]
                #res_variant(Box<#res_args>),
            });
            match_arms.push(quote! {
                #cmd_ident::#res_variant(args) => args.command.execute(config).await,
            });
            items.push(gen_resource(module, resource, fns));
        }
    }

    quote! {
        #[derive(Debug, Clone, clap::Args)]
        pub struct #args_ident {
            #[command(subcommand)]
            pub command: #cmd_ident,
        }

        #[derive(Debug, Clone, clap::Subcommand)]
        pub enum #cmd_ident {
            #(#variants)*
        }

        impl #cmd_ident {
            pub async fn execute(
                &self,
                config: &authentik_client::apis::configuration::Configuration,
            ) -> anyhow::Result<()> {
                match self {
                    #(#match_arms)*
                }
            }
        }

        #(#items)*
    }
}

// ── Resource level ────────────────────────────────────────────────────────────

fn gen_resource(module: &str, resource: &str, functions: &[&ApiFunction]) -> TokenStream {
    let args_ident = resource_args_ident(module, resource);
    let cmd_ident = resource_command_ident(module, resource);

    let variants: Vec<TokenStream> = functions
        .iter()
        .map(|f| {
            let variant = op_variant_ident(&f.operation);
            let args_struct = fn_args_struct_ident(f);
            let doc = format!("`{}`", f.full_name);
            quote! {
                #[doc = #doc]
                #variant(Box<#args_struct>),
            }
        })
        .collect();

    let arms: Vec<TokenStream> = functions
        .iter()
        .map(|f| {
            let variant = op_variant_ident(&f.operation);
            quote! { #cmd_ident::#variant(args) => args.execute(config).await, }
        })
        .collect();

    let fn_items: Vec<TokenStream> = functions.iter().map(|f| gen_fn_item(f)).collect();

    quote! {
        #[derive(Debug, Clone, clap::Args)]
        pub struct #args_ident {
            #[command(subcommand)]
            pub command: #cmd_ident,
        }

        #[derive(Debug, Clone, clap::Subcommand)]
        pub enum #cmd_ident {
            #(#variants)*
        }

        impl #cmd_ident {
            pub async fn execute(
                &self,
                config: &authentik_client::apis::configuration::Configuration,
            ) -> anyhow::Result<()> {
                match self {
                    #(#arms)*
                }
            }
        }

        #(#fn_items)*
    }
}

// ── Function-level args struct and execute impl ───────────────────────────────

/// Return a clone of `param` with model fields filtered to exclude any whose `rust_ident`
/// matches a name already used by another (non-model) param in the same function.
fn filter_model_fields(param: &ApiParam, occupied: &HashSet<String>) -> ApiParam {
    let ty = match &param.ty {
        ParamType::RequiredModel(name, fields) => {
            let filtered: Vec<ModelField> = fields
                .iter()
                .filter(|f| !occupied.contains(&f.rust_ident))
                .cloned()
                .collect();
            ParamType::RequiredModel(name.clone(), filtered)
        }
        ParamType::OptionalModel(name, fields) => {
            let filtered: Vec<ModelField> = fields
                .iter()
                .filter(|f| !occupied.contains(&f.rust_ident))
                .cloned()
                .collect();
            ParamType::OptionalModel(name.clone(), filtered)
        }
        other => other.clone(),
    };
    ApiParam {
        name: param.name.clone(),
        field_name: param.field_name.clone(),
        cli_flag: param.cli_flag.clone(),
        ty,
    }
}

fn gen_fn_item(f: &ApiFunction) -> TokenStream {
    let struct_ident = fn_args_struct_ident(f);
    let module_mod = format_ident!("{}_api", f.module);
    let fn_ident = format_ident!("{}", f.full_name);

    // Collect field names used by non-model params so we can exclude conflicting model fields.
    let non_model_field_names: HashSet<String> = f
        .params
        .iter()
        .filter(|p| {
            !matches!(
                &p.ty,
                ParamType::RequiredModel(_, _) | ParamType::OptionalModel(_, _)
            )
        })
        .map(|p| p.field_name.clone())
        .collect();

    // Build params with model fields filtered to remove conflicts.
    let params: Vec<ApiParam> = f
        .params
        .iter()
        .map(|p| filter_model_fields(p, &non_model_field_names))
        .collect();

    // Assign positional indexes to required path params (RequiredStr, RequiredInt).
    let mut pos_index: usize = 1;
    let fields: Vec<TokenStream> = params
        .iter()
        .map(|p| gen_field(p, &mut pos_index))
        .collect();

    let mut conversions: Vec<TokenStream> = Vec::new();
    let mut call_args: Vec<TokenStream> = Vec::new();
    for param in &params {
        let (conv, arg) = gen_param_conversion(param);
        if let Some(c) = conv {
            conversions.push(c);
        }
        call_args.push(arg);
    }

    let result_handling = if f.returns_unit {
        quote! {
            #(#conversions)*
            authentik_client::apis::#module_mod::#fn_ident(config, #(#call_args),*).await?;
            println!("ok");
            Ok(())
        }
    } else if f.returns_response {
        quote! {
            #(#conversions)*
            let response = authentik_client::apis::#module_mod::#fn_ident(config, #(#call_args),*).await?;
            let body = response.text().await
                .map_err(|e| anyhow::anyhow!("failed to read response body: {e}"))?;
            print!("{}", body);
            Ok(())
        }
    } else {
        quote! {
            #(#conversions)*
            let result = authentik_client::apis::#module_mod::#fn_ident(config, #(#call_args),*).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(())
        }
    };

    quote! {
        #[derive(Debug, Clone, clap::Args, Default)]
        pub struct #struct_ident {
            #(#fields)*
        }

        impl #struct_ident {
            pub async fn execute(
                &self,
                config: &authentik_client::apis::configuration::Configuration,
            ) -> anyhow::Result<()> {
                #result_handling
            }
        }
    }
}

/// Generate a single struct field for a parameter.
/// `pos_index` is incremented for each positional (required path param) field.
fn gen_field(param: &ApiParam, pos_index: &mut usize) -> TokenStream {
    let flag = &param.cli_flag;
    let field = format_ident!("{}", &param.field_name);

    match &param.ty {
        // Required path params → positional arguments (no --flag)
        ParamType::RequiredStr => {
            let idx = *pos_index;
            *pos_index += 1;
            quote! {
                #[arg(index = #idx)]
                pub #field: String,
            }
        }
        ParamType::RequiredInt => {
            let idx = *pos_index;
            *pos_index += 1;
            quote! {
                #[arg(index = #idx)]
                pub #field: i32,
            }
        }

        // Everything else stays as named flags
        ParamType::OptionalStr => quote! {
            #[arg(long = #flag)]
            pub #field: Option<String>,
        },
        ParamType::OptionalInt => quote! {
            #[arg(long = #flag)]
            pub #field: Option<i32>,
        },
        ParamType::OptionalBool => quote! {
            #[arg(long = #flag)]
            pub #field: Option<String>,
        },
        ParamType::RequiredModel(_, fields) | ParamType::OptionalModel(_, fields) => {
            let field_flags: Vec<TokenStream> = fields
                .iter()
                .map(|f| {
                    let flag = &f.cli_flag;
                    let ident = format_ident!("{}", &f.rust_ident);
                    let help_attr = match &f.help {
                        Some(h) => quote! { help = #h, },
                        None => quote! {},
                    };
                    quote! {
                        #[arg(long = #flag, #help_attr)]
                        pub #ident: Option<String>,
                    }
                })
                .collect();
            quote! {
                #[arg(long = "body", help = "JSON body (individual field flags override specific fields)")]
                pub body: Option<String>,
                #(#field_flags)*
            }
        }
        ParamType::OptionalEnum(_) => quote! {
            #[arg(long = #flag)]
            pub #field: Option<String>,
        },
        ParamType::RequiredVecStr => quote! {
            #[arg(long = #flag, num_args = 1..)]
            pub #field: Vec<String>,
        },
        ParamType::RequiredVecInt => quote! {
            #[arg(long = #flag, num_args = 1..)]
            pub #field: Vec<String>,
        },
        ParamType::OptionalVecStr => quote! {
            #[arg(long = #flag, num_args = 0..)]
            pub #field: Vec<String>,
        },
        ParamType::OptionalVecInt => quote! {
            #[arg(long = #flag, num_args = 0..)]
            pub #field: Vec<String>,
        },
        ParamType::OptionalVecUuid => quote! {
            #[arg(long = #flag, num_args = 0..)]
            pub #field: Vec<String>,
        },
        ParamType::OptionalVecModel(_) => quote! {
            #[arg(long = #flag, num_args = 0.., help = "JSON-encoded values")]
            pub #field: Vec<String>,
        },
        ParamType::OptionalChronoStr => quote! {
            #[arg(long = #flag, help = "RFC 3339 datetime string")]
            pub #field: Option<String>,
        },
        ParamType::RequiredFile => quote! {
            #[arg(long = #flag)]
            pub #field: std::path::PathBuf,
        },
        ParamType::OptionalFile => quote! {
            #[arg(long = #flag)]
            pub #field: Option<std::path::PathBuf>,
        },
    }
}

fn gen_param_conversion(param: &ApiParam) -> (Option<TokenStream>, TokenStream) {
    let field = format_ident!("{}", &param.field_name);

    match &param.ty {
        ParamType::RequiredStr => (None, quote! { &self.#field }),
        ParamType::OptionalStr => (None, quote! { self.#field.as_deref() }),
        ParamType::RequiredInt => (None, quote! { self.#field }),
        ParamType::OptionalInt => (None, quote! { self.#field }),
        ParamType::RequiredFile => (None, quote! { self.#field.clone() }),
        ParamType::OptionalFile => (None, quote! { self.#field.clone() }),
        ParamType::RequiredVecStr => (None, quote! { self.#field.clone() }),

        ParamType::RequiredVecInt => {
            let local = format_ident!("_vec_{}", &param.field_name);
            (
                Some(quote! {
                    let #local: Vec<i32> = self.#field
                        .iter()
                        .map(|s| s.parse::<i32>().map_err(|e| anyhow::anyhow!("{e}")))
                        .collect::<Result<Vec<_>, _>>()?;
                }),
                quote! { #local },
            )
        }

        ParamType::OptionalBool => {
            let local = format_ident!("_bool_{}", &param.field_name);
            let flag = &param.cli_flag;
            (
                Some(quote! {
                    let #local: Option<bool> = self.#field
                        .as_deref()
                        .map(|s| s.parse::<bool>()
                            .map_err(|e| anyhow::anyhow!("invalid bool for --{}: {e}", #flag)))
                        .transpose()?;
                }),
                quote! { #local },
            )
        }

        ParamType::RequiredModel(type_name, fields) => {
            let type_ident = format_ident!("{}", type_name);
            let local = format_ident!("_body_{}", &param.name.replace("__", "_"));
            let field_overrides: Vec<TokenStream> = fields
                .iter()
                .map(|f| {
                    let ident = format_ident!("{}", &f.rust_ident);
                    let key = &f.json_key;
                    quote! {
                        if let Some(__v) = &self.#ident {
                            __obj.insert(#key.to_owned(), __json_field_value(__v));
                        }
                    }
                })
                .collect();
            (
                Some(quote! {
                    let mut __body_json: serde_json::Value = match &self.body {
                        Some(b) => serde_json::from_str(b)?,
                        None => serde_json::Value::Object(Default::default()),
                    };
                    let __obj = __body_json.as_object_mut()
                        .ok_or_else(|| anyhow::anyhow!("--body must be a JSON object"))?;
                    #(#field_overrides)*
                    let #local: authentik_client::models::#type_ident =
                        serde_json::from_value(__body_json)?;
                }),
                quote! { #local },
            )
        }

        ParamType::OptionalModel(type_name, fields) => {
            let type_ident = format_ident!("{}", type_name);
            let local = format_ident!("_body_{}", &param.name.replace("__", "_"));
            let field_idents_vec: Vec<proc_macro2::Ident> = fields
                .iter()
                .map(|f| format_ident!("{}", &f.rust_ident))
                .collect();
            let field_overrides: Vec<TokenStream> = fields
                .iter()
                .map(|f| {
                    let ident = format_ident!("{}", &f.rust_ident);
                    let key = &f.json_key;
                    quote! {
                        if let Some(__v) = &self.#ident {
                            __obj.insert(#key.to_owned(), __json_field_value(__v));
                        }
                    }
                })
                .collect();
            let has_any_input = quote! {
                self.body.is_some() #(|| self.#field_idents_vec.is_some())*
            };
            (
                Some(quote! {
                    let #local: Option<authentik_client::models::#type_ident> = if !{ #has_any_input } {
                        None
                    } else {
                        let mut __body_json: serde_json::Value = match &self.body {
                            Some(b) => serde_json::from_str(b)?,
                            None => serde_json::Value::Object(Default::default()),
                        };
                        let __obj = __body_json.as_object_mut()
                            .ok_or_else(|| anyhow::anyhow!("--body must be a JSON object"))?;
                        #(#field_overrides)*
                        Some(serde_json::from_value(__body_json)?)
                    };
                }),
                quote! { #local },
            )
        }

        ParamType::OptionalEnum(type_name) => {
            let type_ident = format_ident!("{}", type_name);
            let local = format_ident!("_enum_{}", &param.field_name);
            let flag = &param.cli_flag;
            (
                Some(quote! {
                    let #local: Option<authentik_client::models::#type_ident> =
                        self.#field.as_deref()
                            .map(|s| {
                                let quoted = format!("\"{}\"", s);
                                serde_json::from_str::<authentik_client::models::#type_ident>(&quoted)
                                    .or_else(|_| serde_json::from_str(s))
                                    .map_err(|e| anyhow::anyhow!("invalid value for --{}: {}", #flag, e))
                            })
                            .transpose()?;
                }),
                quote! { #local },
            )
        }

        ParamType::OptionalVecStr => {
            let local = format_ident!("_vec_{}", &param.field_name);
            (
                Some(quote! {
                    let #local = if self.#field.is_empty() { None } else { Some(self.#field.clone()) };
                }),
                quote! { #local },
            )
        }

        ParamType::OptionalVecInt => {
            let local = format_ident!("_vec_{}", &param.field_name);
            (
                Some(quote! {
                    let #local: Option<Vec<i32>> = if self.#field.is_empty() {
                        None
                    } else {
                        Some(self.#field.iter()
                            .map(|s| s.parse::<i32>().map_err(|e| anyhow::anyhow!("{e}")))
                            .collect::<Result<Vec<_>, _>>()?)
                    };
                }),
                quote! { #local },
            )
        }

        ParamType::OptionalVecUuid => {
            let local = format_ident!("_vec_{}", &param.field_name);
            (
                Some(quote! {
                    let #local: Option<Vec<uuid::Uuid>> = if self.#field.is_empty() {
                        None
                    } else {
                        Some(self.#field.iter()
                            .map(|s| s.parse::<uuid::Uuid>().map_err(|e| anyhow::anyhow!("{e}")))
                            .collect::<Result<Vec<_>, _>>()?)
                    };
                }),
                quote! { #local },
            )
        }

        ParamType::OptionalVecModel(type_name) => {
            let type_ident = format_ident!("{}", type_name);
            let local = format_ident!("_vec_{}", &param.field_name);
            (
                Some(quote! {
                    let #local: Option<Vec<authentik_client::models::#type_ident>> =
                        if self.#field.is_empty() {
                            None
                        } else {
                            Some(self.#field.iter()
                                .map(|s| serde_json::from_str(s))
                                .collect::<Result<Vec<_>, _>>()?)
                        };
                }),
                quote! { #local },
            )
        }

        ParamType::OptionalChronoStr => {
            let local = format_ident!("_dt_{}", &param.field_name);
            (
                Some(quote! {
                    let #local: Option<chrono::DateTime<chrono::FixedOffset>> =
                        self.#field.as_deref()
                            .map(|s| {
                                s.parse::<chrono::DateTime<chrono::FixedOffset>>()
                                    .map_err(|e| anyhow::anyhow!("{e}"))
                            })
                            .transpose()?;
                }),
                quote! { #local },
            )
        }
    }
}

// ── Naming helpers ────────────────────────────────────────────────────────────

fn module_variant_ident(module: &str) -> Ident {
    Ident::new(&capitalize_first(module), Span::call_site())
}

fn module_args_ident(module: &str) -> Ident {
    Ident::new(
        &format!("{}Args", capitalize_first(module)),
        Span::call_site(),
    )
}

fn module_command_ident(module: &str) -> Ident {
    Ident::new(
        &format!("{}Command", capitalize_first(module)),
        Span::call_site(),
    )
}

fn resource_variant_ident(resource: &str) -> Ident {
    Ident::new(&to_pascal_case(resource), Span::call_site())
}

fn resource_args_ident(module: &str, resource: &str) -> Ident {
    Ident::new(
        &format!(
            "{}{}Args",
            capitalize_first(module),
            to_pascal_case(resource)
        ),
        Span::call_site(),
    )
}

fn resource_command_ident(module: &str, resource: &str) -> Ident {
    Ident::new(
        &format!(
            "{}{}Command",
            capitalize_first(module),
            to_pascal_case(resource)
        ),
        Span::call_site(),
    )
}

fn op_variant_ident(operation: &str) -> Ident {
    Ident::new(&to_pascal_case(operation), Span::call_site())
}

fn fn_args_struct_ident(f: &ApiFunction) -> Ident {
    Ident::new(
        &format!(
            "{}{}{}Args",
            capitalize_first(&f.module),
            to_pascal_case(&f.resource),
            to_pascal_case(&f.operation)
        ),
        Span::call_site(),
    )
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::parser::{ApiFunction, ApiParam, ParamType};

    fn make_list_fn() -> ApiFunction {
        ApiFunction {
            module: "core".to_owned(),
            full_name: "core_applications_list".to_owned(),
            resource: "applications".to_owned(),
            operation: "list".to_owned(),
            params: vec![
                ApiParam {
                    name: "name".to_owned(),
                    field_name: "name".to_owned(),
                    cli_flag: "name".to_owned(),
                    ty: ParamType::OptionalStr,
                },
                ApiParam {
                    name: "page".to_owned(),
                    field_name: "page".to_owned(),
                    cli_flag: "page".to_owned(),
                    ty: ParamType::OptionalInt,
                },
            ],
            returns_unit: false,
            returns_response: false,
        }
    }

    fn make_retrieve_fn() -> ApiFunction {
        ApiFunction {
            module: "core".to_owned(),
            full_name: "core_users_retrieve".to_owned(),
            resource: "users".to_owned(),
            operation: "retrieve".to_owned(),
            params: vec![ApiParam {
                name: "id".to_owned(),
                field_name: "id".to_owned(),
                cli_flag: "id".to_owned(),
                ty: ParamType::RequiredInt,
            }],
            returns_unit: false,
            returns_response: false,
        }
    }

    fn make_create_fn() -> ApiFunction {
        ApiFunction {
            module: "core".to_owned(),
            full_name: "core_applications_create".to_owned(),
            resource: "applications".to_owned(),
            operation: "create".to_owned(),
            params: vec![ApiParam {
                name: "application_request".to_owned(),
                field_name: "application_request".to_owned(),
                cli_flag: "application-request".to_owned(),
                ty: ParamType::RequiredModel("ApplicationRequest".to_owned(), vec![]),
            }],
            returns_unit: false,
            returns_response: false,
        }
    }

    fn make_destroy_fn() -> ApiFunction {
        ApiFunction {
            module: "core".to_owned(),
            full_name: "core_applications_destroy".to_owned(),
            resource: "applications".to_owned(),
            operation: "destroy".to_owned(),
            params: vec![ApiParam {
                name: "slug".to_owned(),
                field_name: "slug".to_owned(),
                cli_flag: "slug".to_owned(),
                ty: ParamType::RequiredStr,
            }],
            returns_unit: true,
            returns_response: false,
        }
    }

    fn make_direct_fn() -> ApiFunction {
        // e.g. schema_retrieve — no resource level
        ApiFunction {
            module: "schema".to_owned(),
            full_name: "schema_retrieve".to_owned(),
            resource: String::new(),
            operation: "retrieve".to_owned(),
            params: vec![],
            returns_unit: false,
            returns_response: false,
        }
    }

    #[test]
    fn generate_compiles_with_list_fn() {
        let tokens = generate_all(&[make_list_fn()]);
        assert!(syn::parse2::<syn::File>(tokens).is_ok());
    }

    #[test]
    fn generate_compiles_with_retrieve_fn() {
        let tokens = generate_all(&[make_retrieve_fn()]);
        assert!(syn::parse2::<syn::File>(tokens).is_ok());
    }

    #[test]
    fn generate_compiles_with_create_fn() {
        let tokens = generate_all(&[make_create_fn()]);
        assert!(syn::parse2::<syn::File>(tokens).is_ok());
    }

    #[test]
    fn generate_compiles_with_destroy_fn() {
        let tokens = generate_all(&[make_destroy_fn()]);
        assert!(syn::parse2::<syn::File>(tokens).is_ok());
    }

    #[test]
    fn generate_compiles_with_direct_fn() {
        let tokens = generate_all(&[make_direct_fn()]);
        assert!(syn::parse2::<syn::File>(tokens).is_ok());
    }

    #[test]
    fn generate_compiles_with_mixed_module() {
        // Same module, two resources
        let fns = vec![
            make_list_fn(),
            make_retrieve_fn(),
            make_create_fn(),
            make_destroy_fn(),
        ];
        let tokens = generate_all(&fns);
        assert!(syn::parse2::<syn::File>(tokens).is_ok());
    }

    #[test]
    fn module_naming_conventions() {
        assert_eq!(module_variant_ident("core").to_string(), "Core");
        assert_eq!(module_variant_ident("oauth2").to_string(), "Oauth2");
        assert_eq!(module_args_ident("admin").to_string(), "AdminArgs");
        assert_eq!(module_command_ident("flows").to_string(), "FlowsCommand");
    }

    #[test]
    fn resource_naming_conventions() {
        assert_eq!(resource_variant_ident("users").to_string(), "Users");
        assert_eq!(
            resource_variant_ident("application_entitlements").to_string(),
            "ApplicationEntitlements"
        );
        assert_eq!(
            resource_args_ident("core", "users").to_string(),
            "CoreUsersArgs"
        );
        assert_eq!(
            resource_command_ident("core", "users").to_string(),
            "CoreUsersCommand"
        );
    }

    #[test]
    fn op_naming_conventions() {
        assert_eq!(op_variant_ident("retrieve").to_string(), "Retrieve");
        assert_eq!(
            op_variant_ident("partial_update").to_string(),
            "PartialUpdate"
        );
        assert_eq!(op_variant_ident("used_by_list").to_string(), "UsedByList");
    }

    #[test]
    fn fn_struct_naming() {
        let f = make_retrieve_fn();
        assert_eq!(
            fn_args_struct_ident(&f).to_string(),
            "CoreUsersRetrieveArgs"
        );
    }

    #[test]
    fn modules_const_in_output() {
        let tokens = generate_all(&[make_list_fn()]);
        let code = tokens.to_string();
        assert!(code.contains("API_MODULES"));
        assert!(code.contains("\"core\""));
    }

    #[test]
    fn snapshot_list_fn() {
        let tokens = generate_all(&[make_list_fn()]);
        let parsed = syn::parse2::<syn::File>(tokens).unwrap();
        let code = prettyplease::unparse(&parsed);
        insta::assert_snapshot!(code);
    }
}
