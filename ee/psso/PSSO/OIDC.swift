import AppAuth
import AuthenticationServices

class OIDC {
    static var shared: OIDC = OIDC()

    private var request: OIDAuthorizationRequest?
    public var completion: ((OIDTokenResponse) -> Void)?

    func startAuthorization(
        viewController: AuthenticationViewController,
        loginConfig: ASAuthorizationProviderExtensionLoginConfiguration,
        loginManager: ASAuthorizationProviderExtensionLoginManager,
    ) {
        let gconfig = ConfigManager.shared.getConfig(loginManager: loginManager)
        let config = OIDServiceConfiguration(
            authorizationEndpoint: URL(string: "\(gconfig.BaseURL)/application/o/authorize/")!,
            tokenEndpoint: URL(string: "\(gconfig.BaseURL)/application/o/token/")!)
        var scopes = [
            OIDScopeOpenID,
            OIDScopeProfile,
            OIDScopeEmail,
            "offline_access",
            "goauthentik.io/api",
        ]
        if let asc = loginConfig.additionalAuthorizationScopes {
            scopes.append(asc)
        }
        self.request = OIDAuthorizationRequest(
            configuration: config,
            clientId: loginConfig.clientID,
            clientSecret: nil,
            scopes: scopes,
            redirectURL: URL(string: "io.goauthentik.platform:/oauth2redirect")!,
            responseType: OIDResponseTypeCode,
            additionalParameters: nil
        )
        let authz = self.request!.authorizationRequestURL()
        viewController.logger.debug("authz url \(authz)")
        viewController.authorizationRequest = authz
    }

    func resumeAuthorizationFlow(with url: URL) -> Bool {
        if let request = self.request {
            let parameters = OIDURLQueryComponent(url: url)!.dictionaryValue
            let response = OIDAuthorizationResponse(
                request: request,
                parameters: parameters
            )
            if response.authorizationCode != nil {
                if let tokenExchangeRequest = response.tokenExchangeRequest() {
                    OIDAuthorizationService.perform(tokenExchangeRequest) { tokenResponse, error in
                        if let tokenResponse = tokenResponse {
                            self.completion?(tokenResponse)
                        }
                    }
                }
            }
        }
        return false
    }

}
