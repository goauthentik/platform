import LocalAuthentication
import LocalAuthenticationEmbeddedUI
import SwiftUI

struct AuthenticationView: View {
    var request: AccessRequestModel
    let onResult: (Bool, String?) -> Void
    let onUsePassword: () -> Void
    private var authz: LocalAuthenticationView

    init(
        request: AccessRequestModel,
        onResult: @escaping (Bool, String?) -> Void,
        onUsePassword: @escaping () -> Void,
    ) {
        self.request = request
        self.onResult = onResult
        self.onUsePassword = onUsePassword
        self.authz = LocalAuthenticationView(
            context: LAContext(),
            reason: request.reason,
        )
    }

    var body: some View {
        AuthentikAccessRequestView(
            request: request,
            authz: self.authz,
            onDeny: { onResult(false, nil) },
            onUsePassword: usePassword,
        )
        .onAppear() {
            authz.auth() { ok, err in
                switch (err as? LAError)?.code {
                case nil:
                    onResult(ok, nil)
                // Our own invalidate() in usePassword(); the password prompt reports the result.
                case .appCancel:
                    return
                // Touch ID can't be used right now (e.g. lid closed), fall back like the system dialog.
                case .biometryNotAvailable, .biometryNotEnrolled, .biometryLockout:
                    usePassword()
                default:
                    onResult(false, err?.localizedDescription)
                }
            }
        }
    }

    private func usePassword() {
        authz.context.invalidate()
        onUsePassword()
    }
}

struct LocalAuthenticationView: NSViewRepresentable {
    class Coordinator {
        let context: LAContext

        init(_ context: LAContext) {
            self.context = context
        }
    }

    var context: LAContext
    var reason: String

    func auth(reply: @escaping (Bool, (any Error)?) -> Void) {
        return self.context
            .evaluatePolicy(
                .deviceOwnerAuthenticationWithBiometricsOrCompanion,
                localizedReason: reason.isEmpty ? " " : reason,
                reply: reply,
            )
    }

    // LAAuthenticationView only comes in fixed sizes (16/32/64/128pt); scaling or
    // clipping it makes LocalAuthentication cancel the evaluation.
    static let controlSize: NSControl.ControlSize = .small
    static let size: CGFloat = 32

    func makeNSView(context: Context) -> LAAuthenticationView {
        LAAuthenticationView(context: context.coordinator.context, controlSize: Self.controlSize)
    }

    func updateNSView(_ nsView: LAAuthenticationView, context: Context) {}

    func makeCoordinator() -> Coordinator {
        Coordinator(context)
    }
}

#Preview {
    AuthenticationView(
        request: AccessRequestModel(
            title: "authentik Access Requested",
            requestingApp: "iTermServer-3.6.11",
            profileName: "Jane Doe",
            profileEmail: "jane@goauthentik.io",
            profileUsername: "jdoe",
            profileGroups: "Engineering, Admins"
        ),
        onResult: { _, _ in print("foo") },
        onUsePassword: {}
    )
}
