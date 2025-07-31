use authentik_sys::generated::grpc_request;
use authentik_sys::generated::nss::GetRequest;
use authentik_sys::generated::nss::nss_client::NssClient;
use authentik_sys::logger::log_hook;
use libnss::interop::Response;
use libnss::shadow::{Shadow, ShadowHooks};

pub struct AuthentikShadowHooks;
impl ShadowHooks for AuthentikShadowHooks {
    fn get_all_entries() -> Response<Vec<Shadow>> {
        log_hook("shadow::get_all_entries");
        get_all_entries()
    }

    fn get_entry_by_name(name: String) -> Response<Shadow> {
        log_hook("shadow::get_entry_by_name");
        get_entry_by_name(name)
    }
}

fn get_all_entries() -> Response<Vec<Shadow>> {
    match grpc_request(async |ch| {
        return Ok(NssClient::new(ch).list_users(()).await?);
    }) {
        Ok(r) => {
            let users: Vec<Shadow> = r
                .into_inner()
                .users
                .into_iter()
                .map(|user| shadow_entry(user.name))
                .collect();
            Response::Success(users)
        }
        Err(e) => {
            log::warn!("Failed to get users: {e}");
            Response::Unavail
        }
    }
}

fn get_entry_by_name(name: String) -> Response<Shadow> {
    match grpc_request(async |ch| {
        return Ok(NssClient::new(ch)
            .get_user(GetRequest {
                name: Some(name.clone()),
                id: None,
            })
            .await?);
    }) {
        Ok(r) => Response::Success(shadow_entry(r.into_inner().name)),
        Err(e) => {
            log::warn!("Failed to get user by name '{name}': {e}");
            Response::Unavail
        }
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
