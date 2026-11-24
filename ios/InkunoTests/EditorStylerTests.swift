import SwiftUI
import UIKit
import XCTest
@testable import Inkuno

final class EditorStylerTests: XCTestCase {
    private let light = UITraitCollection(userInterfaceStyle: .light)
    private let provider = CoreMarkdownProvider()

    /// Styles every line of `text`, with the caret on `caretLine`.
    private func styled(_ text: String, caretLine: Int?, showMarkers: Bool = true) -> NSMutableAttributedString {
        let storage = NSMutableAttributedString(string: text, attributes: [.font: UIFont.systemFont(ofSize: 16)])
        let layouts = provider.documentLayout(text)
        let styler = MarkdownStyler(provider: provider, textSize: 16, showMarkersOnCaretLine: showMarkers, traits: light)
        styler.style(storage, layouts: layouts, lines: Array(layouts.indices), caretLine: caretLine)
        return storage
    }

    private func location(of needle: String, in text: String) -> Int {
        let range = (text as NSString).range(of: needle)
        XCTAssertNotEqual(range.location, NSNotFound, "\(needle) not in text")
        return range.location
    }

    private func font(_ storage: NSAttributedString, _ index: Int) -> UIFont {
        storage.attribute(.font, at: index, effectiveRange: nil) as! UIFont
    }

    private func color(_ storage: NSAttributedString, _ index: Int) -> UIColor {
        storage.attribute(.foregroundColor, at: index, effectiveRange: nil) as! UIColor
    }

    private func isHidden(_ storage: NSAttributedString, _ index: Int) -> Bool {
        font(storage, index).pointSize < 0.1 && color(storage, index) == .clear
    }

    private func resolved(_ token: Color) -> UIColor {
        UIColor(token).resolvedColor(with: light)
    }

    // MARK: Hidden and visible markers

    func testMarkersAreHiddenOffTheCaretLine() {
        let text = "# Title\n\nsome **bold** text"
        let storage = styled(text, caretLine: 1)

        XCTAssertTrue(isHidden(storage, 0), "# should be hidden")
        XCTAssertTrue(isHidden(storage, 1), "the space after # should be hidden")
        XCTAssertEqual(font(storage, 2).pointSize, InkType.headingSize(1, base: 16), accuracy: 0.01)
        let stars = location(of: "**", in: text)
        XCTAssertTrue(isHidden(storage, stars))
        XCTAssertTrue(isHidden(storage, stars + 1))
        XCTAssertEqual(font(storage, location(of: "bold", in: text)).pointSize, 16, accuracy: 0.01)
    }

    func testMarkersShowFaintOnTheCaretLine() {
        let storage = styled("# Title\n\nsome **bold**", caretLine: 0)

        XCTAssertFalse(isHidden(storage, 0))
        XCTAssertEqual(color(storage, 0), resolved(InkColor.faint))
        XCTAssertEqual(color(storage, 2), resolved(InkColor.ink))
    }

    func testTheSettingOffHidesMarkersOnTheCaretLineToo() {
        let storage = styled("# Title\n\nsome", caretLine: 0, showMarkers: false)

        XCTAssertTrue(isHidden(storage, 0))
    }

    func testOnlyTheGivenLinesAreRestyled() {
        let text = "# Title\nbody"
        let storage = styled(text, caretLine: 0)
        let layouts = provider.documentLayout(text)
        let styler = MarkdownStyler(provider: provider, textSize: 16, showMarkersOnCaretLine: true, traits: light)

        // Line 1 gets the caret; line 0 is not among the lines given, so it keeps its live look.
        styler.style(storage, layouts: layouts, lines: [1], caretLine: 1)

        XCTAssertEqual(color(storage, 0), resolved(InkColor.faint))
        XCTAssertFalse(isHidden(storage, 0))
    }

    // MARK: Lists, tasks, fences, inline spans

    func testBulletDashBecomesAGlyphOffTheCaretLine() {
        let storage = styled("- item\n- other", caretLine: 1)

        XCTAssertEqual(storage.attribute(.inkunoGlyph, at: 0, effectiveRange: nil) as? NSNumber, NSNumber(value: MarkdownGlyph.bullet.rawValue))
        XCTAssertTrue(isHidden(storage, 1), "the space after the dash should be hidden")
        XCTAssertNil(storage.attribute(.inkunoGlyph, at: 7, effectiveRange: nil))
    }

    func testBulletDashIsPlainTextOnTheCaretLine() {
        let storage = styled("- item\n- other", caretLine: 0)

        XCTAssertNil(storage.attribute(.inkunoGlyph, at: 0, effectiveRange: nil))
        XCTAssertEqual(color(storage, 0), resolved(InkColor.faint))
    }

    func testIndentationOfANestedItemStaysVisible() {
        let storage = styled("- a\n  - b", caretLine: 0)

        XCTAssertFalse(isHidden(storage, 4))
        XCTAssertFalse(isHidden(storage, 5))
        XCTAssertNotNil(storage.attribute(.inkunoGlyph, at: 6, effectiveRange: nil))
    }

