//! Parse authentik token-validity durations to seconds (for `cap`-binned attrs).

use crate::mcp::blueprint::yaml::Plain;

/// Parse an authentik token-validity value — a number of seconds, or a
/// timedelta string like `"hours=1;minutes=30"`. Returns `None` if unparseable.
/// Any unrecognized unit rejects the whole value; never silently ignored.
pub fn parse_token_duration(val: &Plain) -> Option<i64> {
    match val {
        Plain::Int(n) => Some(*n),
        Plain::Float(f) => Some(*f as i64),
        Plain::Str(s) => parse_str(s),
        _ => None,
    }
}

/// All ASCII digits and non-empty — the Rust equivalent of the TS `\d+` guard.
fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

fn parse_str(val: &str) -> Option<i64> {
    let trimmed = val.trim();
    if is_digits(trimmed) {
        return trimmed.parse::<i64>().ok();
    }

    let mut total: i64 = 0;
    let mut parsed = false;
    for part in val.split(';') {
        if part.trim().is_empty() {
            continue; // tolerate trailing/empty segments
        }
        let (unit, amount) = part.split_once('=')?;
        let unit = unit.trim();
        let amount = amount.trim();
        // Mirror the TS `^\s*(\w+)\s*=\s*(\d+)\s*$`: word-char unit, digit amount.
        if unit.is_empty()
            || !unit.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            || !is_digits(amount)
        {
            return None;
        }
        let n: i64 = amount.parse().ok()?;
        parsed = true;
        let seconds = match unit {
            "seconds" => n,
            "minutes" => n.checked_mul(60)?,
            "hours" => n.checked_mul(3600)?,
            "days" => n.checked_mul(86400)?,
            "weeks" => n.checked_mul(604800)?,
            _ => return None, // unknown unit → reject
        };
        total = total.checked_add(seconds)?;
    }

    if parsed { Some(total) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Option<i64> {
        parse_token_duration(&Plain::Str(v.to_string()))
    }

    #[test]
    fn numbers_pass_through_as_seconds() {
        assert_eq!(parse_token_duration(&Plain::Int(0)), Some(0));
        assert_eq!(parse_token_duration(&Plain::Int(3600)), Some(3600));
        assert_eq!(parse_token_duration(&Plain::Int(-5)), Some(-5));
    }

    #[test]
    fn bare_numeric_string_parses() {
        assert_eq!(s("10"), Some(10));
        assert_eq!(s("  3600  "), Some(3600));
    }

    #[test]
    fn single_units_convert() {
        assert_eq!(s("seconds=30"), Some(30));
        assert_eq!(s("minutes=2"), Some(120));
        assert_eq!(s("hours=1"), Some(3600));
        assert_eq!(s("days=1"), Some(86400));
        assert_eq!(s("weeks=1"), Some(604800));
    }

    #[test]
    fn multiple_units_sum() {
        assert_eq!(s("hours=1;minutes=30"), Some(5400));
        assert_eq!(s("days=1;hours=12"), Some(129600));
    }

    #[test]
    fn empty_and_trailing_segments_tolerated() {
        assert_eq!(s("hours=1;"), Some(3600));
        assert_eq!(s("hours=1;;minutes=1"), Some(3660));
    }

    #[test]
    fn unknown_unit_rejects_whole_value() {
        assert_eq!(s("fortnights=10"), None);
        assert_eq!(s("hours=1;fortnights=10"), None);
    }

    #[test]
    fn malformed_and_wrong_types_return_none() {
        assert_eq!(s(""), None);
        assert_eq!(s("   "), None);
        assert_eq!(s("abc"), None);
        assert_eq!(s("hours=abc"), None);
        assert_eq!(s("hours="), None);
        assert_eq!(parse_token_duration(&Plain::Null), None);
        assert_eq!(parse_token_duration(&Plain::Bool(true)), None);
        assert_eq!(parse_token_duration(&Plain::Map(vec![])), None);
    }
}
