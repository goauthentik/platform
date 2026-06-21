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
        window.level = .screenSaver
        window.isOpaque = false
        window.backgroundColor = .clear

        window.isMovableByWindowBackground = true
        window.makeKeyAndOrderFront(nil)
        window.orderFrontRegardless()
        self.app.activate(ignoringOtherApps: true)

        // Defer centering so SwiftUI has finished its layout pass and the window has its final size
        DispatchQueue.main.async {
            if let screen = NSScreen.main {
                let sf = screen.visibleFrame
                let wf = window.frame
                window.setFrameOrigin(NSPoint(
                    x: sf.midX - wf.width / 2,
                    y: sf.midY
                ))
            }
        }
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
