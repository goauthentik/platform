//! Origin allowlisting for the HTTP tools.
//!
//! An agent identity may only reach hosts that are *adjacent* to the application
//! it exchanged a token for: same scheme, same registrable domain (eTLD+1).

use std::fmt;

use url::{Host, Url};

/// A scheme plus the domain (or bare host) requests are allowed to reach.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AllowedOrigin {
    scheme: String,
    host: HostMatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum HostMatch {
    /// Registrable domain (eTLD+1); the domain itself and any subdomain match.
    Domain(String),
    /// No registrable domain (`localhost`, IP literal); the host must match exactly.
    Exact(String),
}

impl AllowedOrigin {
    /// Derive an allowlist entry from a configured URL, such as an application's
    /// launch URL. Returns `None` for unparseable URLs, non-http(s) schemes and
    /// URLs without a host.
    pub(crate) fn from_url(raw: &str) -> Option<Self> {
        let url = Url::parse(raw).ok()?;
        if url.scheme() != "http" && url.scheme() != "https" {
            return None;
        }
        Some(Self {
            scheme: url.scheme().to_owned(),
            host: host_match(&url)?,
        })
    }

    /// Whether `url` is adjacent to this origin. The port is deliberately
    /// ignored — adjacent services routinely live on a different one.
    pub(crate) fn allows(&self, url: &Url) -> bool {
        if url.scheme() != self.scheme {
            return false;
        }
        host_match(url).is_some_and(|host| host == self.host)
    }
}

impl fmt::Display for AllowedOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.host {
            HostMatch::Domain(domain) => write!(f, "{}://*.{domain}", self.scheme),
            HostMatch::Exact(host) => write!(f, "{}://{host}", self.scheme),
        }
    }
}

/// Reduce a URL's host to the value origin comparisons are made on.
///
/// IP literals are matched exactly — the public suffix list would happily reduce
/// `127.0.0.1` to the nonsense "registrable domain" `0.1`, which every other
/// `*.*.0.1` address would then match too.
fn host_match(url: &Url) -> Option<HostMatch> {
    match url.host()? {
        Host::Domain(domain) => {
            let domain = domain.to_ascii_lowercase();
            Some(match psl::domain_str(&domain) {
                Some(registrable) => HostMatch::Domain(registrable.to_ascii_lowercase()),
                None => HostMatch::Exact(domain),
            })
        }
        Host::Ipv4(addr) => Some(HostMatch::Exact(addr.to_string())),
        Host::Ipv6(addr) => Some(HostMatch::Exact(addr.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(raw: &str) -> AllowedOrigin {
        AllowedOrigin::from_url(raw).expect("origin should be derivable")
    }

    fn allows(allowed: &AllowedOrigin, raw: &str) -> bool {
        allowed.allows(&Url::parse(raw).expect("test URL should parse"))
    }

    /// Hosts sharing the launch URL's registrable domain are reachable,
    /// regardless of subdomain, port or path.
    #[test]
    fn test_adjacent_hosts_allowed() {
        let allowed = origin("https://app.example.com/dashboard");
        assert!(allows(&allowed, "https://app.example.com/api/v1/foo"));
        assert!(allows(&allowed, "https://api.example.com/"));
        assert!(allows(&allowed, "https://example.com/"));
        assert!(allows(&allowed, "https://eu.api.example.com:8443/x"));
    }

    /// A host that merely *contains* the allowed domain resolves to a different
    /// registrable domain and must not match.
    #[test]
    fn test_suffix_confusion_denied() {
        let allowed = origin("https://app.example.com/");
        assert!(!allows(&allowed, "https://example.com.evil.com/"));
        assert!(!allows(&allowed, "https://notexample.com/"));
        assert!(!allows(&allowed, "https://example.com.br/"));
    }

    /// The scheme is part of the origin and is compared exactly.
    #[test]
    fn test_scheme_must_match() {
        let allowed = origin("https://app.example.com/");
        assert!(!allows(&allowed, "http://api.example.com/"));
        assert!(!allows(&allowed, "ftp://api.example.com/"));
    }

    /// Multi-label public suffixes are honoured, so `co.uk` is not treated as a
    /// registrable domain that any UK host could match.
    #[test]
    fn test_multi_label_public_suffix() {
        let allowed = origin("https://app.example.co.uk/");
        assert!(allows(&allowed, "https://api.example.co.uk/"));
        assert!(!allows(&allowed, "https://other.co.uk/"));
    }

    /// Hosts with no registrable domain fall back to an exact host match, but
    /// still ignore the port — the adjacent dev service is on another one.
    #[test]
    fn test_hosts_without_registrable_domain() {
        let allowed = origin("http://localhost:8000/");
        assert!(allows(&allowed, "http://localhost:9000/api"));
        assert!(allows(&allowed, "http://localhost/"));
        assert!(!allows(&allowed, "http://otherhost:9000/"));
        assert!(!allows(&allowed, "http://localhost.example.com/"));
    }

    /// IP literals are compared exactly rather than run through the suffix list.
    #[test]
    fn test_ip_literals() {
        let allowed = origin("http://127.0.0.1:8000/");
        assert!(allows(&allowed, "http://127.0.0.1:9000/api"));
        assert!(!allows(&allowed, "http://127.0.0.2:8000/"));
        assert!(!allows(&allowed, "http://192.168.0.1/"));

        let allowed = origin("http://[::1]:8000/");
        assert!(allows(&allowed, "http://[::1]:9000/"));
        assert!(!allows(&allowed, "http://[::2]/"));
    }

    /// Host comparison is case-insensitive.
    #[test]
    fn test_case_insensitive() {
        let allowed = origin("https://App.Example.COM/");
        assert!(allows(&allowed, "https://API.EXAMPLE.com/"));
    }

    /// Launch URLs carrying authentik placeholders still yield an origin, since
    /// only the scheme and host are read.
    #[test]
    fn test_launch_url_with_placeholder() {
        let allowed = origin("https://app.example.com/u/%(username)s");
        assert!(allows(&allowed, "https://app.example.com/"));
    }

    /// Launch URLs that aren't usable http(s) URLs yield no origin at all.
    #[test]
    fn test_unusable_launch_urls() {
        assert!(AllowedOrigin::from_url("").is_none());
        assert!(AllowedOrigin::from_url("/relative/path").is_none());
        assert!(AllowedOrigin::from_url("blank://blank").is_none());
        assert!(AllowedOrigin::from_url("mailto:someone@example.com").is_none());
    }

    /// The rendered form is what the refusal message shows the caller.
    #[test]
    fn test_display() {
        assert_eq!(
            origin("https://app.example.com/x").to_string(),
            "https://*.example.com"
        );
        assert_eq!(
            origin("http://localhost:8000/").to_string(),
            "http://localhost"
        );
    }
}
