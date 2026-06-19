mod group;
mod passwd;
mod shadow;

use ak_platform::generated::sys_directory::{Group as AKGroup, User};
use ak_platform::log::LevelFilter;
use ak_platform::log::unix::log_hook;
use ak_platform::log::{init_log, set_log_level};
use ak_platform::string::PlatformString;
use ctor::ctor;
use dtor::dtor;
use libnss::passwd::Passwd;
use libnss::shadow::Shadow;
use libnss::{libnss_group_hooks, libnss_passwd_hooks, libnss_shadow_hooks};

use libnss::group::Group;

pub struct AuthentikNSS {}

libnss_passwd_hooks!(authentik, AuthentikNSS);
libnss_shadow_hooks!(authentik, AuthentikNSS);
libnss_group_hooks!(authentik, AuthentikNSS);

impl AuthentikNSS {
    fn ak_group_to_group_entry(group: AKGroup) -> Group {
        Group {
            name: group.name,
            passwd: group.passwd,
            gid: group.gid,
            members: group.members,
        }
    }

    fn user_to_passwd_entry(entry: User) -> Passwd {
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

    fn shadow_entry(name: String) -> Shadow {
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
}

#[ctor(unsafe)]
fn ctor() {
    init_log(PlatformString::new_with_default("libnss-authentik"));
    // With NSS we don't have a good way to configure log level dynamically
    // we could read it from /etc/authentik
    set_log_level(LevelFilter::Warn);
    log_hook("ctor");
}

#[dtor(unsafe)]
fn dtor() {
    log_hook("dtor");
}
