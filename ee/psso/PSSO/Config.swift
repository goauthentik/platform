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

    func getConfig() -> Config {
        let sharedMDMConfig = UserDefaults(suiteName: "io.goauthentik.endpoint")?
            .object(forKey: "com.apple.configuration.managed")
        self.logger.debug(
            "Config: Shared MDM \(String(describing: sharedMDMConfig), privacy: .public)")
        let mdmConfig = UserDefaults.standard.object(forKey: "com.apple.configuration.managed")
        self.logger.debug("Config: MDM \(String(describing: mdmConfig), privacy: .public)")
        return Config(
            BaseURL: "https://ak.beryju.dev",
            ClientID: "apple-platform-sso",
            AppSlug: "apple-platform-sso"
        )
    }

}
