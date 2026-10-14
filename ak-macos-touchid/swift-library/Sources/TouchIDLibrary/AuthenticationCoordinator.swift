import SwiftUI
import AppKit

// A non-activating panel can become key without activating the app. Since macOS 14
// activation is cooperative, so activating our (accessory) app to steal focus from
// the frontmost app is not reliable.
private class AuthenticationPanel: NSPanel {
    override var canBecomeKey: Bool { true }
}

public class AuthenticationCoordinator: ObservableObject {
    private var currentWindow: NSWindow?
    private var currentCallback: ((Bool, String?) -> Void)?
    private var app: NSApplication { NSApplication.shared }

    func showAuthenticationSync(request: AccessRequestModel, callback: @escaping (Bool, String?) -> Void) {
        self.currentCallback = callback
        self.presentAuthenticationWindow(request: request)
    }

    private func presentAuthenticationWindow(request: AccessRequestModel) {
        let authView = AuthenticationView(
            request: request,
            onResult: { [weak self] success, error in
                self?.handleAuthenticationResult(success: success, error: error)
            },
        )

        let hostingController = NSHostingController(rootView: authView)

        let window = AuthenticationPanel(
            contentRect: NSRect(x: 0, y: 0, width: 400, height: 500),
            styleMask: [.titled, .fullSizeContentView, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )

        window.center()
        window.contentViewController = hostingController
        window.level = .popUpMenu
        window.isFloatingPanel = true
        window.hidesOnDeactivate = false
        window.becomesKeyOnlyIfNeeded = false
        // Show on the current Space, including over full-screen apps.
        window.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        window.isOpaque = false
        window.backgroundColor = .clear
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.standardWindowButton(.closeButton)?.isHidden = true
        window.standardWindowButton(.miniaturizeButton)?.isHidden = true
        window.standardWindowButton(.zoomButton)?.isHidden = true

        window.isMovableByWindowBackground = true

        // Force SwiftUI layout so we know the window's real size before centering.
        hostingController.view.layoutSubtreeIfNeeded()

        if let screen = NSScreen.main {
            let sf = screen.visibleFrame
            let wf = window.frame
            window.setFrameOrigin(NSPoint(
                x: sf.midX - wf.width / 2,
                y: sf.midY
            ))
        }

        // We are already on the main thread (dispatched from TouchIDLibrary).
        // Do all window/app setup synchronously — a nested DispatchQueue.main.async
        // would be blocked behind the outer dispatch block for the entire duration
        // of runModal(for:), so it would never fire.
        window.orderFrontRegardless()
        window.makeKey()

        currentWindow = window
        self.app.runModal(for: window)
        window.orderOut(nil)
        currentWindow = nil
    }

    private func handleAuthenticationResult(success: Bool, error: String?) {
        print("got authorization result: \(success)")
        currentCallback?(success, error)
        // LAContext fires its reply on a background thread. stopModal() uses
        // postEvent internally, which is documented thread-safe, so this is
        // safe to call here without dispatching to main.
        self.app.stopModal()
    }

    func cancelAuthentication() {
        handleAuthenticationResult(success: false, error: "Authentication cancelled")
    }
}
