import AuthenticationServices

extension AuthenticationViewController: ASAuthorizationProviderExtensionAuthorizationRequestHandler
{

    public func beginAuthorization(
        with request: ASAuthorizationProviderExtensionAuthorizationRequest
    ) {
        if !(request.loginManager?.isDeviceRegistered ?? false)
            || !(request.loginManager?.isUserRegistered ?? false)
        {
            self.logger.info("SSOE: Skipping due to unregistered user or device")
            request.doNotHandle()
            return
        }
        let callerBundle = request.callerBundleIdentifier
        if let exclusions = request.extensionData["ExcludedApps"] as? [String],
            exclusions.contains(callerBundle)
        {
            self.logger.info("SSOE: Skipping for excluded bundle \(callerBundle)")
            request.doNotHandle()
            return
        }
        self.logger.debug("SSOE: URL \(request.url.absoluteString, privacy: .public)")
        if request.url.absoluteString.contains("ak-ssoe") {
            self.logger.info("SSOE: Query contains ak-ssoe")
            request.doNotHandle()
            return
        }
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
            }
            return
        }
        request.doNotHandle()
    }
}
