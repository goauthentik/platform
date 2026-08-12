use ak_meta::user_agent;
use ak_platform::{
    generated::{
        agent::{ResponseHeader, Token},
        agent_auth::{
            AuthorizeRequest, AuthorizeResponse, CurrentTokenRequest, CurrentTokenResponse,
            TokenExchangeRequest, TokenExchangeResponse, WhoAmIRequest, WhoAmIResponse,
            agent_auth_server::AgentAuth, current_token_request::Type,
        },
    },
    string::PlatformString,
};
use ak_platform_authz::grpc::AuthPeer;
use ak_platform_keyring::cache::Cache;
use ak_platform_keyring::cache::CacheData;
use chrono::{DateTime, Utc};
use hex::encode as hex_encode;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt::Debug, time::Duration};
use tonic::{Request, Response, Status};
use url::form_urlencoded;

#[derive(Clone, Serialize, Deserialize)]
struct CachedExchangeToken {
    #[serde(rename = "at")]
    access_token: String,
    expires_in: i64,
    #[serde(rename = "iat")]
    created: DateTime<Utc>,
}

impl CacheData for CachedExchangeToken {
    fn expiry(&self) -> DateTime<Utc> {
        self.created + chrono::TimeDelta::seconds(self.expires_in)
    }
}

impl Debug for CachedExchangeToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedExchangeToken")
            .field("access_token", &self.access_token.len())
            .field("expires_in", &self.expires_in)
            .field("created", &self.created)
            .finish()
    }
}

#[derive(Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    expires_in: Option<i64>,
}

use crate::{config::ConfigV1Profile, grpc::AgentGRPCServer};

#[tonic::async_trait]
impl AgentAuth for AgentGRPCServer {
    async fn who_am_i(
        &self,
        request: Request<WhoAmIRequest>,
    ) -> Result<Response<WhoAmIResponse>, Status> {
        let profile = self
            .profile_for_request(request.get_ref().header.clone())
            .await?;

        request
            .auth_peer()
            .with_message(|c| {
                let cmd = c.clone().proc_info()?.parent_cmdline()?;
                Ok(PlatformString::new()
                    .with_darwin(format!("authorize access to your account info in '{cmd}'"))
                    .with_windows(format!("'{cmd}' is attempting to access your account info"))
                    .with_linux(format!("'{cmd}' is attempting to access your account info")))
            })
            .with_uid(|c| c.clone().proc_info()?.unique_process_id())
            .with_success_timeout(Duration::from_secs(0))
            .with_denied_timeout(Duration::from_secs(0))
            .finish()
            .await?;

        let req = match profile
            .clone()
            .http_client()
            .request(
                Method::GET,
                format!("{}/application/o/userinfo/", profile.clone().authentik_url),
            )
            .bearer_auth(profile.access_token())
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => return Err(Status::from_error(e.into())),
        };
        if !req.status().is_success() {
            return Err(Status::internal("Invalid status code for whoami request"));
        }

