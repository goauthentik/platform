pub mod codegen;
pub mod parser;

use std::path::Path;

/// Generated CLI source, split into a root file and one file per API module.
///
/// Splitting the output keeps each file small, which improves compile times for the crate that
/// writes these under its `OUT_DIR` and `include!`s them (see `ak-cli/build.rs`).
pub struct GeneratedFiles {
    /// Contents for the root file (conventionally `api_commands.rs`).
    pub root: String,
    /// `(file_name, contents)` pairs for each API module. `file_name` is the exact name the root
    /// file's `include!` expects — write each one under the same directory as the root file.
    pub modules: Vec<(String, String)>,
}

pub fn generate(
    apis_dir: &Path,
) -> Result<GeneratedFiles, Box<dyn std::error::Error + Send + Sync>> {
    let functions = parser::parse_all_apis(apis_dir)?;
    let generated = codegen::generate_all(&functions);
    let root = render(generated.root)?;
    let modules = generated
        .modules
        .into_iter()
        .map(|(name, tokens)| Ok((name, render(tokens)?)))
        .collect::<Result<Vec<_>, Box<dyn std::error::Error + Send + Sync>>>()?;
    Ok(GeneratedFiles { root, modules })
}

fn render(
    tokens: proc_macro2::TokenStream,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let parsed = syn::parse2(tokens)?;
    Ok(prettyplease::unparse(&parsed))
}
