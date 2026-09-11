import AuthenticationServices
import Bridge

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
        let registration = await API.shared.RegisterDevice(
            loginManager: loginManager,
        )
        if let registration = registration {
            registration.accountDisplayName = "authentik"
            registration.includePreviousRefreshTokenInLoginRequest = true
            do {
                try loginManager.saveLoginConfiguration(registration)
                return .success
            } catch {
                self.logger.warning("failed to save login configuration: \(error)")
                return .failed
            }
        }
        return .failed
    }

    func beginUserRegistration(
        loginManager: ASAuthorizationProviderExtensionLoginManager,
        userName: String?,
        method: ASAuthorizationProviderExtensionAuthenticationMethod,
        options: ASAuthorizationProviderExtensionRequestOptions = [],
    ) async -> ASAuthorizationProviderExtensionRegistrationResult {
        self.logger.debug(
            "beginUserRegistration \(userName ?? ""), method \(String(describing: method)), options \(String(describing: options))"
        )
        do {
            let supported = try await SysdBridge.shared.interactiveAuthSupported()
            if !supported {
                self.logger.warning("Interactive authentication not supported")
                return .failedNoRetry
            }
        } catch {
            self.logger.error("Failed to check if interactive auth is available: \(error)")
            return .failed
        }
        let interactive = InteractiveAuth(loginManager: loginManager, method: method)
        self.interactive = interactive
        do {
            return try await interactive.startAuth(viewController: self) ?? .failed
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
        // Both methods the profile can ask for. macOS picks one according to the
        // payload's AuthenticationMethod: jwt-bearer carries an assertion signed by the
        // user's Secure Enclave key, password carries the login-window credential as
        // claims of the login request. Advertising only jwt-bearer leaves a Password
        // profile with no grant it can use, and macOS never attempts a login at all.
        return [.jwtBearer, .password]
    }

    func keyWillRotate(
        for keyType: ASAuthorizationProviderExtensionKeyType,
        newKey _: SecKey,
        loginManager _: ASAuthorizationProviderExtensionLoginManager,
    ) async -> Bool {
        self.logger.debug("keyWillRotate \(String(describing: keyType))")
        return false
    }
}
