use libc::uid_t;
use libnss::interop::Response;
use libnss::passwd::{Passwd, PasswdHooks};
use tokio::runtime::Runtime;

use crate::config::Config;
use crate::generated::create_grpc_client;
use crate::generated::nss::{Empty, GetRequest, User};
use crate::logger::log_hook;

pub struct AuthentikPasswdHooks;
impl PasswdHooks for AuthentikPasswdHooks {
    /// get_all_entries returns all passwd entries.
    fn get_all_entries() -> Response<Vec<Passwd>> {
        log_hook("passwd@get_all_entries");
        get_all_entries()
    }

    /// get_entry_by_uid returns the passwd entry for the given uid.
    fn get_entry_by_uid(uid: uid_t) -> Response<Passwd> {
        log_hook("passwd@get_entry_by_uid");
        get_entry_by_uid(uid)
    }

    /// get_entry_by_name returns the passwd entry for the given name.
    fn get_entry_by_name(name: String) -> Response<Passwd> {
        log_hook("passwd@get_entry_by_name");
        get_entry_by_name(name)
    }
}

/// get_all_entries connects to the grpc server and asks for all passwd entries.
fn get_all_entries() -> Response<Vec<Passwd>> {
    let config = Config::from_file("/etc/authentik/host.yaml").expect("Failed to load config");

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::warn!("Failed to create runtime: {}", e);
            return Response::Unavail;
        }
    };
    rt.block_on(async {
        let mut client = match create_grpc_client(config).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to create grpc client: {}", e);
                return Response::Unavail;
            }
        };
        match client.list_users(Empty {}).await {
            Ok(r) => {
                let users: Vec<Passwd> = r
                    .into_inner()
                    .users
                    .into_iter()
                    .map(user_to_passwd_entry)
                    .collect();
                Response::Success(users)
            }
            Err(e) => {
                log::warn!("failed to send GRPC request: {}", e);
                Response::Unavail
            }
        }
    })
}

/// get_entry_by_uid connects to the grpc server and asks for the passwd entry with the given uid.
fn get_entry_by_uid(uid: uid_t) -> Response<Passwd> {
    let config = Config::from_file("/etc/authentik/host.yaml").expect("Failed to load config");

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::warn!("Failed to create runtime: {}", e);
            return Response::Unavail;
        }
    };
    rt.block_on(async {
        let mut client = match create_grpc_client(config).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to create grpc client: {}", e);
                return Response::Unavail;
            }
        };
        match client
            .get_user(GetRequest {
                id: Some(uid),
                name: None,
            })
            .await
        {
            Ok(r) => Response::Success(user_to_passwd_entry(r.into_inner())),
            Err(e) => {
                log::warn!("failed to send GRPC request: {}", e);
                Response::Unavail
            }
        }
    })
}

/// get_entry_by_name connects to the grpc server and asks for the passwd entry with the given name.
fn get_entry_by_name(name: String) -> Response<Passwd> {
    let config = Config::from_file("/etc/authentik/host.yaml").expect("Failed to load config");

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::warn!("Failed to create runtime: {}", e);
            return Response::Unavail;
        }
    };
    return rt.block_on(async {
        let mut client = match create_grpc_client(config).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to create grpc client: {}", e);
                return Response::Unavail;
            }
        };

        // This is a fake call done by PAM to avoid attacks, so we need to special case it to avoid spamming
        // logs with "Not Found" messages, as this call is done quite frequently.
        if name == "pam_unix_non_existent:" {
            return Response::NotFound;
        }
        match client
            .get_user(GetRequest {
                name: Some(name.clone()),
                id: None,
            })
            .await
        {
            Ok(r) => Response::Success(user_to_passwd_entry(r.into_inner())),
            Err(e) => {
                log::info!("error when getting user by name '{}': {}", name, e.code());
                Response::Unavail
            }
        }
    })
}

fn user_to_passwd_entry(entry: User) -> Passwd {
    let e = Passwd {
        name: entry.name,
        passwd: "x".to_owned(),
        uid: entry.uid,
        gid: entry.gid,
        gecos: entry.gecos,
        dir: entry.homedir,
        shell: entry.shell,
    };
    log::trace!("user: '{}' {}:{}", e.name, e.uid, e.gid);
    return e
}
