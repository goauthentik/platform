use std::env;

fn main() {
    let envs = vec!["AK_VERSION", "AK_BUILDHASH", "AK_TAG"];
    for key in envs {
        let v = match env::var(key) {
            Ok(v) => v,
            Err(_) => {
                println!("cargo::warning=Failed to get value for {}", key);
                "".to_string()
            }
        };
        println!("cargo:rustc-env={key}={v}");
    }
}
