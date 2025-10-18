import AuthenticationServices
import Generated

extension URL {
    func valueOf(_ queryParameterName: String) -> String? {
        guard let url = URLComponents(string: self.absoluteString) else { return nil }
        return url.queryItems?.first(where: { $0.name == queryParameterName })?.value
    }
}


extension AuthenticationViewController: ASAuthorizationProviderExtensionAuthorizationRequestHandler
{

    static let ssoExtURLPath = "/endpoint/agent/apple_ssoext/"
    static let queryChallenge = "challenge"
    static let queryResponse = "response"

    private func shouldSkip(request: ASAuthorizationProviderExtensionAuthorizationRequest) -> Bool {
//        if !(request.loginManager?.isDeviceRegistered ?? false)
//            || !(request.loginManager?.isUserRegistered ?? false)
//        {
//            self.logger.info("SSOE: Skipping due to unregistered user or device")
//            return true
//        }
        let callerBundle = request.callerBundleIdentifier
        if let exclusions = request.extensionData["ExcludedApps"] as? [String],
            exclusions.contains(callerBundle)
        {
            self.logger.info("SSOE: Skipping for excluded bundle \(callerBundle)")
            return true
        }
        if request.loginManager == nil {
            self.logger.info("SSOE: No login manager, skipping")
            return true
        }
        let config = ConfigManager.shared.getConfig(
            loginManager: request.loginManager!
        )
        guard let base = URL(string: config.BaseURL) else {
            self.logger.info("SSOE: Unable to parse base URL")
            return true
        }
        if request.url.scheme != base.scheme
            || request.url
                .host()
                != base
                .host()
            || !request.url.path().starts(with: base.path())
        {
            self.logger.info("SSOE: Skipping due to mismatching base URL")
            return true
        }
        if request.url.valueOf(AuthenticationViewController.queryResponse) == nil {
            self.logger.info("SSOE: Skipping due to existing response")
            return true
        }
        return false
    }

    public func beginAuthorization(
        with request: ASAuthorizationProviderExtensionAuthorizationRequest
    ) {
        self.logger.debug("SSOE:beginAuthorization URL \(request.url.absoluteString)")
        if self.shouldSkip(request: request) {
            request.doNotHandle()
            return
        }
        // TODO: Subpath
        if request.url.path() != AuthenticationViewController.ssoExtURLPath {
            request.doNotHandle()
            return
        }
        guard let challenge = request.url.valueOf(AuthenticationViewController.queryChallenge) else {
            self.logger.debug("SSOE: no challenge")
            request.doNotHandle()
            return
        }
        Task {
            do {
                let header = try await Generated.GRPCsysd.shared.platformSignedEndpointHeader(
                    challenge: challenge
                )
                let url = request.url.appending(
                    queryItems: [URLQueryItem(name: AuthenticationViewController.queryResponse, value: header)]
                )
                let headers: [String: String] = [
                    "Location": url.absoluteString
                ]
                if let response = HTTPURLResponse.init(
                    url: request.url, statusCode: 302, httpVersion: nil, headerFields: headers)
                {
                    request.complete(httpResponse: response, httpBody: nil)
                }
            } catch {
                self.logger.error("failed to register: \(error)")
                request.doNotHandle()
                return
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