    func testTaskBoxesAreGlyphsAndCheckedTextIsStruck() {
        let text = "- [ ] open\n- [x] done"
        let storage = styled(text, caretLine: nil)

        XCTAssertEqual(storage.attribute(.inkunoGlyph, at: 0, effectiveRange: nil) as? NSNumber, NSNumber(value: MarkdownGlyph.openTask.rawValue))
        XCTAssertEqual(storage.attribute(.inkunoGlyph, at: location(of: "- [x]", in: text), effectiveRange: nil) as? NSNumber, NSNumber(value: MarkdownGlyph.checkedTask.rawValue))
        XCTAssertNil(storage.attribute(.strikethroughStyle, at: location(of: "open", in: text), effectiveRange: nil))
        XCTAssertNotNil(storage.attribute(.strikethroughStyle, at: location(of: "done", in: text), effectiveRange: nil))
    }

    // MARK: Tables

    func testTableOffTheCaretHidesOuterPipesAndTheDelimiterRow() {
        let text = "| a | b |\n| - | - |\n| 1 | two |\n\nafter"
        let storage = styled(text, caretLine: 4)
        let delimiter = (text as NSString).range(of: "| - |").location

        XCTAssertTrue(isHidden(storage, 0), "the leading pipe")
        XCTAssertTrue(isHidden(storage, delimiter), "the delimiter row collapses")
        XCTAssertTrue(isHidden(storage, delimiter + 3), "the delimiter row collapses")
        XCTAssertFalse(isHidden(storage, location(of: "a", in: text)))
        XCTAssertEqual(color(storage, location(of: "after", in: text)), resolved(InkColor.ink))
    }

    func testTableColumnsArePaddedToTheirWidestCell() {
        // The first column is 4 wide ("ab"), so the narrower "1" below gets the padding: a kern on the pipe after it.
        let text = "| ab | b |\n| - | - |\n| 1 | two |"
        let storage = styled(text, caretLine: nil)
        let widest = (text as NSString).range(of: "| b").location
        let narrow = (text as NSString).range(of: "| two").location

        XCTAssertNil(storage.attribute(.kern, at: widest, effectiveRange: nil) as? CGFloat)
        XCTAssertGreaterThan(storage.attribute(.kern, at: narrow, effectiveRange: nil) as? CGFloat ?? 0, 0)
    }

    func testTheCaretOnATableShowsItsRawMarkdown() {
        let text = "| a | b |\n| - | - |\n| 1 | two |"
        let storage = styled(text, caretLine: 1)

        XCTAssertFalse(isHidden(storage, 0))
        XCTAssertFalse(isHidden(storage, (text as NSString).range(of: "| -").location))
    }

    func testTableRowsCarryTheirPlaceAndOneWidth() {
        let text = "| a | b |\n| - | - |\n| 1 | two |\n\nafter"
        let storage = styled(text, caretLine: 4)
        func mark(_ needle: String) -> TableRowMark? {
            storage.attribute(.inkTableRow, at: location(of: needle, in: text), effectiveRange: nil) as? TableRowMark
        }

        let header = mark("| a")
        let last = mark("| 1")
        XCTAssertEqual(header?.isHeader, true)
        XCTAssertEqual(header?.isLast, false)
        XCTAssertEqual(last?.isHeader, false)
        XCTAssertEqual(last?.isLast, true)
        XCTAssertNil(storage.attribute(.inkTableRow, at: location(of: "| -", in: text), effectiveRange: nil))
        XCTAssertNil(storage.attribute(.inkTableRow, at: location(of: "after", in: text), effectiveRange: nil))
        XCTAssertNotNil(header)
        XCTAssertEqual(header?.width, last?.width)
        XCTAssertGreaterThan(header?.width ?? 0, 0)
        XCTAssertNil(storage.attribute(.underlineStyle, at: location(of: "a", in: text), effectiveRange: nil),
                     "the rules are drawn by the layout, not underlined")
    }

    func testTableTextIsInsetFromItsBorder() {
        let text = "| a | b |\n| - | - |\n| 1 | two |"
        let storage = styled(text, caretLine: nil)
        let style = storage.attribute(.paragraphStyle, at: location(of: "a", in: text), effectiveRange: nil) as? NSParagraphStyle

        XCTAssertEqual(style?.firstLineHeadIndent, MarkdownStyler.tablePadding)
        XCTAssertEqual(style?.headIndent, MarkdownStyler.tablePadding)
    }

    func testTableLastCellEndsAtTheBorder() {
        let text = "| a | b |\n| - | - |\n| 1 | two |"
        let storage = styled(text, caretLine: nil)

        XCTAssertTrue(isHidden(storage, (text as NSString).range(of: "b |").location + 1), "the space before the trailing pipe")
        XCTAssertFalse(isHidden(storage, location(of: "b", in: text)))
    }

    func testTableHasNoRowMarkWhileTheCaretIsOnIt() {
        let text = "| a | b |\n| - | - |\n| 1 | two |"
        let storage = styled(text, caretLine: 1)

        XCTAssertNil(storage.attribute(.inkTableRow, at: location(of: "| a", in: text), effectiveRange: nil))
    }

