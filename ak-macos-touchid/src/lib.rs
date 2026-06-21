#[swift_bridge::bridge]
mod ffi {
    extern "Swift" {
        fn authenticate_with_touchid(reason: String) -> bool;
        fn is_touchid_available() -> bool;
    }
}
