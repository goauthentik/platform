import AuthenticationServices
import Generated

extension AuthenticationViewController: ASAuthorizationProviderExtensionRegistrationHandler {

    var supportedDeviceEncryptionAlgorithms: [ASAuthorizationProviderExtensionEncryptionAlgorithm] {
        return [.ecdhe_A256GCM]
    }

    var supportedUserSecureEnclaveKeySigningAlgorithms:
        [ASAuthorizationProviderExtensionSigningAlgorithm]
    {
        return [.ed25519]
    }

    var supportedDeviceSigningAlgorithms: [ASAuthorizationProviderExtensionSigningAlgorithm] {
        return [.ed25519]
    }

    func beginDeviceRegistration(
        loginManager: ASAuthorizationProviderExtensionLoginManager,
        options: ASAuthorizationProviderExtensionRequestOptions = [],
        completion:
            @escaping (
                ASAuthorizationProviderExtensionRegistrationResult
            ) -> Void
    ) {
        self.logger.debug("Begin Device Registration")
        let config = SysdBridge.shared.oauthConfig
        self.logger.debug("Options: \(String(describing: config))")
        let loginConfig = ASAuthorizationProviderExtensionLoginConfiguration(
            clientID: config.ClientID,
            issuer: "\(config.BaseURL)/application/o/\(config.AppSlug)/",
            tokenEndpointURL: URL(string: "\(config.BaseURL)/endpoint/apple/sso/token/")!,
            jwksEndpointURL: URL(
                string: "\(config.BaseURL)/application/o/\(config.AppSlug)/jwks/")!,
            audience: config.ClientID
        )

        loginConfig.nonceEndpointURL = URL(
            string: "\(config.BaseURL)/endpoint/apple/sso/nonce/")!
        loginConfig.accountDisplayName = "authentik"
        loginConfig.includePreviousRefreshTokenInLoginRequest = true

        self.logger.debug("clientID: \(loginConfig.clientID)")
        self.logger.debug("issuer: \(loginConfig.issuer)")
        self.logger.debug("audience: \(loginConfig.audience)")

        self.cancelFunc = {
            completion(.failed)
        }

        API.shared.RegisterDevice(
            loginConfig: loginConfig,
            loginManager: loginManager,
            token: loginManager.registrationToken ?? "",
        ) { status in
            completion(status)
        }
    }

    func beginUserRegistration(
        loginManager: ASAuthorizationProviderExtensionLoginManager,
        userName: String?,
        method: ASAuthorizationProviderExtensionAuthenticationMethod,
        options: ASAuthorizationProviderExtensionRequestOptions = [],
        completion: @escaping (ASAuthorizationProviderExtensionRegistrationResult) -> Void
    ) {
        self.logger.debug(
            "beginUserRegistration \(userName ?? ""), method \(String(describing: method))"
        )
        self.logger.debug("options: \(String.init(describing: options))")
        let loginConfig = ASAuthorizationProviderExtensionUserLoginConfiguration(
            loginUserName: userName ?? "")
        let config = SysdBridge.shared.oauthConfig
        self.cancelFunc = {
            completion(.failed)
        }
        OIDC.shared.startAuthorization(
            viewController: self,
            loginConfig: ASAuthorizationProviderExtensionLoginConfiguration(
                clientID: config.ClientID,
                issuer: "\(config.BaseURL)/application/o/\(config.AppSlug)/",
                tokenEndpointURL: URL(
                    string: "\(config.BaseURL)/application/o/token/"
                )!,
                jwksEndpointURL: URL(
                    string: "\(config.BaseURL)/application/o/\(config.AppSlug)/jwks/")!,
                audience: config.ClientID,
            ),
            loginManager: loginManager
        )
        loginManager.presentRegistrationViewController { error in
            if let err = error {
                self.logger.error("error presentRegistrationViewController \(err)")
                completion(.failed)
            }
            OIDC.shared.completion = { token in
                self.logger.debug("got token \(String(describing: token))")

                API.shared.RegisterUser(
                    loginConfig: loginConfig,
                    loginManger: loginManager,
                    token: token.accessToken!,
                ) { st in
                    completion(st)
                }
            }
        }
    }

    func registrationDidComplete() {
        self.logger.debug("registrationDidComplete")
    }

    func protocolVersion() -> ASAuthorizationProviderExtensionPlatformSSOProtocolVersion {
        self.logger.debug("protocolVersion")
        return .version2_0
    }

    func registrationDidCancel() {
        self.logger.debug("registrationDidCancel")
    }

    func supportedGrantTypes() -> ASAuthorizationProviderExtensionSupportedGrantTypes {
        self.logger.debug("supportedGrantTypes")
        return [.password, .jwtBearer]
    }

    func keyWillRotate(
        for keyType: ASAuthorizationProviderExtensionKeyType,
        newKey _: SecKey,
        loginManager _: ASAuthorizationProviderExtensionLoginManager,
        completion: @escaping (Bool) -> Void
    ) {
        self.logger.debug("keyWillRotate \(String(describing: keyType))")
        completion(false)
    }
}
