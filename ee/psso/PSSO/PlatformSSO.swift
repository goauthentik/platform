import AppKit
import AuthenticationServices
import Bridge
import Foundation

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

    @available(macOS 26.0, *)
    func profilePictureForUser(using loginManager: ASAuthorizationProviderExtensionLoginManager)
        async -> Data
    {
        self.logger.debug("profilePictureForUser")
        guard let idToken = loginManager.ssoTokens?["id_token"] as? String,
            let claims = jwtClaims(idToken),
            let urlString = claims["picture"] as? String,
            let url = URL(string: urlString),
            url.scheme == "https" || url.scheme == "http"
        else {
            self.logger.debug("No picture claim in id_token")
            return Data()
        }
        do {
            let (data, response) = try await URLSession.shared.data(from: url)
            guard let http = response as? HTTPURLResponse,
                (200..<300).contains(http.statusCode)
            else {
                self.logger.warning("failed to download profile picture: \(response)")
                return Data()
            }
            // The system only accepts JPEG, convert anything else
            if data.starts(with: [0xFF, 0xD8, 0xFF]) {
                return data
            }
            guard let tiff = NSImage(data: data)?.tiffRepresentation,
                let jpeg = NSBitmapImageRep(data: tiff)?
                    .representation(using: .jpeg, properties: [:])
            else {
                self.logger.warning("failed to convert profile picture to JPEG")
                return Data()
            }
            return jpeg
        } catch {
            self.logger.warning("failed to download profile picture: \(error)")
            return Data()
        }
    }

    private func jwtClaims(_ token: String) -> [String: Any]? {
        let parts = token.split(separator: ".")
        guard parts.count == 3 else { return nil }
        var payload = String(parts[1])
            .replacingOccurrences(of: "-", with: "+")
            .replacingOccurrences(of: "_", with: "/")
        payload += String(repeating: "=", count: (4 - payload.count % 4) % 4)
        guard let data = Data(base64Encoded: payload) else { return nil }
        return try? JSONSerialization.jsonObject(with: data) as? [String: Any]
    }
}
