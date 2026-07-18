pub mod agent;
pub mod config;
pub mod grpc;
pub mod ssh;
pub mod token;

pub use agent::Agent;


use authentik_client;

pub fn aki() {
    authentik_client::apis::core_api::core_users_list();
}
