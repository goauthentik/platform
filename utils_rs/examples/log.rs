use authentik_sys::logger::init_log;


fn main() {
    init_log("authentik WCP");
    log::debug!("foo");
}
