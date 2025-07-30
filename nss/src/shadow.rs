use authentik_sys::config::Config;
use authentik_sys::generated::nss::GetRequest;
use authentik_sys::logger::log_hook;
use libnss::interop::Response;
use libnss::shadow::{Shadow, ShadowHooks};
use tokio::runtime::Runtime;

use crate::generated::create_grpc_client;
use crate::grpc_status_to_nss_response;

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
                let users: Vec<Shadow> = r
                    .into_inner()
                    .users
                    .into_iter()
                    .map(|user| shadow_entry(user.name))
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

fn get_entry_by_name(name: String) -> Response<Shadow> {
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
                name: Some(name.clone()),
                id: None,
            })
            .await
        {
            Ok(r) => Response::Success(shadow_entry(r.into_inner().name)),
            Err(e) => {
                log::info!("error when getting user by name '{}': {}", name, e.code());
                grpc_status_to_nss_response(e)
            }
        }
    })
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
