use std::error::Error;

use crate::setup::Profile;
use url::Url;

pub const DEFAULT_APP_SLUG: &str = "authentik-cli";
pub const DEFAULT_CLIENT_ID: &str = "authentik-cli";

pub struct URLSet {
    pub authorize_url: Url,
    pub device_code_url: Url,
    pub token_url: Url,
    pub user_info: Url,
    pub jwks: Url,
}

pub fn urls_for_profile(profile: Profile) -> Result<URLSet, Box<dyn Error>> {
    let mut base = profile.authentik_url.clone();
    if !base.path().ends_with("/") {
        let new_path = base.path().to_string() + "/";
        base.set_path(&new_path);
    }
    Ok(URLSet {
        authorize_url: base.join("application/o/authorize/")?,
        device_code_url: base.join("application/o/device/")?,
        token_url: base.join("application/o/token/")?,
        user_info: base.join("application/o/userinfo/")?,
        jwks: base.join(&format!("application/o/{}/jwks/", profile.app_slug))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urls() {
        let root = Url::parse("http://localhost").unwrap();
        let urls = urls_for_profile(Profile::new(
            root,
            "my-app".to_string(),
            "client-id".to_string(),
        ))
        .unwrap();
        assert_eq!(
            urls.authorize_url.to_string(),
            "http://localhost/application/o/authorize/"
        );
        assert_eq!(
            urls.device_code_url.to_string(),
            "http://localhost/application/o/device/"
        );
        assert_eq!(
            urls.token_url.to_string(),
            "http://localhost/application/o/token/"
        );
        assert_eq!(
            urls.user_info.to_string(),
            "http://localhost/application/o/userinfo/"
        );
        assert_eq!(
            urls.jwks.to_string(),
            "http://localhost/application/o/my-app/jwks/"
        );
    }

    #[test]
    fn test_urls_subdir() {
        let root = Url::parse("http://localhost/foo/bar").unwrap();
        let urls = urls_for_profile(Profile::new(
            root,
            "my-app".to_string(),
            "client-id".to_string(),
        ))
        .unwrap();
        assert_eq!(
            urls.authorize_url.to_string(),
            "http://localhost/foo/bar/application/o/authorize/"
        );
        assert_eq!(
            urls.device_code_url.to_string(),
            "http://localhost/foo/bar/application/o/device/"
        );
        assert_eq!(
            urls.token_url.to_string(),
            "http://localhost/foo/bar/application/o/token/"
        );
        assert_eq!(
            urls.user_info.to_string(),
            "http://localhost/foo/bar/application/o/userinfo/"
        );
        assert_eq!(
            urls.jwks.to_string(),
            "http://localhost/foo/bar/application/o/my-app/jwks/"
        );
    }
}
