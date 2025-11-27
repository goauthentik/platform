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
    ) async -> ASAuthorizationProviderExtensionRegistrationResult {
        self.logger.debug("Begin Device Registration")
        //        self.logger.debug("Options: \(String(describing: config))")
        let loginConfig = ASAuthorizationProviderExtensionLoginConfiguration(
            clientID: "",
            issuer: "",
            tokenEndpointURL: URL(string: "")!,
            jwksEndpointURL: URL(string: "")!,
            audience: ""
        )

        loginConfig.nonceEndpointURL = URL(string: "")!
        loginConfig.accountDisplayName = "authentik"
        loginConfig.includePreviousRefreshTokenInLoginRequest = true

        return await API.shared.RegisterDevice(
            loginConfig: loginConfig,
            loginManager: loginManager,
            token: loginManager.registrationToken ?? "",
        )
    }

    func beginUserRegistration(
        loginManager: ASAuthorizationProviderExtensionLoginManager,
        userName: String?,
        method: ASAuthorizationProviderExtensionAuthenticationMethod,
        options: ASAuthorizationProviderExtensionRequestOptions = [],
    ) async -> ASAuthorizationProviderExtensionRegistrationResult {
        self.logger.debug(
            "beginUserRegistration \(userName ?? ""), method \(String(describing: method))"
        )
        self.logger.debug("options: \(String.init(describing: options))")
        InteractiveAuth.shared.completion = { token async in
            self.logger.trace("got token \(String(describing: token))")
            do {
                switch try await SysdBridge.shared.authToken(token: token) {
                case true:
                    return .success
                case false:
                    return .failed
                }
            } catch {
                self.logger.error("error presentRegistrationViewController \(error)")
                return .failed
            }
        }
        do {
            return try await InteractiveAuth.shared
                .startAuth(
                    viewController: self,
                    loginManager: loginManager) ?? .failed
        } catch {
            self.logger.error("Error starting interactive authentication: \(error)")
            return .failed
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
