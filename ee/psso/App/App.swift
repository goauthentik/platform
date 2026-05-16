import SwiftUI
import Sysd
import Bridge
import BackgroundTasks
import OSLog

class AppDelegate: NSObject, UIApplicationDelegate {
    var logger: Logger

    override init() {
        self.logger = Logger(
            subsystem: Bundle.main.bundleIdentifier!, category: "App")
        super.init()
    }

    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey : Any]? = nil
    ) -> Bool {
        self.logger.debug("Setting up sentry")
        Sentry.setup()

        self.logger.debug("Setting sysd override serial")
        Sysd.MobilebindFactsSetSerial(
            UIDevice.current.identifierForVendor?.uuidString
        )

        self.logger.debug("Init sysd")
        Sysd.MobilebindInitSystemlog()
        Sysd.MobilebindInitConfig(
            URL.applicationSupportDirectory.relativePath,
            URL.temporaryDirectory.relativePath,
        )
        Sysd.MobilebindInit()

        self.logger.debug("Starting sysd")
        Sysd.MobilebindStart()

        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: "io.goauthentik.platform.app.sysd_refresh", using: nil
        ) { task in
            self.logger.debug("Task start io.goauthentik.platform.app.sysd_refresh")
            task.expirationHandler = {
                task.setTaskCompleted(success: false)
                Sysd.MobilebindCancelPeriodical()
            }

            Sysd.MobilebindHandlePeriodical()
            task.setTaskCompleted(success: true)
            self.scheduleSysdPeriodical()
        }

        NotificationCenter.default.addObserver(forName:UIApplication.didEnterBackgroundNotification, object: nil, queue: nil) { (_) in
            self.logger.debug("didEnterBackgroundNotification")
            self.scheduleSysdPeriodical()
        }

        return true
    }

    func scheduleSysdPeriodical() {
        let sysdRefreshTask = BGProcessingTaskRequest(identifier: "io.goauthentik.platform.app.sysd_refresh")
        sysdRefreshTask.earliestBeginDate = Date(timeIntervalSinceNow: 60)
        sysdRefreshTask.requiresNetworkConnectivity = true
        do {
            try BGTaskScheduler.shared.submit(sysdRefreshTask)
        } catch {
            print("Unable to submit task: \(error.localizedDescription)")
        }
    }

    func applicationWillTerminate(_ application: UIApplication) {
        self.logger.debug("applicationWillTerminate")
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
