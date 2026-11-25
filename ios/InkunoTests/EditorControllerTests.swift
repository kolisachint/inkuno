import UIKit
import XCTest
@testable import Inkuno

/// Drives the live editor's text view the way the keyboard does, and reads back the text and the styling.
@MainActor
final class EditorControllerTests: XCTestCase {
    private var controller: MarkdownEditorController!

    override func setUp() async throws {
        try await super.setUp()
        controller = MarkdownEditorController()
    }

    override func tearDown() async throws {
        controller = nil
        try await super.tearDown()
    }

    private var view: EditorTextView { controller.textView }

    private func place(_ location: Int) {
        view.selectedRange = NSRange(location: location, length: 0)
        controller.selectionChanged()
    }

    private func isHidden(_ index: Int) -> Bool {
        let font = view.textStorage.attribute(.font, at: index, effectiveRange: nil) as? UIFont
        return (font?.pointSize ?? 1) < 0.1
    }

    func testEnterContinuesABulletList() {
        controller.update(text: "- a")
        place(3)

        view.insertText("\n")

        XCTAssertEqual(view.textStorage.string, "- a\n- ")
    }

    func testEnterOnAnEmptyItemRemovesTheMarker() {
        controller.update(text: "- ")
        place(2)

        view.insertText("\n")

        XCTAssertEqual(view.textStorage.string, "")
    }

    func testTabIndentsTheCaretLine() {
        controller.update(text: "a")
        place(0)

        view.insertText("\t")

        XCTAssertEqual(view.textStorage.string, "    a")
    }

    func testTypingStylesTheCaretLineAndHidesItOnceLeft() {
        controller.update(text: "")
        view.insertText("# Title")
        XCTAssertFalse(isHidden(0), "the caret line shows its marker")

        view.insertText("\nbody")
        XCTAssertTrue(isHidden(0), "a line the caret left hides its marker")
        XCTAssertFalse(isHidden(8), "the body line is not a marker")
    }

    func testStylingDoesNotAddAnUndoStep() {
        controller.update(text: "# Title\nbody")
        place(10)
        place(2)

        XCTAssertFalse(view.undoManager?.canUndo ?? false)
    }

    func testUndoRestoresTheTextAndItsStyling() {
        controller.update(text: "# Title")
        place(7)
        view.insertText("!")
        XCTAssertEqual(view.textStorage.string, "# Title!")

        view.undoManager?.undo()

        XCTAssertEqual(view.textStorage.string, "# Title")
        XCTAssertFalse(isHidden(0))
    }

    func testInsertBlockAddsTheTaskMarkdown() {
        controller.update(text: "")

        controller.insertBlock(.task)

        XCTAssertEqual(view.textStorage.string, "- [ ] ")
    }

    func testInsertBlockGoesOnANewLineAfterText() {
        controller.update(text: "Note")
        place(4)

        controller.insertBlock(.code)

        XCTAssertEqual(view.textStorage.string, "Note\n```\n\n```")
        XCTAssertEqual(view.selectedRange, NSRange(location: 9, length: 0))
    }

    // MARK: Block menu

    func testCommentWritesTheDesktopSyntaxAfterTheLine() {
        controller.update(text: "Ship it\nnext")
        place(0)

        XCTAssertTrue(controller.commentLine(0, said: "Keep it short"))

        let written = CommentFormat.parse(view.textStorage.string)
        XCTAssertTrue(view.textStorage.string.hasPrefix("Ship it<!-- comment @"))
        XCTAssertTrue(view.textStorage.string.hasSuffix(" -->\nnext"))
        XCTAssertEqual(written.map(\.passage), ["Ship it"])
        XCTAssertEqual(written.map(\.text), ["Keep it short"])
    }

    func testAnEmptyCommentWritesNothing() {
        controller.update(text: "Ship it")

        XCTAssertFalse(controller.commentLine(0, said: "   "))
        XCTAssertEqual(view.textStorage.string, "Ship it")
    }

    func testDuplicateCopiesTheBlockBelowIt() {
        controller.update(text: "one\ntwo")
        place(1)

        controller.duplicateBlock()

        XCTAssertEqual(view.textStorage.string, "one\none\ntwo")
    }

    func testDeleteRemovesTheBlockAndItsBreak() {
        controller.update(text: "one\ntwo\nthree")
        place(5)

        controller.deleteBlock()

        XCTAssertEqual(view.textStorage.string, "one\nthree")
    }

    func testDeleteOfTheLastBlockTakesTheBreakBefore() {
        controller.update(text: "one\ntwo")
        place(6)

        controller.deleteBlock()

        XCTAssertEqual(view.textStorage.string, "one")
    }

