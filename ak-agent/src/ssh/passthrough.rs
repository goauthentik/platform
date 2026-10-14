use ssh_key::public::KeyData;

/// Built-in `known_hosts`-format entries for hosts that are never managed by
/// authentik. Matching one of these lets the agent skip straight to the
/// fallback agent instead of prompting the user and asking authentik for a
/// host token that will never succeed.
const WELL_KNOWN_HOSTS: &[&str] = &[
    "github.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl",
    "github.com ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBEmKSENjQEezOmxkZMy7opKgwFB9nkt5YRrYMjNuG5N87uRgg6CLrbo5wAdT/y6v0mKV0U2w0WZ2YB/++Tpockg=",
    "github.com ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQCj7ndNxQowgcQnjshcLrqPEiiphnt+VTTvDP6mHBL9j1aNUkY4Ue1gvwnGLVlOhGeYrnZaMgRK6+PKCUXaDbC7qtbW8gIkhL7aGCsOr/C56SJMy/BCZfxd1nWzAOxSDPgVsmerOBYfNqltV9/hWCqBywINIR+5dIg6JTJ72pcEpEjcYgXkE2YEFXV1JHnsKgbLWNlhScqb2UmyRkQyytRLtL+38TGxkxCflmO+5Z8CSSNY7GidjMIZ7Q4zMjA2n1nGrlTDkzwDCsw+wqFPGQA179cnfGWOWRVruj16z6XyvxvjJwbz0wQZ75XK5tKSb7FNyeIEs4TT4jk+S4dhPeAUC5y+bDYirYgM4GC7uEnztnZyaVWQ7B381AK4Qdrwt51ZqExKbQpTUNn+EjqoTwvqNj4kqx5QUCI0ThS/YkOxJCXmPUWZbhjpCg56i+2aB6CmK2JGhn57K5mj0MNdBXA4/WnwH6XoPWJzK5Nyu2zB3nAZp+S5hpQs+p1vN1/wsjk=",
    "gitlab.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAfuCHKVTjquxvt6CM6tdG4SLp1Btn/nOeHHE5UOzRdf",
    "gitlab.com ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBFSMqzJeV9rUzU4kWitGjeR4PWSa29SPqJ1fVkhtj3Hw9xjLVXVYrU9QlYWrOLXBpQ6KWjbjTDTdDkoohFzgbEY=",
    "gitlab.com ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQCsj2bNKTBSpIYDEGk9KxsGh3mySTRgMtXL583qmBpzeQ+jqCMRgBqB98u3z++J1sKlXHWfM9dyhSevkMwSbhoR8XIq/U0tCNyokEi/ueaBMCvbcTHhO7FcwzY92WK4Yt0aGROY5qX2UKSeOvuP4D6TPqKF1onrSzH9bx9XUf2lEdWT/ia1NEKjunUqu1xOB/StKDHMoX4/OKyIzuS0q/T1zOATthvasJFoPrAjkohTyaDUz2LN5JoH839hViyEG82yB+MjcFV5MU3N1l1QL3cVUCh93xSaua1N85qivl+siMkPGbO5xR/En4iEY6K2XPASUEMaieWVNTRCtJ4S8H+9",
];

/// Parses a single `known_hosts` line ("host [marker] algo base64 [comment]")
/// and returns the key it encodes, ignoring the leading hostname field(s).
fn parse_known_hosts_line(line: &str) -> Option<KeyData> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (_host, key_part) = line.split_once(char::is_whitespace)?;
    ssh_key::PublicKey::from_openssh(key_part.trim())
        .ok()
        .map(|k| k.key_data().clone())
}

/// Whether `host_key` belongs to a host that should always bypass authentik,
/// either because it's a well-known git host or because it was listed in
/// `extra_hosts` (both given as `known_hosts`-format lines).
pub fn is_passthrough_host(host_key: &KeyData, extra_hosts: &[String]) -> bool {
    WELL_KNOWN_HOSTS
        .iter()
        .copied()
        .chain(extra_hosts.iter().map(String::as_str))
        .filter_map(parse_known_hosts_line)
        .any(|k| &k == host_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_well_known_github_key() {
        let key = parse_known_hosts_line(WELL_KNOWN_HOSTS[0]).expect("built-in key must parse");
        assert!(is_passthrough_host(&key, &[]));
    }

    #[test]
    fn matches_user_supplied_extra_host() {
        let extra = vec![WELL_KNOWN_HOSTS[3].to_string()]; // gitlab.com ed25519
        let key = parse_known_hosts_line(&extra[0]).unwrap();
        assert!(is_passthrough_host(&key, &extra));
    }

    #[test]
    fn does_not_match_unrelated_key() {
        use ssh_key::{Algorithm, PrivateKey, rand_core::OsRng};
        let random_key = PrivateKey::random(&mut OsRng, Algorithm::Ed25519).unwrap();
        assert!(!is_passthrough_host(
            random_key.public_key().key_data(),
            &[]
        ));
    }

    #[test]
    fn ignores_malformed_lines() {
        assert!(parse_known_hosts_line("").is_none());
        assert!(parse_known_hosts_line("# comment").is_none());
        assert!(parse_known_hosts_line("not-enough-fields").is_none());
    }
}
