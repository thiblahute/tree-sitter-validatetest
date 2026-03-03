import XCTest
import SwiftTreeSitter
import TreeSitterValidatetest

final class TreeSitterValidatetestTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_validatetest())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading Validatetest grammar")
    }
}
