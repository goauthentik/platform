use pam::{constants::PamResultCode, conv::Conv};

pub fn auth_interactive(_username: String, _password: &str, _conv: &Conv<'_>) -> PamResultCode {
    PamResultCode::PAM_IGNORE
}
