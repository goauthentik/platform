import AuthenticationServices
internal import Foundation
internal import GRPCCore
internal import GRPCNIOTransportHTTP2
internal import GRPCProtobuf
import OSLog
internal import SwiftProtobuf

final class LogInterceptor: ClientInterceptor {
    let logger: Logger
    init(logger: Logger) {
        self.logger = logger
    }
    func intercept<Input, Output>(
        request: GRPCCore.StreamingClientRequest<Input>,
        context: GRPCCore.ClientContext,
        next: (GRPCCore.StreamingClientRequest<Input>, GRPCCore.ClientContext) async throws ->
            GRPCCore.StreamingClientResponse<Output>
    ) async throws -> GRPCCore.StreamingClientResponse<Output>
    where Input: Sendable, Output: Sendable {
        self.logger
            .info("GRPC Method: '\(context.descriptor, privacy: .public)'")
        let response = try await next(request, context)

        switch response.accepted {
        case .success:
            self.logger.info("Server accepted RPC for processing")
        case .failure(let error):
            self.logger.warning("Server rejected RPC with error '\(error)'")
        }

        return response
    }
}

public struct AKInteractiveAuth {
    public var URL: String
    public var DTH: String
}

public enum SocketID: String {
    case defaultSocket
    case ctrlSocket = "ctrl"
}

public class SysdBridge {

    public static let shared: SysdBridge = SysdBridge()

    var logger: Logger
    var logInterceptor: LogInterceptor

    init() {
        self.logger = Logger(
            subsystem: Bundle.main.bundleIdentifier!, category: "GRPC.sysd")
        self.logInterceptor = LogInterceptor(logger: self.logger)
    }

    func getSocketPath(id: SocketID) -> String {
        #if os(macOS)
            if id == .defaultSocket {
                return "/var/run/authentik-sysd.sock"
            } else {
                return "/var/run/authentik-sysd-\(id).sock"
            }
        #elseif os(iOS)
            return URL.temporaryDirectory.relativePath + "/\(id).sock"
        #endif
    }

    func withClient<Result: Sendable>(
        id: SocketID = .defaultSocket,
        handleClient: (GRPCClient<HTTP2ClientTransport.Posix>) async throws -> Result
    ) async throws -> Result {
        return try await withGRPCClient(
            transport: .http2NIOPosix(
                target: .unixDomainSocket(path: self.getSocketPath(id: id)),
                transportSecurity: .plaintext
            ),
            interceptors: [self.logInterceptor],
            handleClient: handleClient,
        )
    }

    public func authInteractive() async throws -> AKInteractiveAuth {
        return try await self.withClient { client in
            let res = SystemAuthInteractive.Client(wrapping: client)
            let url = try await res.interactiveAuthAsync(
                request: ClientRequest(message: Google_Protobuf_Empty())
            )
            return AKInteractiveAuth(URL: url.url, DTH: url.headerToken)
        }
    }

    public func authToken(token: String) async throws -> Bool {
        return try await self.withClient { client in
            let c = SystemAuthToken.Client(wrapping: client)
            let res = try await c.tokenAuth(
                request: ClientRequest(
                    message: TokenAuthRequest.with { request in
                        request.token = token
                    }))
            return res.successful
        }
    }

    public func platformSignedEndpointHeader(challenge: String) async throws -> String {
        return try await self.withClient { client in
            let agentPlatform = AgentPlatform.Client(wrapping: client)
            let reply = try await agentPlatform.signedEndpointHeader(
                request: ClientRequest(
                    message: PlatformEndpointRequest.with {
                        $0.header = RequestHeader.with {
                            $0.profile = "default"
                        }
                        $0.challenge = challenge
                    })
            )
            return reply.message
        }
    }

    public func interactiveAuthSupported() async throws -> Bool {
        return try await self.withClient(id: .ctrlSocket) { client in
            let c = SystemCtrl.Client(wrapping: client)
            let reply = try await c.capabilities(
                request: ClientRequest(message: Google_Protobuf_Empty())
            )
            return reply.capabilities
                .contains(CapabilitiesResponse.Capability.authInteractive)
        }
    }

    public func ping() async throws -> String {
        return try await self.withClient { client in
            let c = Ping.Client(wrapping: client)
            let reply = try await c.ping(
                request: ClientRequest(message: Google_Protobuf_Empty())
            )
            return reply.version
        }
    }

    public func domainsEnroll(name: String, authentikURL: String, token: String) async throws {
        return try await self.withClient(id: .ctrlSocket) { client in
            let c = SystemCtrl.Client(wrapping: client)
            let _ = try await c.domainEnroll(
                request: ClientRequest(
                    message: DomainEnrollRequest.with {
                        $0.authentikURL = authentikURL
                        $0.name = name
                        $0.token = token
                    })
            )
        }
    }

    public func domainsList(name: String, authentikURL: String, token: String) async throws {
        return try await self.withClient(id: .ctrlSocket) { client in
            let c = SystemCtrl.Client(wrapping: client)
            let reply = try await c.domainList(
                request: ClientRequest(message: Google_Protobuf_Empty())
            )
//            return reply.domains[0].
        }
    }

    #if os(macOS)
        public func pssoRegisterUser(
            enclaveKeyID: String,
            userSecureEnclaveKey: String,
            userAuth: String,
        ) async throws -> ASAuthorizationProviderExtensionUserLoginConfiguration {
            return try await self.withClient { client in
                let c = SystemAuthApple.Client(wrapping: client)
                let reply = try await c.registerUser(
                    request: ClientRequest(
                        message: RegisterUserRequest.with {
                            $0.enclaveKeyID = enclaveKeyID
                            $0.userSecureEnclaveKey = userSecureEnclaveKey
                            $0.userAuth = userAuth
                        }
                    ))
                return ASAuthorizationProviderExtensionUserLoginConfiguration(
                    loginUserName: reply.username
                )
            }
        }

        public func pssoRegisterDevice(
            deviceSigningKey: String,
            deviceEncryptionKey: String,
            encKeyID: String,
            signKeyID: String,
        ) async throws -> ASAuthorizationProviderExtensionLoginConfiguration {
            return try await self.withClient { client in
                let c = SystemAuthApple.Client(wrapping: client)
                let res = try await c.registerDevice(
                    request: ClientRequest(
                        message: RegisterDeviceRequest.with {
                            $0.deviceSigningKey = deviceSigningKey
                            $0.deviceEncryptionKey = deviceEncryptionKey
                            $0.encKeyID = encKeyID
                            $0.signKeyID = signKeyID
                        }
                    ))
                let cfg = ASAuthorizationProviderExtensionLoginConfiguration(
                    clientID: res.clientID,
                    issuer: res.issuer,
                    tokenEndpointURL: URL(string: res.tokenEndpoint)!,
                    jwksEndpointURL: URL(string: res.jwksEndpoint)!,
                    audience: res.audience
                )
                cfg.nonceEndpointURL = URL(string: res.nonceEndpoint)!
                cfg.customNonceRequestValues
                    .append(
                        URLQueryItem(
                            name: "x-ak-device-token",
                            value: res.deviceToken.addingPercentEncoding(
                                withAllowedCharacters: .alphanumerics)
                        )
                    )
                return cfg
            }
        }
    #endif
}
