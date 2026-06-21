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
            targets: ["TouchIDLibrary"]),
    ],
    targets: [
        .target(
            name: "TouchIDLibrary",
            dependencies: [],
            publicHeadersPath: "include"),
    ]
)
