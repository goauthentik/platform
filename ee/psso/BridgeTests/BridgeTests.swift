//
//  BridgeTests.swift
//  BridgeTests
//
//  Created by Jens Langhammer on 20.05.26.
//  Copyright © 2026 Authentik Security Inc. All rights reserved.
//

import Foundation
import Testing
@testable import Bridge

extension Tag {
    @Tag static var integration: Self
}

// MARK: - SocketID Tests

@Suite("Bridge")
struct BridgeTests {

    @Test func socketPathDefault() {
        #expect(SysdBridge.shared.getSocketPath(id: .defaultSocket) == "/var/run/authentik-sysd.sock")
        #expect(
            SysdBridge.shared.getSocketPath(id: .ctrlSocket) == 
            "/var/run/authentik-sysd-ctrl.sock"
        )
    }

}

// MARK: - SysdBridge Integration Tests (require live socket)

@Suite("SysdBridge - Integration", .tags(.integration))
struct SysdBridgeIntegrationTests {

    let bridge = SysdBridge()

    var defaultSocketAvailable: Bool {
        FileManager.default.fileExists(atPath: bridge.getSocketPath(id: .defaultSocket))
    }

    var ctrlSocketAvailable: Bool {
        FileManager.default.fileExists(atPath: bridge.getSocketPath(id: .ctrlSocket))
    }

    @Test func ping_returnsNonEmptyVersion() async throws {
        guard defaultSocketAvailable else { return }
        let version = try await bridge.ping()
        #expect(!version.isEmpty)
    }

    @Test func interactiveAuthSupported_doesNotThrow() async throws {
        guard ctrlSocketAvailable else { return }
        let _ = try await bridge.interactiveAuthSupported()
    }

    @Test func authToken_invalidTokenReturnsFalse() async throws {
        guard defaultSocketAvailable else { return }
        let result = try await bridge.authToken(token: "invalid-token-\(UUID().uuidString)")
        #expect(result == false)
    }

    @Test func authInteractive_returnsURLAndDTH() async throws {
        guard defaultSocketAvailable else { return }
        let auth = try await bridge.authInteractive()
        #expect(!auth.URL.isEmpty)
        #expect(!auth.DTH.isEmpty)
        #expect(URL(string: auth.URL) != nil)
    }

    @Test func platformSignedEndpointHeader_withLiveSocket() async throws {
        guard defaultSocketAvailable else { return }
        let header = try await bridge.platformSignedEndpointHeader(challenge: "test-challenge-\(UUID().uuidString)")
        #expect(!header.isEmpty)
    }
}
