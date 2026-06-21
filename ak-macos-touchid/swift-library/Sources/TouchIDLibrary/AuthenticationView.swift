import LocalAuthentication
import LocalAuthenticationEmbeddedUI
import SwiftUI

struct AuthenticationView: View {
    var reason: String;
    let onResult: (Bool, String?) -> Void
    private var authz: LocalAuthenticationView

    init(
        reason: String,
        onResult: @escaping (Bool, String?) -> Void,
    ) {
        self.reason = reason
        self.onResult = onResult
        self.authz = LocalAuthenticationView(
            context: LAContext(),
        )
    }

    var body: some View {
        VStack {
            Text(self.reason)
            authz
        }
        .padding()
        .onAppear() {
            authz.auth() { ok, err in
                if err != nil {
                    onResult(false, err?.localizedDescription)
                } else {
                    onResult(ok, nil)
                }
            }
        }
        .containerBackground(
            .thinMaterial, for: .window
        )
        .toolbarBackgroundVisibility(
            .hidden, for: .windowToolbar
        )
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
    AuthenticationView(reason: "my reason") {_,_ in
        print("foo")
    }
}
