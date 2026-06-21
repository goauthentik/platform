import LocalAuthentication
import LocalAuthenticationEmbeddedUI
import SwiftUI

struct AuthenticationView: View {
    var request: AccessRequestModel
    let onResult: (Bool, String?) -> Void
    private var authz: LocalAuthenticationView

    init(
        request: AccessRequestModel,
        onResult: @escaping (Bool, String?) -> Void,
    ) {
        self.request = request
        self.onResult = onResult
        self.authz = LocalAuthenticationView(
            context: LAContext(),
        )
    }

    var body: some View {
        AuthentikAccessRequestView(request: request, authz: self.authz) {
            onResult(false, nil)
        }
        .onAppear() {
            authz.auth() { ok, err in
                if err != nil {
                    onResult(false, err?.localizedDescription)
                } else {
                    onResult(ok, nil)
                }
            }
        }
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

    func auth(reply: @escaping (Bool, (any Error)?) -> Void) {
        return self.context
            .evaluatePolicy(
                .deviceOwnerAuthenticationWithBiometricsOrCompanion,
                localizedReason: "my reason",
                reply: reply,
            )
    }

    func makeNSView(context: Context) -> LAAuthenticationView {
        LAAuthenticationView(context: context.coordinator.context)
    }

    func updateNSView(_ nsView: LAAuthenticationView, context: Context) {

    }

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
        )
    ) { _, _ in
        print("foo")
    }
}
