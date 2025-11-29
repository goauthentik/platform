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

final class InteractiveAuth: Sendable {
    static let targetUrl: String = "goauthentik.io://platform/finished"
    static let tokenQS: String = "ak-auth-ia-token"
    static let dthHeader: String = "X-Authentik-Platform-Auth-DTH"

    private var authState: AKInteractiveAuth?
    private let loginManager: ASAuthorizationProviderExtensionLoginManager
    private var continuation:
        CheckedContinuation<ASAuthorizationProviderExtensionRegistrationResult, Error>?

    var logger: Logger = Logger(
        subsystem: Bundle.main.bundleIdentifier!, category: "InteractiveAuth")

    init(loginManager: ASAuthorizationProviderExtensionLoginManager) {
        self.loginManager = loginManager
    }

    func resumeAuthorizationFlow(with url: URL) async -> Bool {
        let token = url.valueOf(InteractiveAuth.tokenQS)
        if let token = token {
            let authResult = await self.handleToken(token: token)
            self.continuation?.resume(returning: authResult)
            return true
        }
        self.logger.warning("failed to get token from authorization URL")
        return false
    }

    private func handleToken(token: String) async
        -> ASAuthorizationProviderExtensionRegistrationResult
    {
        self.logger.trace("got token \(String(describing: token))")
        do {
            self.logger.debug("Validating auth token")
            if try await SysdBridge.shared.authToken(token: token) {
                self.logger.debug("Successfully validated token, registering user")
                return await API.shared
                    .RegisterUser(
                        loginManger: self.loginManager
                    )
            } else {
                return .failed
            }
        } catch {
            self.logger.error("error presentRegistrationViewController \(error)")
            return .failed
        }

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
    ) async throws -> ASAuthorizationProviderExtensionRegistrationResult? {
        let authInteractive = try await SysdBridge.shared.authInteractive()
        viewController.authorizationRequest = URL(string: authInteractive.URL)
        authState = authInteractive
        return try await withCheckedThrowingContinuation { continuation in
            loginManager.presentRegistrationViewController { error in
                if let err = error {
                    continuation.resume(throwing: err)
                    return
                }
                self.continuation = continuation
            }
        }
    }

}
