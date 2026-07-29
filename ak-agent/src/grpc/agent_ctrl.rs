use ak_platform::dpop::{DpopKeyPair, DpopSigner};
use ak_platform::generated::{
    agent::{RequestHeader, ResponseHeader},
    agent_ctrl::{
        CurrentProfileResponse, ListProfilesResponse, PrepareDpopKeyRequest,
        PrepareDpopKeyResponse, Profile, SetupRequest, SetupResponse, agent_ctrl_server::AgentCtrl,
    },
};
use ak_platform_keyring::hardware::{HardwareKeyError, HardwareSigningKey};
use tonic::{Request, Response, Status};

use crate::config::{ConfigV1Profile, DpopKeyBackend, dpop_hardware_app_name};
use crate::grpc::AgentGRPCServer;

#[tonic::async_trait]
impl AgentCtrl for AgentGRPCServer {
    async fn list_profiles(
        &self,
        _request: Request<()>,
    ) -> Result<Response<ListProfilesResponse>, Status> {
        let mut profiles = vec![];
        for (key, c_prof) in self.agent.cfg.read().await.profiles.iter() {
            let ptm = self
                .agent
                .gtm
                .for_profile(key)
                .await
                .ok_or(Status::invalid_argument("profile not found"))?;
            let token = ptm
                .unverified()
                .await
                .map_err(|e| Status::from_error(e.into()))?;
            let claims = token.claims().map_err(|e| Status::from_error(e.into()))?;
            let o_prof = Profile {
                name: key.clone(),
                username: claims.preferred_username,
                authentik_url: c_prof.authentik_url.clone(),
                last_renewed: Some(claims.iat.into()),
                next_renew: Some(claims.exp.into()),
                dpop_bound: c_prof.dpop_enabled(),
            };
            profiles.push(o_prof);
        }
        Ok(Response::new(ListProfilesResponse {
            header: Some(ResponseHeader { successful: true }),
            profiles,
        }))
    }

    async fn setup(
        &self,
        request: Request<SetupRequest>,
    ) -> Result<Response<SetupResponse>, Status> {
        let req = request.into_inner();
        let profile_name = req
            .header
            .ok_or(Status::invalid_argument("missing header"))?
            .profile;
        {
            let mut cfg = self.agent.cfg.write().await;
            // A DPoP-bound profile already exists at this point — created by
            // `prepare_dpop_key` before the device flow started — so only its
            // tokens need filling in, preserving `dpop_key_backend` and any
            // key material. The non-DPoP case (no prior PrepareDpopKey call)
            // creates the profile fresh, as before.
            match cfg.profiles.get_mut(&profile_name) {
                Some(profile) => {
                    profile.authentik_url = req.authentik_url;
                    profile.app_slug = req.app_slug;
                    profile.client_id = req.client_id;
                    profile.set_access_token(req.access_token);
                    profile.set_refresh_token(req.refresh_token);
                }
                None => {
                    cfg.profiles.insert(
                        profile_name.clone(),
                        ConfigV1Profile::from_tokens(
                            req.authentik_url,
                            req.app_slug,
                            req.client_id,
                            req.access_token,
                            req.refresh_token,
                        ),
                    );
                }
            }
            if cfg.active_profile.is_empty() {
                cfg.active_profile = profile_name.clone();
            }
        }
        if let Err(e) = self.agent.cfg.save().await {
            tracing::warn!("failed to save config: {e:?}");
            return Err(Status::from_error(e.into()));
        }
        self.agent.gtm.wait_for_profile(&profile_name).await;
        tracing::info!(profile = profile_name, "setup new profile");
        Ok(Response::new(SetupResponse {
            header: Some(ResponseHeader { successful: true }),
        }))
    }

    async fn prepare_dpop_key(
        &self,
        request: Request<PrepareDpopKeyRequest>,
    ) -> Result<Response<PrepareDpopKeyResponse>, Status> {
        let req = request.into_inner();
        let profile_name = req
            .header
            .ok_or(Status::invalid_argument("missing header"))?
            .profile;

        let (signer, dpop_key_backend, dpop_private_key, hardware_backed) =
            match HardwareSigningKey::open_or_generate(&dpop_hardware_app_name(), &profile_name) {
                Ok(hw) => (
                    DpopSigner::Hardware(hw),
                    DpopKeyBackend::Hardware,
                    String::new(),
                    true,
                ),
                Err(HardwareKeyError::NotAvailable) => {
                    tracing::info!(
                        profile = profile_name,
                        "no hardware key storage available on this device, using a software DPoP key"
                    );
                    let kp = DpopKeyPair::generate();
                    let pem = kp
                        .to_pkcs8_pem()
                        .map_err(|e| Status::from_error(e.into()))?;
                    (
                        DpopSigner::Software(kp),
                        DpopKeyBackend::Software,
                        pem,
                        false,
                    )
                }
                Err(HardwareKeyError::Other(e)) => {
                    tracing::warn!(
                        profile = profile_name,
                        error = %e,
                        "hardware DPoP key generation failed, falling back to a software key"
                    );
                    let kp = DpopKeyPair::generate();
                    let pem = kp
                        .to_pkcs8_pem()
                        .map_err(|e| Status::from_error(e.into()))?;
                    (
                        DpopSigner::Software(kp),
                        DpopKeyBackend::Software,
                        pem,
                        false,
                    )
                }
            };
        let dpop_jkt = signer
            .thumbprint()
            .map_err(|e| Status::from_error(e.into()))?;

        {
            let mut cfg = self.agent.cfg.write().await;
            let profile = cfg.profiles.entry(profile_name.clone()).or_insert_with(|| {
                ConfigV1Profile::from_tokens(
                    req.authentik_url.clone(),
                    req.app_slug.clone(),
                    req.client_id.clone(),
                    String::new(),
                    String::new(),
                )
            });
            profile.authentik_url = req.authentik_url;
            profile.app_slug = req.app_slug;
            profile.client_id = req.client_id;
            profile.dpop_key_backend = dpop_key_backend;
            profile.set_dpop_private_key(dpop_private_key);
        }
        if let Err(e) = self.agent.cfg.save().await {
            tracing::warn!("failed to save config: {e:?}");
            return Err(Status::from_error(e.into()));
        }

        tracing::info!(
            profile = profile_name,
            hardware_backed,
            "prepared DPoP key for profile"
        );
        Ok(Response::new(PrepareDpopKeyResponse {
            header: Some(ResponseHeader { successful: true }),
            dpop_jkt,
            hardware_backed,
        }))
    }

    async fn switch_profile(
        &self,
        request: Request<RequestHeader>,
    ) -> Result<Response<ResponseHeader>, Status> {
        let new_profile = request.into_inner().profile;
        {
            let mut cfg = self.agent.cfg.write().await;
            cfg.active_profile = new_profile.clone();
        }
        if let Err(e) = self.agent.cfg.save().await {
            tracing::warn!("failed to save config: {e:?}");
            return Err(Status::from_error(e.into()));
        }
        tracing::debug!(profile = new_profile, "Switched active profile");
        Ok(Response::new(ResponseHeader { successful: true }))
    }

    async fn current_profile(
        &self,
        _request: Request<()>,
    ) -> Result<Response<CurrentProfileResponse>, Status> {
        let cfg = self.agent.cfg.read().await;
        Ok(Response::new(CurrentProfileResponse {
            header: Some(ResponseHeader { successful: true }),
            profile: cfg.active_profile.clone(),
        }))
    }
}