        Ok(Response::new(WhoAmIResponse {
            header: Some(ResponseHeader { successful: true }),
            body: req
                .text()
                .await
                .map_err(|e| Status::from_error(Box::from(e)))?,
        }))
    }

    async fn get_current_token(
        &self,
        request: Request<CurrentTokenRequest>,
    ) -> Result<Response<CurrentTokenResponse>, Status> {
        let inner_req = request.get_ref();
        let profile = self.profile_for_request(inner_req.header.clone()).await?;
        let token_manager = self
            .agent
            .gtm
            .for_profile(
                &inner_req
                    .header
                    .as_ref()
                    .ok_or(Status::invalid_argument("missing header"))?
                    .profile,
            )
            .await
            .ok_or(Status::invalid_argument("profile not found"))?;

        request
            .auth_peer()
            .with_message(|c| {
                let cmd = c.clone().proc_info()?.parent_cmdline()?;
                Ok(PlatformString::new()
                    .with_darwin(format!("authorize access to your account in '{cmd}'"))
                    .with_windows(format!("'{cmd}' is attempting to access your account"))
                    .with_linux(format!("'{cmd}' is attempting to access your account")))
            })
            .with_uid(move |c| c.clone().proc_info()?.unique_process_id())
            .with_success_timeout(Duration::from_secs(0))
            .with_denied_timeout(Duration::from_secs(0))
            .finish()
            .await?;

        let token = match inner_req.r#type() {
            Type::Unspecified => Err(Status::invalid_argument("unsupported token type")),
            Type::Unverified => Ok(token_manager
                .unverified()
                .await
                .map_err(|e| Status::from_error(e.into()))?),
            Type::Verified => Ok(token_manager
                .token()
                .await
                .map_err(|e| Status::from_error(e.into()))?),
        }?;
        let c = token.claims().map_err(|e| Status::from_error(e.into()))?;

        Ok(Response::new(CurrentTokenResponse {
            header: Some(ResponseHeader { successful: true }),
            token: Some(Token {
                preferred_username: c.preferred_username,
                iss: c.iss,
                sub: c.sub.unwrap_or_default(),
                aud: c.aud,
                exp: Some(c.exp.into()),
                nbf: None,
                iat: Some(c.iat.into()),
                jti: c.jti.unwrap_or_default(),
            }),
            raw: token.access_token,
            url: profile.authentik_url.to_string(),
        }))
    }

    async fn cached_token_exchange(
        &self,
        request: Request<TokenExchangeRequest>,
    ) -> Result<Response<TokenExchangeResponse>, Status> {
        let inner = request.get_ref();
        let profile_name = inner
            .header
            .as_ref()
            .ok_or(Status::invalid_argument("missing header"))?
            .profile
            .clone();
        let profile = self.profile_for_request(inner.header).await?;
        let client_id = inner.audience.clone();

        let cid1 = audience.clone();
        let cid2 = audience.clone();
        request
            .auth_peer()
            .with_message(move |c| {
                let cmd = c.clone().proc_info()?.parent_cmdline()?;
                Ok(PlatformString::new()
                    .with_darwin(format!(
                        "authorize access to your account '{cid1}' in '{cmd}'"
                    ))
                    .with_windows(format!(
                        "'{cid1}' is attempting to access your account in '{cmd}'"
                    ))
                    .with_linux(format!(
                        "'{cid1}' is attempting to access your account in '{cmd}'"
                    )))
            })
            .with_uid(move |c| {
                let pid = c.clone().proc_info()?.unique_process_id()?;
                Ok(format!("{cid2}:{pid}"))
            })
            .with_success_timeout(Duration::from_secs(30 * 60))
            .with_denied_timeout(Duration::from_secs(1))
            .finish()
            .await?;

        let mut cache_key = vec!["token-cache".to_string(), audience.clone()];
        if !inner.scopes.is_empty() {
            cache_key.extend(inner.scopes.clone());
        }
        if let Some(at) = inner.actor_token.clone() {
            let mut hasher = Sha256::new();
            hasher.update(at.as_bytes());
            cache_key.push(hex_encode(hasher.finalize())[..8].to_owned())
        }

        let cache = Cache::<CachedExchangeToken>::new(profile_name.clone(), cache_key);
        if let Ok(cached) = cache.get().await {
            tracing::debug!(audience, "cached_token_exchange: returning cached token");
            return Ok(Response::new(TokenExchangeResponse {
                header: Some(ResponseHeader { successful: true }),
                access_token: cached.access_token,
                expires_in: cached.expires_in,
            }));
        }

        let scope_string = if inner.scopes.is_empty() {
            "openid email profile".to_string()
        } else {
            inner.scopes.join(" ")
        };

        let token_url = format!("{}/application/o/token/", profile.authentik_url);
        let body = self._token_exchange_request(inner.clone(), &profile)?;

        let res = reqwest::Client::new()
            .post(&token_url)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .header(reqwest::header::USER_AGENT, user_agent())
            .body(body)
            .send()
            .await
            .map_err(|e| Status::from_error(e.into()))?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(Status::internal(format!("token exchange failed: {body}")));
        }

        let new_token: OAuthTokenResponse =
            res.json().await.map_err(|e| Status::from_error(e.into()))?;
        let expires_in = new_token.expires_in.unwrap_or(0);

        let cached = CachedExchangeToken {
            access_token: new_token.access_token.clone(),
            expires_in,
            created: Utc::now(),
        };
        let cache = Cache::<CachedExchangeToken>::new(
            profile_name,
            vec!["token-cache".to_string(), audience.clone()],
        );
        if let Err(e) = cache.set(cached).await {
            tracing::warn!("cached_token_exchange: failed to write cache: {e:?}");
        }

        tracing::debug!(audience, "cached_token_exchange: exchanged new token");
        Ok(Response::new(TokenExchangeResponse {
            header: Some(ResponseHeader { successful: true }),
            access_token: new_token.access_token,
            expires_in,
        }))
    }

    async fn authorize(
        &self,
        request: Request<AuthorizeRequest>,
    ) -> Result<Response<AuthorizeResponse>, Status> {
        let inner = request.get_ref();
        let service = inner.service.clone();
        let uid = inner.uid.clone();

        request
            .auth_peer()
            .with_message(move |_c| {
                Ok(PlatformString::new().with_darwin(format!("authorize access to '{}'", service)))
            })
            .with_uid(move |_c| Ok(uid.clone()))
            .with_success_timeout(Duration::from_hours(2))
            .with_denied_timeout(Duration::from_mins(5))
            .finish()
            .await?;

        Ok(Response::new(AuthorizeResponse {
            header: Some(ResponseHeader { successful: true }),
        }))
    }
}

impl AgentGRPCServer {
    pub fn _token_exchange_request(
        &self,
        request: TokenExchangeRequest,
        profile: &ConfigV1Profile,
    ) -> Result<String, Status> {
        let scope_string = if request.scopes.is_empty() {
            "openid email profile".to_string()
        } else {
            request.scopes.join(" ")
        };
        let mut body = form_urlencoded::Serializer::new(String::new());
        body.append_pair("scope", &scope_string);

        // Since token-exchange (especially with actor & targeting) was only added in 2026.8
        // fallback to client_credentials if we don't need targeting
        if let Some(at) = request.actor_token {
            body.append_pair(
                "grant_type",
                "urn:ietf:params:oauth:grant-type:token-exchange",
            )
            .append_pair("client_id", &profile.client_id)
            .append_pair("subject_token", &profile.access_token())
            .append_pair(
                "subject_token_type",
                "urn:ietf:params:oauth:token-type:access_token",
            )
            .append_pair("audience", &request.audience)
            .append_pair("actor_token", &at);
            let Some(at_type) = request.actor_token_type else {
                return Err(Status::invalid_argument("Missing actor_token_type"));
            };
            body.append_pair("actor_token_type", &at_type);
        } else {
            body.append_pair("grant_type", "client_credentials")
                .append_pair("client_id", &request.audience)
                .append_pair(
                    "client_assertion_type",
                    "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
                )
                .append_pair("client_assertion", &profile.access_token());
        }
        Ok(body.finish())
    }
}
