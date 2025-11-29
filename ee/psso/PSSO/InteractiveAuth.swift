import AuthenticationServices
import Bridge
import OSLog
import WebKit

extension URL {
    func valueOf(_ queryParameterName: String) -> String? {
        guard let url = URLComponents(string: self.absoluteString) else { return nil }
        return url.queryItems?.first(where: { $0.name == queryParameterName })?.value
    }
}

class InteractiveAuth {
    static let targetUrl: String = "goauthentik.io://platform/finished"
    static let tokenQS: String = "ak-auth-ia-token"
    static let dthHeader: String = "X-Authentik-Platform-Auth-DTH"

    static var shared: InteractiveAuth = InteractiveAuth()

    public var completion: ((String) async -> ASAuthorizationProviderExtensionRegistrationResult)?
    private var authState: AKInteractiveAuth?
    private var authResult: ASAuthorizationProviderExtensionRegistrationResult?

    var logger: Logger = Logger(
        subsystem: Bundle.main.bundleIdentifier!, category: "InteractiveAuth")

    func resumeAuthorizationFlow(with url: URL) async -> Bool {
        let token = url.valueOf(InteractiveAuth.tokenQS)
        if let token = token, let completion = completion {
            authResult = await completion(token)
            return true
        }
        self.logger.warning("failed to get token from authorization URL")
        return false
    }

    func injectDTH(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction) async
        -> WKNavigationActionPolicy
    {
        var request = await navigationAction.request
        if request
            .value(forHTTPHeaderField: InteractiveAuth.dthHeader) != nil
        {
            return .allow
        }
        request
            .setValue(
                authState?.DTH,
                forHTTPHeaderField: InteractiveAuth.dthHeader
            )
        await webView.load(request)
        return .cancel
    }

    @MainActor
    func startAuth(
        viewController: AuthenticationViewController,
        loginManager: ASAuthorizationProviderExtensionLoginManager
    ) async throws -> ASAuthorizationProviderExtensionRegistrationResult? {
        let authInteractive = try await SysdBridge.shared.authInteractive()
        viewController.authorizationRequest = URL(string: authInteractive.URL)
        authState = authInteractive
        return try await withCheckedThrowingContinuation { continuation in
            loginManager.presentRegistrationViewController { error in
                if let err = error {
                    continuation.resume(throwing: err)
                }
                continuation.resume(returning: .success)
            }
        }
    }

}
