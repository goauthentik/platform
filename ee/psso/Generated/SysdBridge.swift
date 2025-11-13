//
//  GRPC.swift
//  PSSO
//
//  Created by Jens Langhammer on 17.10.25.
//  Copyright © 2025 Authentik Security Inc. All rights reserved.
//
import Foundation
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
        self.logger.info("GRPC Method: '\(context.descriptor)'")
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

public class SysdBridge {

    public static let shared: SysdBridge = SysdBridge()

    var logger: Logger
    private var _oauthConfig: OAuthConfig?
    var logInterceptor: LogInterceptor

    public var oauthConfig: OAuthConfig {
        return _oauthConfig!
    }

    init() {
        self.logger = Logger(
            subsystem: Bundle.main.bundleIdentifier!, category: "GRPC.sysd")
        self.logInterceptor = LogInterceptor(logger: self.logger)
        do {
            self._oauthConfig = try sysdOAuthConfig()
        } catch {
            self.logger.error("failed to get OAuth Config: \(error)")
        }
    }

    func withClient<Result: Sendable>(
        handleClient: (GRPCClient<HTTP2ClientTransport.Posix>) async throws -> Result
    ) async throws -> Result {
        return try await withGRPCClient(
            transport: .http2NIOPosix(
                target: .unixDomainSocket(path: "/var/run/authentik-sysd.sock"),
                transportSecurity: .plaintext
            ),
            interceptors: [self.logInterceptor],
            handleClient: handleClient,
        )
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

    private func sysdOAuthConfig() throws -> OAuthConfig {
        let semaphore = DispatchSemaphore(value: 0)
        var value: OAuthConfig? = nil
        Task {
            defer { semaphore.signal() }
            value = try await sysdOAuthConfigAsync()
        }

        semaphore.wait()
        return value!
    }

    private func sysdOAuthConfigAsync() async throws -> OAuthConfig {
        return try await self.withClient { client in
            let agentPlatform = SystemAuthToken.Client(wrapping: client)
            let reply = try await agentPlatform.oAuthParams(
                request: ClientRequest(message: Google_Protobuf_Empty())
            )
            return OAuthConfig(
                BaseURL: reply.url,
                ClientID: reply.clientID,
                AppSlug: reply.appSlug
            )
        }
    }

}

public struct OAuthConfig {
    public var BaseURL: String
    public var ClientID: String
    public var AppSlug: String
}
