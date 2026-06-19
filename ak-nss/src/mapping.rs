use ak_platform::generated::sys_directory::{Group as AKGroup, User};
use libnss::group::Group;
use libnss::passwd::Passwd;
use libnss::shadow::Shadow;

pub fn user_to_passwd_entry(entry: User) -> Passwd {
    Passwd {
        name: entry.name,
        passwd: "x".to_owned(),
        uid: entry.uid,
        gid: entry.gid,
        gecos: entry.gecos,
        dir: entry.homedir,
        shell: entry.shell,
    }
}

pub fn ak_group_to_group_entry(group: AKGroup) -> Group {
    Group {
        name: group.name,
        passwd: group.passwd,
        gid: group.gid,
        members: group.members,
    }
}

pub fn shadow_entry(name: String) -> Shadow {
    Shadow {
        name,
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
