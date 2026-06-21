import SwiftUI
import AppKit

public class AuthenticationCoordinator: ObservableObject {
    private var currentWindow: NSWindow?
    private var currentCallback: ((Bool, String?) -> Void)?
    private var app: NSApplication

    init() {
        self.app = NSApplication.shared
        self.app.setActivationPolicy(.regular)
    }

    public func showAuthenticationSync(reason: String, callback: @escaping (Bool, String?) -> Void) {
        self.currentCallback = callback
        self.presentAuthenticationWindow(reason: reason)
    }

    private func presentAuthenticationWindow(reason: String) {
        // Create the SwiftUI view with dismiss callback
        let authView = AuthenticationView(
            reason: reason,
            onResult: { [weak self] success, error in
                self?.handleAuthenticationResult(success: success, error: error)
            },
        )

        // Create hosting controller
        let hostingController = NSHostingController(rootView: authView)

        // Create window
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 400, height: 500),
            styleMask: [.titled, .closable],
            backing: .buffered,
            defer: false
        )

        window.contentViewController = hostingController
        window.center()

        // Force the window to be key and front
        window.makeKeyAndOrderFront(nil)
//        self.app!.activate(ignoringOtherApps: true)
        window.orderFrontRegardless()
//        window.standardWindowButton(.closeButton)?.isHidden = true
//        window.standardWindowButton(.miniaturizeButton)?.isHidden = true
//        window.standardWindowButton(.zoomButton)?.isHidden = true
        currentWindow = window
        self.app.run()
    }

    private func handleAuthenticationResult(success: Bool, error: String?) {
        print("got authorization result: \(success)")
        currentCallback?(success, error)
        self.app.stop(nil)
        // NSApplication.stop will only stop the application on the next UI event
        // hence we post an empty event to trigger that
        self.app.postEvent(.init(), atStart: false)
    }

    func cancelAuthentication() {
        handleAuthenticationResult(success: false, error: "Authentication cancelled")
    }
}
