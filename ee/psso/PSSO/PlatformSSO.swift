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
        return [.jwtBearer]
    }

    func keyWillRotate(
        for keyType: ASAuthorizationProviderExtensionKeyType,
        newKey: SecKey,
        loginManager: ASAuthorizationProviderExtensionLoginManager,
    ) async -> Bool {
        self.logger.debug("keyWillRotate \(String(describing: keyType))")
        switch keyType {
        case .currentDeviceSigning, .currentDeviceEncryption:
            // Re-register the rotated device key so the IdP keeps a matching public key.
            // Rejecting the rotation here would leave the server with the old key and break
            // subsequent login assertions.
            let ok = await API.shared.RotateDeviceKey(
                loginManager: loginManager, keyType: keyType, newKey: newKey)
            if !ok {
                self.logger.warning(
                    "failed to re-register rotated device key; rejecting rotation")
            }
            return ok
        case .userSecureEnclaveKey:
            // Re-registering the user SE key requires fresh user auth, which isn't available in
            // this callback. Reject so the OS falls back to interactive user re-registration via
            // beginUserRegistration. See plan: user-key rotation needs a server-coordinated change.
            self.logger.warning(
                "user SE key rotation requested; rejecting to force interactive re-registration")
            return false
        default:
            self.logger.warning(
                "unhandled key rotation for \(String(describing: keyType)); rejecting")
            return false
        }
    }
}
