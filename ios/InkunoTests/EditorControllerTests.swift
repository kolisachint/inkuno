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
}
