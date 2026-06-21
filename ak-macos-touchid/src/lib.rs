#[swift_bridge::bridge]
mod ffi {
    #[swift_bridge(swift_repr = "struct")]
    struct AuthRequest {
        reason: String,
    }
    extern "Swift" {
        fn authenticate_with_touchid(req: AuthRequest) -> bool;
        fn is_touchid_available() -> bool;
    }
}

pub struct AuthRequest {
    pub reason: String,
}

pub fn authenticate_with_touchid(req: AuthRequest) -> bool {
    ffi::authenticate_with_touchid(ffi::AuthRequest { reason: req.reason })
}

pub fn is_touchid_available() -> bool {
    ffi::is_touchid_available()
}
