import SwiftUI
import AppKit
import LocalAuthentication

// A non-activating panel can become key without activating the app. Since macOS 14
// activation is cooperative, so activating our (accessory) app to steal focus from
// the frontmost app is not reliable.
private class AuthenticationPanel: NSPanel {
    override var canBecomeKey: Bool { true }
}

public class AuthenticationCoordinator: ObservableObject {
    private var currentWindow: NSWindow?
    private var currentCallback: ((Bool, String?) -> Void)?
    private var passwordContext: LAContext?

    func showAuthenticationSync(request: AccessRequestModel, callback: @escaping (Bool, String?) -> Void) {
        self.currentCallback = callback
        self.presentAuthenticationWindow(request: request)
    }

    private func presentAuthenticationWindow(request: AccessRequestModel) {
        let authView = AuthenticationView(
            request: request,
            // Strong capture keeps the coordinator alive while the window is up;
            // the cycle is broken in handleAuthenticationResult().
            onResult: { success, error in
                self.handleAuthenticationResult(success: success, error: error)
            },
            onUsePassword: {
                self.usePassword(reason: request.reason)
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

        // Size the window to the SwiftUI content now; otherwise it resizes after
        // being positioned and ends up off-center.
        window.setContentSize(hostingController.view.fittingSize)

        // Center horizontally, a bit above the middle (like NSWindow.center()), on the
        // screen the user is working on (the one with the mouse).
        let mouse = NSEvent.mouseLocation
        if let screen = NSScreen.screens.first(where: { $0.frame.contains(mouse) }) ?? NSScreen.main {
            let sf = screen.visibleFrame
            let wf = window.frame
            window.setFrameOrigin(NSPoint(
                x: sf.midX - wf.width / 2,
                y: sf.midY - wf.height / 2 + sf.height / 6
            ))
        }

        // No runModal: the caller already blocks on a semaphore, and a modal loop
        // inside this main-queue block would stall the main queue (LocalAuthentication's
        // embedded view and the host app both need it) until the prompt closes.
        window.orderFrontRegardless()
        window.makeKey()
        currentWindow = window
    }

    /// Hands over to the system dialog, which offers the account password (and
    /// Touch ID again). We never see the password ourselves.
    private func usePassword(reason: String) {
        DispatchQueue.main.async {
            guard self.currentCallback != nil, self.passwordContext == nil else { return }
            // Our panel sits above normal windows and would cover the system dialog.
            self.currentWindow?.orderOut(nil)
            let context = LAContext()
            self.passwordContext = context
            context.evaluatePolicy(
                .deviceOwnerAuthentication,
                localizedReason: reason.isEmpty ? " " : reason
            ) { success, error in
                self.handleAuthenticationResult(success: success, error: error?.localizedDescription)
            }
        }
    }

    private func handleAuthenticationResult(success: Bool, error: String?) {
        // LAContext replies on a background thread; AppKit needs main.
        DispatchQueue.main.async {
            guard let callback = self.currentCallback else { return }
            self.currentCallback = nil
            callback(success, error)
            self.passwordContext?.invalidate()
            self.passwordContext = nil
            self.currentWindow?.orderOut(nil)
            self.currentWindow?.contentViewController = nil
            self.currentWindow = nil
        }
    }

    func cancelAuthentication() {
        handleAuthenticationResult(success: false, error: "Authentication cancelled")
    }
}
