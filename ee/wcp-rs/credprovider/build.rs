fn main() {
    if std::env::var("CARGO_CFG_WINDOWS").is_ok()
        && let Err(result) = embed_resource::compile("res/resource.rc", embed_resource::NONE).manifest_optional()
    {
        println!("cargo::error={result}");
        std::process::exit(1);
    }
}
