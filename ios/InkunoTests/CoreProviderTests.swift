import Foundation
import XCTest
@testable import Inkuno

/// The editor's Markdown layer on the Rust core: the adapter's mapping of the core's records and bits.
final class CoreProviderTests: XCTestCase {
    private let provider = CoreMarkdownProvider()

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

    func testUncheckedTaskAndMetaLines() {
        XCTAssertEqual(kinds("- [ ] open"), [.task(checked: false)])
        XCTAssertEqual(kinds("---\ntitle: Hi\n---\n# Hi"), [.meta, .meta, .meta, .heading])
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

    func testLayoutOffsetsAreUTF16() {
        // "é😀" is 3 units, plus the line break: the next line starts at 4.
        let layouts = provider.documentLayout("é😀\n# T")
        XCTAssertEqual(layouts.map(\.startUTF16), [0, 4])
        XCTAssertEqual(layouts.map(\.lenUTF16), [4, 3])
        XCTAssertEqual(layouts[1].headingLevel, 1)
    }

    // MARK: Inline runs

    func testBoldItalicCodeLinkWikiTagDate() {
        let line = "**bold** *it* `code` [site](https://x.y) [[Wiki]] #tag @2026-11-24"
        XCTAssertEqual(bits(of: "bold", in: line) & StyleBit.bold, StyleBit.bold)
        XCTAssertEqual(bits(of: "**", in: line) & StyleBit.marker, StyleBit.marker)
        XCTAssertEqual(bits(of: "it", in: line) & StyleBit.italic, StyleBit.italic)
        XCTAssertEqual(bits(of: "code", in: line) & StyleBit.code, StyleBit.code)
        XCTAssertEqual(bits(of: "site", in: line) & StyleBit.link, StyleBit.link)
        // The core dims the brackets and the parentheses of a link; the address itself is quiet text, not a marker.
        XCTAssertEqual(bits(of: "](", in: line) & StyleBit.marker, StyleBit.marker)
        XCTAssertEqual(bits(of: "Wiki", in: line) & StyleBit.link, StyleBit.link)
        XCTAssertEqual(bits(of: "#tag", in: line) & StyleBit.tag, StyleBit.tag)
    }

    func testStrikeIsMapped() {
        XCTAssertEqual(bits(of: "gone", in: "~~gone~~") & StyleBit.strike, StyleBit.strike)
    }

    func testRunsUseUTF16OffsetsAfterAnEmoji() {
        // "😀" is 2 units, the space 1: the backtick is unit 3 and the code text starts at unit 4.
        let line = "😀 `c`"
        let runs = provider.styleRuns(line: line)
        XCTAssertTrue(runs.contains { $0.startUTF16 == 4 && $0.lenUTF16 == 1 && $0.bits & StyleBit.code != 0 })
        XCTAssertEqual(bits(of: "c", in: line) & StyleBit.code, StyleBit.code)
    }

    func testEmphasisInsideCodeIsNotStyled() {
        let line = "`**x**`"
        XCTAssertEqual(bits(of: "x", in: line) & StyleBit.bold, 0)
    }

    func testHashInsideWordIsNotATag() {
        let line = "C#rocks"
        XCTAssertEqual(bits(of: "rocks", in: line) & StyleBit.tag, 0)
    }

    func testPlainLineHasNoRuns() {
        XCTAssertEqual(provider.styleRuns(line: "just words"), [])
    }

    // MARK: Enter

    func testEnterContinuesListsAndQuotes() {
        XCTAssertEqual(provider.onEnter(line: "- a", colUTF16: 3, inFence: false), .insert("- "))
        XCTAssertEqual(provider.onEnter(line: "  2. x", colUTF16: 6, inFence: false), .insert("  3. "))
        XCTAssertEqual(provider.onEnter(line: "9. nine", colUTF16: 7, inFence: false), .insert("10. "))
        XCTAssertEqual(provider.onEnter(line: "- [x] task", colUTF16: 10, inFence: false), .insert("- [ ] "))
        XCTAssertEqual(provider.onEnter(line: "> q", colUTF16: 3, inFence: false), .insert("> "))
    }

    func testEnterOnAnEmptyItemClearsIt() {
        XCTAssertEqual(provider.onEnter(line: "- ", colUTF16: 2, inFence: false), .clear)
        XCTAssertEqual(provider.onEnter(line: "- [ ] ", colUTF16: 6, inFence: false), .clear)
        XCTAssertEqual(provider.onEnter(line: "2. ", colUTF16: 3, inFence: false), .clear)
    }

    func testOrdinaryEnterAndCaretInsideMarker() {
        XCTAssertEqual(provider.onEnter(line: "plain", colUTF16: 5, inFence: false), .none)
        XCTAssertEqual(provider.onEnter(line: "- a", colUTF16: 1, inFence: false), .none)
        XCTAssertEqual(provider.onEnter(line: "- a", colUTF16: 3, inFence: true), .none)
    }

    func testEnterInsideAFenceKeepsOnlyTheIndent() {
        XCTAssertEqual(provider.onEnter(line: "  - not a list", colUTF16: 14, inFence: true), .insert("  "))
    }

    func testEnterCountsTheCaretInUTF16() {
        // After the marker (unit 2) Enter continues the list; before it (unit 0) it only breaks the line.
        XCTAssertEqual(provider.onEnter(line: "- 😀 é", colUTF16: 2, inFence: false), .insert("- "))
        XCTAssertEqual(provider.onEnter(line: "- 😀 é", colUTF16: 0, inFence: false), .none)
    }

    // MARK: Indent

    func testIndentAddsFourSpacesToEveryTouchedLine() {
        let text = "a\nb\nc"
        let result = provider.indent(text, rangeUTF16: NSRange(location: 2, length: 3), outdent: false)
        XCTAssertEqual(result.newText, "a\n    b\n    c")
        XCTAssertEqual(result.newRange, NSRange(location: 6, length: 7))
    }

    func testIndentOfASelectionKeepsItsLines() {
        // The start moves past the new indent of the first line; the end moves by the whole change.
        let result = provider.indent("a\nb", rangeUTF16: NSRange(location: 0, length: 3), outdent: false)
        XCTAssertEqual(result.newText, "    a\n    b")
        XCTAssertEqual(result.newRange, NSRange(location: 4, length: 7))
    }

    func testOutdentRemovesUpToFourSpacesOrATab() {
        let result = provider.indent("  x\n\ty\n    z", rangeUTF16: NSRange(location: 0, length: 12), outdent: true)
        XCTAssertEqual(result.newText, "x\ny\nz")
    }

    func testIndentWithEmojiBeforeTheSelection() {
        // The selection starts after "😀\n" (unit 3), on "b".
        let result = provider.indent("😀\nb", rangeUTF16: NSRange(location: 3, length: 1), outdent: false)
        XCTAssertEqual(result.newText, "😀\n    b")
        XCTAssertEqual(result.newRange, NSRange(location: 7, length: 1))
    }

    func testIndentOfCaretOnEmptyText() {
        let result = provider.indent("", rangeUTF16: NSRange(location: 0, length: 0), outdent: false)
        XCTAssertEqual(result.newText, "    ")
    }

    // MARK: Runs the editor reads

    func testAtDatesAreDateRuns() {
        let line = "see @2026-11-24 and @nonsense"
        let runs = provider.styleRuns(line: line)

        let date = (line as NSString).range(of: "@2026-11-24")
        XCTAssertTrue(runs.contains { $0.startUTF16 == date.location && $0.lenUTF16 == date.length && $0.bits & StyleBit.date != 0 })
        XCTAssertFalse(runs.contains { $0.bits & StyleBit.date != 0 && $0.startUTF16 > 20 }, "an unknown word is not a date")
    }

    func testTheQuietAddressIsAStyleRun() {
        let line = "[site](https://x.y)"
        XCTAssertNotEqual(bits(of: "https", in: line) & StyleBit.quiet, 0)
    }

    func testCodeRunsAreSyntaxColours() {
        let runs = provider.codeStyleRuns(line: "let s = 1; // hi", lang: "rust")

        XCTAssertTrue(runs.contains { $0.bits & StyleBit.keyword != 0 })
        XCTAssertTrue(runs.contains { $0.bits & StyleBit.number != 0 })
        XCTAssertTrue(runs.contains { $0.bits & StyleBit.comment != 0 })
        XCTAssertFalse(runs.contains { $0.bits & StyleBit.marker != 0 }, "code runs carry no markers")
    }
}
