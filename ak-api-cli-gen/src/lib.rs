pub mod codegen;
pub mod parser;

use std::path::Path;

pub fn generate(apis_dir: &Path) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let functions = parser::parse_all_apis(apis_dir)?;
    let tokens = codegen::generate_all(&functions);
    let parsed = syn::parse2(tokens)?;
    Ok(prettyplease::unparse(&parsed))
}
