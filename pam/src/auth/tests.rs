use super::{AuthenticationAttempt, PW_PREFIX, parse_authentication_attempt};

#[test]
fn parses_interactive_passwords_without_prefix() {
    assert_eq!(
        parse_authentication_attempt("plain-password"),
        AuthenticationAttempt::Interactive
    );
}

#[test]
fn parses_prefixed_passwords_as_tokens() {
    assert_eq!(
        parse_authentication_attempt(&format!("{PW_PREFIX}encoded-token")),
        AuthenticationAttempt::Token("encoded-token".to_owned())
    );
}

#[test]
fn only_strips_the_leading_prefix_marker() {
    let password = format!("{PW_PREFIX}token{PW_PREFIX}body");

    assert_eq!(
        parse_authentication_attempt(&password),
        AuthenticationAttempt::Token(format!("token{PW_PREFIX}body"))
    );
}
