use std::env;

pub fn version() -> String {
    env!("AK_VERSION").to_string()
}

pub fn build_hash() -> String {
    env!("AK_BUILDHASH").to_string()
}

pub fn tag() -> String {
    env!("AK_TAG").to_string()
}

pub fn full_version() -> String {
    let mut fw = version();
    if !build_hash().is_empty() && tag().is_empty() {
        let mut sh = build_hash();
        sh.truncate(8);
        fw += "-";
        fw += &sh;
    }
    fw
}

pub fn user_agent() -> String {
    format!("goauthentik.io/platform/{}", full_version())
}

pub fn build_url() -> String {
    format!(
        "https://github.com/goauthentik/platform/commit/{}",
        build_hash()
    )
}
