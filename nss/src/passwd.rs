use authentik_sys::config::Config;
use authentik_sys::generated::nss::{GetRequest, User};
use authentik_sys::logger::log_hook;
use libc::uid_t;
use libnss::interop::Response;
use libnss::passwd::{Passwd, PasswdHooks};
use tokio::runtime::Runtime;

use crate::generated::create_grpc_client;
use crate::grpc_status_to_nss_response;

pub struct AuthentikPasswdHooks;
impl PasswdHooks for AuthentikPasswdHooks {
    fn get_all_entries() -> Response<Vec<Passwd>> {
        log_hook("passwd::get_all_entries");
        get_all_entries()
    }

    fn get_entry_by_uid(uid: uid_t) -> Response<Passwd> {
        log_hook("passwd::get_entry_by_uid");
        get_entry_by_uid(uid)
    }

    fn get_entry_by_name(name: String) -> Response<Passwd> {
        log_hook("passwd::get_entry_by_name");
        get_entry_by_name(name)
    }
}

fn get_all_entries() -> Response<Vec<Passwd>> {
    let config = Config::from_default().expect("Failed to load config");

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::warn!("Failed to create runtime: {e}");
            return Response::Unavail;
        }
    };
    rt.block_on(async {
        let mut client = match create_grpc_client(config).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to create grpc client: {e}");
                return Response::Unavail;
            }
        };
        match client.list_users(()).await {
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
                log::warn!("failed to send GRPC request: {e}");
                grpc_status_to_nss_response(e)
            }
        }
    })
}

fn get_entry_by_uid(uid: uid_t) -> Response<Passwd> {
    let config = Config::from_default().expect("Failed to load config");

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::warn!("Failed to create runtime: {e}");
            return Response::Unavail;
        }
    };
    rt.block_on(async {
        let mut client = match create_grpc_client(config).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to create grpc client: {e}");
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
                log::warn!("failed to send GRPC request: {e}");
                grpc_status_to_nss_response(e)
            }
        }
    })
}

fn get_entry_by_name(name: String) -> Response<Passwd> {
    let config = Config::from_default().expect("Failed to load config");

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::warn!("Failed to create runtime: {e}");
            return Response::Unavail;
        }
    };
    rt.block_on(async {
        let mut client = match create_grpc_client(config).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to create grpc client: {e}");
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
                grpc_status_to_nss_response(e)
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
    e
}
