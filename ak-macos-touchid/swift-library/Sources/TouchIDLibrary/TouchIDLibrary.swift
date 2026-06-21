import Foundation
import LocalAuthentication
import Dispatch

// Synchronous authentication result structure
private struct AuthResult {
    let success: Bool
    let error: String?
}

public func authenticate_with_touchid(req: AuthRequest) -> Bool {
    let reasonString = req.reason.toString()
    // Use a semaphore to make the async call synchronous
    let semaphore = DispatchSemaphore(value: 0)
    var result = AuthResult(success: false, error: nil)

    let coord = AuthenticationCoordinator()
    coord.showAuthenticationSync(reason: reasonString) { success, error in
        result = AuthResult(success: success, error: error)
        semaphore.signal()
    }

    // Wait for authentication to complete (with timeout)
    let timeout = DispatchTime.now() + .seconds(60) // 60 second timeout
    let waitResult = semaphore.wait(timeout: timeout)

    if waitResult == .timedOut {
        coord.cancelAuthentication()
        return false
    }

    return result.success
}

public func is_touchid_available() -> Bool {
    let context = LAContext()
    var error: NSError?
    return context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &error)
}
