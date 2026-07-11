fn main() {
    let facts = ak_platform_facts::gather();
    println!("{}", serde_json::to_string_pretty(&facts).unwrap_or_default());
}
