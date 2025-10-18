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

public class GRPCsysd {

    public static let shared: GRPCsysd = GRPCsysd()

    var logger: Logger = Logger(
        subsystem: Bundle.main.bundleIdentifier!, category: "GRPC.sysd")

    public func platformSignedEndpointHeader(challenge: String) async throws -> String {
        self.logger.debug("Connecting to GRPC Sysd")
        return try await withGRPCClient(
            transport: .http2NIOPosix(
                target: .unixDomainSocket(path: "/var/run/authentik-sysd.sock"),
                transportSecurity: .plaintext
            )
        ) { client in
            let agentPlatform = AgentPlatform.Client(wrapping: client)
            let reply = try await agentPlatform.signedEndpointHeader(
                request: ClientRequest(message: PlatformEndpointRequest.with{
                    $0.header = RequestHeader.with {
                        $0.profile = "default"
                    }
                    $0.challenge = challenge
                })
            )
            return reply.message
        }
    }

}