    func testShowMarkdownRevealsTheCaretBlockUntilTheCaretLeaves() {
        controller.showMarkers = false
        controller.update(text: "**bold** here\nnext")
        place(2)
        XCTAssertTrue(isHidden(0), "with the setting off, the caret line hides its markers")
        controller.revealBlock()
        XCTAssertFalse(isHidden(0), "the revealed block shows its markers")

        place(16)
        XCTAssertTrue(isHidden(0), "the markers hide again once the caret leaves the block")
    }

    func testRevealedOutlineLineScrollsTheCaretThere() {
        controller.update(text: "# One\n\n## Two")

        controller.reveal(line: 2)

        XCTAssertEqual(view.selectedRange, NSRange(location: 7, length: 0))
    }

    func testLinkAtColumnIsTheWikiLinkUnderIt() {
        let line = "see [[Plan]] now"

        XCTAssertEqual(MarkdownEditorController.link(in: line, at: 6)?.kind, .wiki)
        XCTAssertEqual(MarkdownEditorController.link(in: line, at: 6)?.target, "Plan")
        XCTAssertNil(MarkdownEditorController.link(in: line, at: 0))
    }

    func testCommentPassageIsTheTextBeforeItsComment() {
        let passages = MarkdownEditorController.passages(of: "Hello world<!-- comment @Ana 2026-11-24T10:02Z: nice -->")

        XCTAssertEqual(passages.first?.range, NSRange(location: 0, length: 11))
        XCTAssertEqual(passages.first?.comment.text, "nice")
    }

    func testLinkAtColumnIsTheTagUnderIt() {
        XCTAssertEqual(MarkdownEditorController.link(in: "a #idea b", at: 3)?.kind, .tag)
        XCTAssertNil(MarkdownEditorController.link(in: "a #idea b", at: 8))
    }

    func testWrapEditWrapsTheSelection() {
        let edit = MarkdownEditorController.wrapEdit("hello world", selection: NSRange(location: 6, length: 5), marker: "**")

        XCTAssertEqual(edit, EditorEdit(range: NSRange(location: 6, length: 5), text: "**world**", selection: NSRange(location: 8, length: 5)))
    }

    func testWrapEditUnwrapsASelectionThatHoldsTheMarkers() {
        let edit = MarkdownEditorController.wrapEdit("**hi**", selection: NSRange(location: 0, length: 6), marker: "**")

        XCTAssertEqual(edit, EditorEdit(range: NSRange(location: 0, length: 6), text: "hi", selection: NSRange(location: 0, length: 2)))
    }

    func testWrapEditUnwrapsMarkersJustOutsideTheSelection() {
        let edit = MarkdownEditorController.wrapEdit("**hi**", selection: NSRange(location: 2, length: 2), marker: "**")

        XCTAssertEqual(edit, EditorEdit(range: NSRange(location: 0, length: 6), text: "hi", selection: NSRange(location: 0, length: 2)))
    }

    func testWrapEditWithEmptySelectionInsertsThePairWithTheCaretBetween() {
        let edit = MarkdownEditorController.wrapEdit("ab", selection: NSRange(location: 1, length: 0), marker: "**")

        XCTAssertEqual(edit, EditorEdit(range: NSRange(location: 1, length: 0), text: "****", selection: NSRange(location: 3, length: 0)))
    }

    func testItalicInsideBoldWrapsInsteadOfUnwrapping() {
        // `**hi**` is bold: the single `*` around `hi` is part of the bold run, so Italic adds its own pair.
        let edit = MarkdownEditorController.wrapEdit("**hi**", selection: NSRange(location: 2, length: 2), marker: "*")

        XCTAssertEqual(edit, EditorEdit(range: NSRange(location: 2, length: 2), text: "*hi*", selection: NSRange(location: 3, length: 2)))
    }

    func testHeadingAddsAndRemovesTheMarkerOfTheCaretLine() {
        let added = MarkdownEditorController.headingEdit("a\nb", caret: 2)
        XCTAssertEqual(added, EditorEdit(range: NSRange(location: 2, length: 0), text: "# ", selection: NSRange(location: 4, length: 0)))

        let removed = MarkdownEditorController.headingEdit("# b", caret: 3)
        XCTAssertEqual(removed, EditorEdit(range: NSRange(location: 0, length: 2), text: "", selection: NSRange(location: 1, length: 0)))
    }

    func testBoldAndItalicWrapTheSelectionInTheEditor() {
        controller.update(text: "hello")
        view.selectedRange = NSRange(location: 0, length: 5)

        controller.apply(.bold)
        XCTAssertEqual(view.textStorage.string, "**hello**")

        view.selectedRange = NSRange(location: 2, length: 5)
        controller.toggleItalic()
        XCTAssertEqual(view.textStorage.string, "***hello***")
    }

    func testHeadingIsToggledOnTheCaretLineInTheEditor() {
        controller.update(text: "one\ntwo")
        place(5)

        controller.toggleHeading()
        XCTAssertEqual(view.textStorage.string, "one\n# two")

        controller.toggleHeading()
        XCTAssertEqual(view.textStorage.string, "one\ntwo")
    }
}

