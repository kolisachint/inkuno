import Foundation
import XCTest
@testable import Inkuno

final class EditorProviderTests: XCTestCase {
    private let provider = SwiftMarkdownProvider()

    private func kinds(_ text: String) -> [LineKind] {
        provider.documentLayout(text).map(\.kind)
    }

    /// The bits of every run that covers `needle` inside `line`.
    private func bits(of needle: String, in line: String) -> UInt32 {
        let range = (line as NSString).range(of: needle)
        XCTAssertNotEqual(range.location, NSNotFound, "\(needle) not in \(line)")
        var result: UInt32 = 0
        for run in provider.styleRuns(line: line) where run.startUTF16 <= range.location
            && run.startUTF16 + run.lenUTF16 >= range.location + range.length {
            result |= run.bits
        }
        return result
    }

    // MARK: Lines

    func testEmptyTextHasOneEmptyLine() {
        let layouts = provider.documentLayout("")
        XCTAssertEqual(layouts, [LineLayout(startUTF16: 0, lenUTF16: 0, kind: .para)])
    }

    func testTrailingNewlineEndsWithAnEmptyLine() {
        let layouts = provider.documentLayout("a\nb\n")
        XCTAssertEqual(layouts.map(\.startUTF16), [0, 2, 4])
        XCTAssertEqual(layouts.map(\.lenUTF16), [2, 2, 0])
    }

    func testBlockKinds() {
        let text = """
        # Title
        ---
        - item
        - [x] done
        1. one
        > quote
        | a | b |
        plain
        """
        XCTAssertEqual(kinds(text), [
            .heading, .rule, .bullet, .task(checked: true), .ordered, .quote, .table, .para,
        ])
        let layouts = provider.documentLayout(text)
        XCTAssertEqual(layouts[0].headingLevel, 1)
        XCTAssertEqual(layouts[0].markerLenUTF16, 2)
        XCTAssertEqual(layouts[3].markerLenUTF16, 6)
        XCTAssertEqual(layouts[5].markerLenUTF16, 2)
    }

    func testIndentedBulletKeepsItsIndentInTheMarker() {
        let layouts = provider.documentLayout("- a\n    - b")
        XCTAssertEqual(layouts[1].markerLenUTF16, 6)
        XCTAssertEqual(layouts[1].kind, .bullet)
    }

    func testFenceLinesAndCodeAreInCodeBlock() {
        let layouts = provider.documentLayout("```swift\nlet x = 1\n```\nafter")
        XCTAssertEqual(layouts.map(\.kind), [.fence, .code, .fence, .para])
        XCTAssertEqual(layouts.map(\.inCodeBlock), [true, true, true, false])
    }

    func testHashWithoutSpaceIsNotAHeading() {
        XCTAssertEqual(kinds("#tag"), [.para])
    }

    // MARK: Inline runs

    func testBoldItalicCodeLinkWikiTagDate() {
        let line = "**bold** *it* `code` [site](https://x.y) [[Wiki]] #tag @2026-11-24"
        XCTAssertEqual(bits(of: "bold", in: line) & StyleBit.bold, StyleBit.bold)
        XCTAssertEqual(bits(of: "**", in: line) & StyleBit.marker, StyleBit.marker)
        XCTAssertEqual(bits(of: "it", in: line) & StyleBit.italic, StyleBit.italic)
        XCTAssertEqual(bits(of: "code", in: line) & StyleBit.code, StyleBit.code)
        XCTAssertEqual(bits(of: "site", in: line) & StyleBit.link, StyleBit.link)
        XCTAssertEqual(bits(of: "](https://x.y)", in: line) & StyleBit.marker, StyleBit.marker)
        XCTAssertEqual(bits(of: "Wiki", in: line) & StyleBit.link, StyleBit.link)
        XCTAssertEqual(bits(of: "#tag", in: line) & StyleBit.tag, StyleBit.tag)
        XCTAssertEqual(bits(of: "@2026-11-24", in: line) & StyleBit.date, StyleBit.date)
    }

    func testEmphasisInsideCodeIsNotStyled() {
        let line = "`**x**`"
        XCTAssertEqual(bits(of: "x", in: line) & StyleBit.bold, 0)
    }

    func testHashInsideWordIsNotATag() {
        let line = "C#rocks"
        XCTAssertEqual(bits(of: "rocks", in: line) & StyleBit.tag, 0)
    }

    // MARK: Enter

    func testEnterContinuesListsAndQuotes() {
        XCTAssertEqual(provider.onEnter(line: "- a", colUTF16: 3, inFence: false), .insert("- "))
        XCTAssertEqual(provider.onEnter(line: "  2. x", colUTF16: 6, inFence: false), .insert("  3. "))
        XCTAssertEqual(provider.onEnter(line: "- [x] task", colUTF16: 10, inFence: false), .insert("- [ ] "))
        XCTAssertEqual(provider.onEnter(line: "> q", colUTF16: 3, inFence: false), .insert("> "))
    }

    func testEnterOnAnEmptyItemClearsIt() {
        XCTAssertEqual(provider.onEnter(line: "- ", colUTF16: 2, inFence: false), .clear)
        XCTAssertEqual(provider.onEnter(line: "- [ ] ", colUTF16: 6, inFence: false), .clear)
    }

    func testOrdinaryEnterAndCaretInsideMarker() {
        XCTAssertEqual(provider.onEnter(line: "plain", colUTF16: 5, inFence: false), .none)
        XCTAssertEqual(provider.onEnter(line: "- a", colUTF16: 1, inFence: false), .none)
        XCTAssertEqual(provider.onEnter(line: "- a", colUTF16: 3, inFence: true), .none)
    }

    // MARK: Indent

    func testIndentAddsFourSpacesToEveryTouchedLine() {
        let text = "a\nb\nc"
        let result = provider.indent(text, rangeUTF16: NSRange(location: 2, length: 3), outdent: false)
        XCTAssertEqual(result.newText, "a\n    b\n    c")
        XCTAssertEqual(result.newRange, NSRange(location: 6, length: 7))
    }

    func testOutdentRemovesUpToFourSpacesOrATab() {
        let result = provider.indent("  x\n\ty\n    z", rangeUTF16: NSRange(location: 0, length: 12), outdent: true)
        XCTAssertEqual(result.newText, "x\ny\nz")
    }

    func testIndentOfCaretOnEmptyText() {
        let result = provider.indent("", rangeUTF16: NSRange(location: 0, length: 0), outdent: false)
        XCTAssertEqual(result.newText, "    ")
    }
}
