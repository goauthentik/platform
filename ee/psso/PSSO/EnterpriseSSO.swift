import AuthenticationServices
import Generated

extension AuthenticationViewController: ASAuthorizationProviderExtensionAuthorizationRequestHandler
{
    private func shouldSkip(request: ASAuthorizationProviderExtensionAuthorizationRequest) -> Bool {
        if !(request.loginManager?.isDeviceRegistered ?? false)
            || !(request.loginManager?.isUserRegistered ?? false)
        {
            self.logger.info("SSOE: Skipping due to unregistered user or device")
            return false
        }
        let callerBundle = request.callerBundleIdentifier
        if let exclusions = request.extensionData["ExcludedApps"] as? [String],
            exclusions.contains(callerBundle)
        {
            self.logger.info("SSOE: Skipping for excluded bundle \(callerBundle)")
            return false
        }
        if request.loginManager == nil {
            self.logger.info("SSOE: No login manager, skipping")
            return false
        }
        let config = ConfigManager.shared.getConfig(
            loginManager: request.loginManager!
        )
        let base = URL(string: config.BaseURL)!
        if request.url.scheme != base.scheme
            || request.url
                .host()
                != base
                .host()
            || !request.url.path().starts(with: base.path())
        {
            self.logger.info("SSOE: Skipping due to mismatching base URL")
            return false
        }
        return true
    }

    static let ssoeURL = "/endpoint/agent/apple_ssoext"

    public func beginAuthorization(
        with request: ASAuthorizationProviderExtensionAuthorizationRequest
    ) {
        if !self.shouldSkip(request: request) {
            request.doNotHandle()
            return
        }
        // TODO: Subpath
        if request.url.path() != AuthenticationViewController.ssoeURL {
            request.doNotHandle()
            return
        }
        self.logger.debug("SSOE: URL \(request.url.absoluteString, privacy: .public)")
        Task {
            let header = try await Generated.GRPCsysd.shared.platformSignedEndpointHeader()
            let url = request.url.appending(queryItems: [URLQueryItem(name: "ak-ssoe", value: header)])
            let headers: [String: String] = [
                "Location": url.absoluteString
            ]
            if let response = HTTPURLResponse.init(
                url: request.url, statusCode: 302, httpVersion: nil, headerFields: headers)
            {
                request.complete(httpResponse: response, httpBody: nil)
            }
        }
    }

    private func injectSession(
        with request: ASAuthorizationProviderExtensionAuthorizationRequest
    ) -> Bool {
        let sessionKey = request.loginManager?.ssoTokens?["session_key"]
        if let sk = sessionKey as? String {
            self.logger.debug("SSOE: Injecting session \(sk)")
            let url = request.url.appending(queryItems: [URLQueryItem(name: "ak-ssoe", value: "1")])
            let headers: [String: String] = [
                "Location": url.absoluteString,
                "Set-Cookie": "authentik_session=\(sk); Path=/; Secure; HttpOnly",
            ]
            if let response = HTTPURLResponse.init(
                url: request.url, statusCode: 302, httpVersion: nil, headerFields: headers)
            {
                request.complete(httpResponse: response, httpBody: nil)
                return true
            }
        }
        return false
    }
}
