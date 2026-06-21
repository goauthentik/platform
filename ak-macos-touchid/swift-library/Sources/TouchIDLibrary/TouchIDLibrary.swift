import AppKit
import Foundation
import LocalAuthentication
import Dispatch
import SwiftUI

private struct AuthResult {
    let success: Bool
    let error: String?
}

public func authenticate_with_touchid(req: AccessRequestFfi) -> Bool {
    let model = accessRequestModel(from: req)

    let semaphore = DispatchSemaphore(value: 0)
    var result = AuthResult(success: false, error: nil)

    let coord = AuthenticationCoordinator()
    coord.showAuthenticationSync(request: model) { success, error in
        result = AuthResult(success: success, error: error)
        semaphore.signal()
    }

    let timeout = DispatchTime.now() + .seconds(60)
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

private func accessRequestModel(from req: AccessRequestFfi) -> AccessRequestModel {
    var model = AccessRequestModel()
    model.title = req.title.toString()
    model.requestingApp = req.requesting_app.toString()
    model.accentColor = Color(hex: UInt(req.accent_color))
    model.profileName = req.profile_name.toString()
    model.profileEmail = req.profile_email.toString()
    model.profileUsername = req.profile_username.toString()
    model.profileGroups = req.profile_groups.toString()
    model.reason = req.reason.toString()

    let iconPath = req.app_icon_path.toString()
    if !iconPath.isEmpty, let nsImage = NSImage(contentsOfFile: iconPath) {
        model.appIcon = Image(nsImage: nsImage)
    }

    let avatarPath = req.profile_avatar_path.toString()
    if !avatarPath.isEmpty, let nsImage = NSImage(contentsOfFile: avatarPath) {
        model.profileAvatar = Image(nsImage: nsImage)
    }

    return model
}
