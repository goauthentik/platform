use ak_platform::generated::sys_directory::{Group as AKGroup, User};
use libnss::group::Group;
use libnss::passwd::Passwd;
use libnss::shadow::Shadow;

/// libnss marshals these strings with `CString::new(..).expect(..)`
/// (libnss 0.9.0 `interop.rs`), which panics inside an `extern "C"` NSS entry
/// point — aborting whatever called us, i.e. sshd or login — if a field contains
/// an interior NUL. Directory data is remote-controlled, so strip NULs before
/// they get that far.
fn nss_safe(s: String) -> String {
    if s.contains('\0') {
        tracing::warn!("stripping NUL byte(s) from directory field");
        s.replace('\0', "")
    } else {
        s
    }
}

pub fn user_to_passwd_entry(entry: User) -> Passwd {
    Passwd {
        name: nss_safe(entry.name),
        passwd: "x".to_owned(),
        uid: entry.uid,
        gid: entry.gid,
        gecos: nss_safe(entry.gecos),
        dir: nss_safe(entry.homedir),
        shell: nss_safe(entry.shell),
    }
}

pub fn ak_group_to_group_entry(group: AKGroup) -> Group {
    Group {
        name: nss_safe(group.name),
        passwd: nss_safe(group.passwd),
        gid: group.gid,
        members: group.members.into_iter().map(nss_safe).collect(),
    }
}

pub fn shadow_entry(name: String) -> Shadow {
    Shadow {
        name: nss_safe(name),
        passwd: "x".to_owned(),
        last_change: -1,
        change_min_days: -1,
        change_max_days: -1,
        change_warn_days: -1,
        change_inactive_days: -1,
        expire_date: -1,
        reserved: usize::MAX,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alice_user() -> User {
        User {
            name: "alice".to_owned(),
            uid: 1000,
            gid: 100,
            gecos: "Alice Smith".to_owned(),
            homedir: "/home/alice".to_owned(),
            shell: "/bin/bash".to_owned(),
        }
    }

    fn admins_group() -> AKGroup {
        AKGroup {
            name: "admins".to_owned(),
            gid: 200,
            passwd: "x".to_owned(),
            members: vec!["alice".to_owned(), "bob".to_owned()],
        }
    }

    #[test]
    fn user_to_passwd_entry_maps_all_fields() {
        let p = user_to_passwd_entry(alice_user());
        assert_eq!(p.name, "alice");
        assert_eq!(p.passwd, "x");
        assert_eq!(p.uid, 1000);
        assert_eq!(p.gid, 100);
        assert_eq!(p.gecos, "Alice Smith");
        assert_eq!(p.dir, "/home/alice");
        assert_eq!(p.shell, "/bin/bash");
    }

    #[test]
    fn user_passwd_field_is_always_x() {
        let mut u = alice_user();
        u.name = "root".to_owned();
        assert_eq!(user_to_passwd_entry(u).passwd, "x");
    }

    #[test]
    fn ak_group_to_group_entry_maps_all_fields() {
        let g = ak_group_to_group_entry(admins_group());
        assert_eq!(g.name, "admins");
        assert_eq!(g.gid, 200);
        assert_eq!(g.passwd, "x");
        assert_eq!(g.members, vec!["alice".to_owned(), "bob".to_owned()]);
    }

    #[test]
    fn shadow_entry_name_set() {
        let s = shadow_entry("alice".to_owned());
        assert_eq!(s.name, "alice");
        assert_eq!(s.passwd, "x");
    }

    /// An interior NUL would make libnss's `CString::new(..).expect(..)` panic
    /// inside an `extern "C"` hook, aborting sshd or login.
    #[test]
    fn passwd_entry_strips_nul_bytes() {
        let p = user_to_passwd_entry(User {
            name: "ali\0ce".to_owned(),
            uid: 1000,
            gid: 100,
            gecos: "Alice\0 Smith".to_owned(),
            homedir: "/home/ali\0ce".to_owned(),
            shell: "/bin/ba\0sh".to_owned(),
        });
        for field in [&p.name, &p.passwd, &p.gecos, &p.dir, &p.shell] {
            assert!(!field.contains('\0'), "{field:?} still contains a NUL");
        }
        assert_eq!(p.name, "alice");
        assert_eq!(p.shell, "/bin/bash");
    }

    #[test]
    fn group_entry_strips_nul_bytes_including_members() {
        let g = ak_group_to_group_entry(AKGroup {
            name: "admi\0ns".to_owned(),
            gid: 200,
            passwd: "x\0".to_owned(),
            members: vec!["ali\0ce".to_owned(), "bob".to_owned()],
        });
        assert_eq!(g.name, "admins");
        assert_eq!(g.passwd, "x");
        assert_eq!(g.members, vec!["alice".to_owned(), "bob".to_owned()]);
    }

    #[test]
    fn shadow_entry_strips_nul_bytes() {
        assert_eq!(shadow_entry("ali\0ce".to_owned()).name, "alice");
    }

    #[test]
    fn shadow_entry_aging_fields_disabled() {
        let s = shadow_entry("alice".to_owned());
        assert_eq!(s.last_change, -1);
        assert_eq!(s.change_min_days, -1);
        assert_eq!(s.change_max_days, -1);
        assert_eq!(s.change_warn_days, -1);
        assert_eq!(s.change_inactive_days, -1);
        assert_eq!(s.expire_date, -1);
        assert_eq!(s.reserved, usize::MAX);
    }
}