    func testTableFirstCellStartsAtTheMargin() {
        let text = "| a | b |\n| - | - |\n| 1 | two |"
        let storage = styled(text, caretLine: nil)

        XCTAssertTrue(isHidden(storage, 0), "the leading pipe")
        XCTAssertTrue(isHidden(storage, 1), "the space after the leading pipe")
        XCTAssertFalse(isHidden(storage, location(of: "a", in: text)))
        let second = (text as NSString).range(of: "| 1").location
        XCTAssertTrue(isHidden(storage, second + 1), "the space after the leading pipe of the body row")
    }

    func testCheckedTaskContentIsStruck() {
        let text = "- [x] done"
        let storage = styled(text, caretLine: 1)

        let index = location(of: "done", in: text)
        XCTAssertEqual(storage.attribute(.strikethroughStyle, at: index, effectiveRange: nil) as? Int,
                       NSUnderlineStyle.single.rawValue)
    }

    func testLinkAndTagAreStyled() {
        let text = "see [site](https://x.y) #tag"
        let storage = styled(text, caretLine: 1)

        XCTAssertEqual(color(storage, location(of: "site", in: text)), resolved(InkColor.link))
        XCTAssertEqual(storage.attribute(.backgroundColor, at: location(of: "tag", in: text), effectiveRange: nil) as? UIColor,
                       resolved(InkColor.chip))
        XCTAssertTrue(isHidden(storage, location(of: "](", in: text)))
    }

    func testFenceMarkersAreHiddenAndCodeIsMonospaced() {
        let text = "```\ncode\n```"
        let storage = styled(text, caretLine: 1)

        XCTAssertTrue(isHidden(storage, 0))
        XCTAssertEqual(font(storage, location(of: "code", in: text)).pointSize, 16 * 0.82, accuracy: 0.01)
    }

    // MARK: Text and attributes stay in sync

    func testTextIsNotChangedByStyling() {
        let text = "# Title\n- [ ] task\n**bold**"
        let storage = styled(text, caretLine: 2)

        XCTAssertEqual(storage.string, text)
    }

    func testTextChangeFindsTheEditedPart() {
        let change = TextChange(from: NSString(string: "hello world"), to: NSString(string: "hello big world"))

        XCTAssertEqual(change.prefix, 6)
        XCTAssertEqual(change.oldEnd, 6)
        XCTAssertEqual(change.newEnd, 10)
        XCTAssertEqual(change.map(8), 12)
    }

    // MARK: Code lines, quiet text and dates

    func testInlineStylesAreNotAppliedInsideCodeLines() {
        let text = "```\nlet **x** = `y`\n```\nafter"
        let storage = styled(text, caretLine: 3)

        let stars = location(of: "**x", in: text)
        XCTAssertFalse(isHidden(storage, stars), "the stars of a code line are code, not markers")
        XCTAssertFalse(isHidden(storage, location(of: "`y", in: text)), "backticks in code stay")
        XCTAssertEqual(font(storage, stars).pointSize, 16 * 0.82, accuracy: 0.01, "code lines keep the code size")
        XCTAssertNil(storage.attribute(.strikethroughStyle, at: stars, effectiveRange: nil))
    }

    func testCodeLinesTakeTheSyntaxColours() {
        let text = "```rust\nlet s = \"a\";\n```\nafter"
        let storage = styled(text, caretLine: 3)

        XCTAssertEqual(color(storage, location(of: "let", in: text)), resolved(InkColor.syntaxKeyword))
        XCTAssertEqual(color(storage, location(of: "\"a\"", in: text)), resolved(InkColor.syntaxString))
    }

    func testLinkAddressIsHiddenOffTheCaretLine() {
        let text = "see [site](https://x.y)\nnext"
        let storage = styled(text, caretLine: 1)

        XCTAssertTrue(isHidden(storage, location(of: "https", in: text)), "a link address is quiet, hidden off the caret line")
        XCTAssertEqual(color(storage, location(of: "site", in: text)), resolved(InkColor.link), "the link text stays")
    }

    func testLinkAddressShowsOnTheCaretLine() {
        let text = "see [site](https://x.y)\nnext"
        let storage = styled(text, caretLine: 0)

        XCTAssertFalse(isHidden(storage, location(of: "https", in: text)))
    }

    func testFrontMatterKeepsItsQuietTextVisible() {
        let text = "---\nsource: [x](https://x.y)\n---\n# Title"
        let storage = styled(text, caretLine: 3)

        XCTAssertFalse(isHidden(storage, location(of: "https", in: text)), "front matter is read as it is")
    }

    func testAtDatesKeepTheAccent() {
        let text = "meet @2026-11-24 and @nonsense\nnext"
        let storage = styled(text, caretLine: 1)

        XCTAssertEqual(color(storage, location(of: "2026-11-24", in: text)), resolved(InkColor.accent))
        XCTAssertNotEqual(color(storage, location(of: "nonsense", in: text)), resolved(InkColor.accent))
    }
}
