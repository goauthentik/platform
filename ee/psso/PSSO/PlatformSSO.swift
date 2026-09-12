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
                if #available(macOS 27.0, *) {
                    registration.federationType = .openID
                }
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
            "beginUserRegistration '\(userName ?? "")', method \(String(describing: method)), options \(String(describing: options))"
        )
        if loginManager.isUserRegistered && !options.contains(.registrationRepair) {
            self.logger.info("User is already registered and repair is not required")
            return .success
        }
        if !options.contains(.userInteractionEnabled) {
            self.logger.error("User interaction is required")
            return .userInterfaceRequired
        }
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
        let interactive = InteractiveAuth(loginManager: loginManager)
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
        if #available(macOS 27.0, *) {
            return [.jwtBearer, .tokenExchange];
        }
        return [.jwtBearer];
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
