import SwiftUI
import UIKit
import XCTest
@testable import Inkuno

final class EditorStylerTests: XCTestCase {
    private let light = UITraitCollection(userInterfaceStyle: .light)
    private let provider = SwiftMarkdownProvider()

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

        XCTAssertNotNil(storage.attribute(.attachment, at: 0, effectiveRange: nil))
        XCTAssertTrue(isHidden(storage, 1), "the space after the dash should be hidden")
        XCTAssertNil(storage.attribute(.attachment, at: 7, effectiveRange: nil))
    }

    func testBulletDashIsPlainTextOnTheCaretLine() {
        let storage = styled("- item\n- other", caretLine: 0)

        XCTAssertNil(storage.attribute(.attachment, at: 0, effectiveRange: nil))
        XCTAssertEqual(color(storage, 0), resolved(InkColor.faint))
    }

    func testIndentationOfANestedItemStaysVisible() {
        let storage = styled("- a\n  - b", caretLine: 0)

        XCTAssertFalse(isHidden(storage, 4))
        XCTAssertFalse(isHidden(storage, 5))
        XCTAssertNotNil(storage.attribute(.attachment, at: 6, effectiveRange: nil))
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
}
