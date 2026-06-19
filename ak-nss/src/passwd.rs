use ak_platform::generated::sys_directory::system_directory_client::SystemDirectoryClient;
use ak_platform::generated::sys_directory::{GetRequest, User};
use ak_platform::grpc::grpc_request;
use libc::uid_t;
use libnss::interop::Response;
use libnss::passwd::{Passwd, PasswdHooks};

pub struct AuthentikPasswdHooks;
impl PasswdHooks for AuthentikPasswdHooks {
    #[tracing::instrument]
    fn get_all_entries() -> Response<Vec<Passwd>> {
        get_all_entries()
    }

    #[tracing::instrument(fields(uid))]
    fn get_entry_by_uid(uid: uid_t) -> Response<Passwd> {
        get_entry_by_uid(uid)
    }

    #[tracing::instrument(fields(name))]
    fn get_entry_by_name(name: String) -> Response<Passwd> {
        get_entry_by_name(name)
    }
}

fn get_all_entries() -> Response<Vec<Passwd>> {
    match grpc_request(async |ch| {
        return Ok(SystemDirectoryClient::new(ch).list_users(()).await?);
    }) {
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
            tracing::warn!("error getting groups: {e:?}");
            Response::Unavail
        }
    }
}

fn get_entry_by_uid(uid: uid_t) -> Response<Passwd> {
    match grpc_request(async |ch| {
        return Ok(SystemDirectoryClient::new(ch)
            .get_user(GetRequest {
                id: Some(uid),
                name: None,
            })
            .await?);
    }) {
        Ok(r) => Response::Success(user_to_passwd_entry(r.into_inner())),
        Err(e) => {
            tracing::warn!("error when getting user by ID '{uid}': {e:?}");
            Response::Unavail
        }
    }
}

fn get_entry_by_name(name: String) -> Response<Passwd> {
    // This is a fake call done by PAM to avoid attacks, so we need to special case it to avoid spamming
    // logs with "Not Found" messages, as this call is done quite frequently.
    if name == "pam_unix_non_existent:" {
        return Response::NotFound;
    }
    match grpc_request(async |ch| {
        return Ok(SystemDirectoryClient::new(ch)
            .get_user(GetRequest {
                name: Some(name.clone()),
                id: None,
            })
            .await?);
    }) {
        Ok(r) => Response::Success(user_to_passwd_entry(r.into_inner())),
        Err(e) => {
            tracing::warn!("error when getting user by name '{name}': {e:?}");
            Response::Unavail
        }
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
