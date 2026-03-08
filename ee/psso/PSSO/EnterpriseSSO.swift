import AuthenticationServices
import Bridge

extension AuthenticationViewController: ASAuthorizationProviderExtensionAuthorizationRequestHandler
{

    static let ssoExtURLPath = "/endpoints/agent/browser-backchannel/"
    static let queryChallenge = "xak-agent-challenge"
    static let queryResponse = "xak-agent-response"

    private func shouldSkip(request: ASAuthorizationProviderExtensionAuthorizationRequest) -> Bool {
        // We specifically don't check for PSSO registration status as this functionality doesn't require PSSO
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
        if request.url.valueOf(AuthenticationViewController.queryResponse) != nil {
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
        guard let challenge = request.url.valueOf(AuthenticationViewController.queryChallenge)
        else {
            self.logger.debug("SSOE: no challenge")
            request.doNotHandle()
            return
        }
        Task {
            do {
                let header = try await SysdBridge.shared.platformSignedEndpointHeader(
                    challenge: challenge
                )
                let url = request.url.appending(
                    queryItems: [
                        URLQueryItem(
                            name: AuthenticationViewController.queryResponse, value: header)
                    ]
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
}
