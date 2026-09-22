// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "ReviewRadarMac",
    platforms: [.macOS(.v14)],
    products: [.executable(name: "ReviewRadarMac", targets: ["ReviewRadarMac"])],
    targets: [
        .executableTarget(name: "ReviewRadarMac"),
        .testTarget(name: "ReviewRadarMacTests", dependencies: ["ReviewRadarMac"], resources: [.process("Fixtures")])
    ]
)
