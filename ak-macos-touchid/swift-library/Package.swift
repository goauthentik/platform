// swift-tools-version: 6.1
import PackageDescription

let package = Package(
    name: "TouchIDLibrary",
    platforms: [
        .macOS(.v15),
    ],
    products: [
        .library(
            name: "TouchIDLibrary",
            type: .static,
            targets: ["TouchIDLibrary"]),
    ],
    targets: [
        .target(
            name: "TouchIDLibrary",
            dependencies: [],
            resources: [
                .process("Resources")
            ],
            swiftSettings: [
                .swiftLanguageMode(.v5)
            ]),
    ]
)
