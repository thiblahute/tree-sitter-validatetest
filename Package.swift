// swift-tools-version:5.3
import PackageDescription

let package = Package(
    name: "TreeSitterValidatetest",
    products: [
        .library(name: "TreeSitterValidatetest", targets: ["TreeSitterValidatetest"]),
    ],
    dependencies: [
        .package(url: "https://github.com/ChimeHQ/SwiftTreeSitter", from: "0.8.0"),
    ],
    targets: [
        .target(
            name: "TreeSitterValidatetest",
            dependencies: [],
            path: ".",
            sources: [
                "src/parser.c",
                // NOTE: if your language has an external scanner, add it here.
            ],
            resources: [
                .copy("queries")
            ],
            publicHeadersPath: "bindings/swift",
            cSettings: [.headerSearchPath("src")]
        ),
        .testTarget(
            name: "TreeSitterValidatetestTests",
            dependencies: [
                "SwiftTreeSitter",
                "TreeSitterValidatetest",
            ],
            path: "bindings/swift/TreeSitterValidatetestTests"
        )
    ],
    cLanguageStandard: .c11
)
