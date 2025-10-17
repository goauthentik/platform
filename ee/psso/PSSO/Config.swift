import AuthenticationServices
import Foundation
import OSLog

struct Config {
    var BaseURL: String
    var ClientID: String
    var AppSlug: String
}

class ConfigManager {
    static let shared: ConfigManager = ConfigManager()

    var logger: Logger = Logger(
        subsystem: Bundle.main.bundleIdentifier!, category: "ConfigManager")

    func getConfig(loginManager: ASAuthorizationProviderExtensionLoginManager) -> Config {
        let extData = loginManager.extensionData
        return Config(
            BaseURL: extData["authentik.base"] as! String,
            ClientID: "",
            AppSlug: "",
        )
    }

}
