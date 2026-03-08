import SwiftUI
import Sysd
import Bridge

class AppDelegate: NSObject, UIApplicationDelegate {
    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey : Any]? = nil
    ) -> Bool {

        Sentry.setup()

        Sysd
            .MobilebindFactsSetSerial(
                UIDevice.current.identifierForVendor?.uuidString
            )

        Sysd.MobilebindInitSystemlog()
        Sysd.MobilebindInitConfig(
            URL.applicationSupportDirectory.relativePath,
            URL.temporaryDirectory.relativePath,
        )
        Sysd.MobilebindInit()
        Sysd.MobilebindStart()
        return true
    }

    func applicationWillTerminate(_ application: UIApplication) {
        Sysd.MobilebindStop()
    }
}

@main
struct AKApp: App {

    @UIApplicationDelegateAdaptor(AppDelegate.self) var appDelegate

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
