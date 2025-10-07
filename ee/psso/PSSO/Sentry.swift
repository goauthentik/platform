import Sentry

class Sentry {
    static func setup() {
        let appVersion = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String

        SentrySDK.start { options in
            options.dsn =
                "https://c83cdbb55c9bd568ecfa275932b6de17@o4504163616882688.ingest.us.sentry.io/4509208005312512"
            options.debug = true
            options.sendDefaultPii = false
            options.releaseName = "ak-platform-ee-psso@\(appVersion ?? "dev")"
        }
    }
}
