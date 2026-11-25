import Foundation
import InkunoCore
import XCTest
@testable import Inkuno

/// Palette ranking and search, through the core (`rank.rs` `fuzzy`, `search`, `snippet_line`).
final class PaletteTests: XCTestCase {
    /// The core scores folded text, as the palette passes it.
    private func score(_ query: String, _ name: String) -> Int32? {
        fuzzyScore(query: foldText(text: query), name: foldText(text: name))
    }

    func testEmptyQueryMatchesEverything() {
        XCTAssertNotNil(score("", "Courses"))
    }

    func testSubstringBeatsSubsequenceAndEarlierBeatsLater() {
        let courses = score("cou", "courses")!
        let lesCourses = score("cou", "les courses")!
        XCTAssertGreaterThan(courses, lesCourses)

        let sub = score("crs", "courses")
        let full = score("ours", "courses")
        XCTAssertNotNil(sub)
        XCTAssertGreaterThan(full!, sub!)
    }

    func testNoMatchIsNil() {
        XCTAssertNil(score("xyz", "courses"))
    }

    func testFoldsAccents() {
        XCTAssertEqual(foldText(text: "Thème Été"), "theme ete")
        XCTAssertNotNil(score("theme", "Thème"))
    }

    func testCommandsFilter() {
        let all = PaletteCommand.all(for: URL(fileURLWithPath: "/v/Note.md"), hasPanelHost: true)
        let titles = all.map(\.title)

        XCTAssertTrue(titles.contains("New note"))
        XCTAssertTrue(titles.contains("Today"))
        XCTAssertTrue(titles.contains("Open Settings"))
        XCTAssertTrue(titles.contains("Restore from Trash"))
        XCTAssertTrue(titles.contains("Save version"))
        XCTAssertTrue(titles.contains("Show outline"))
        XCTAssertTrue(titles.contains("Show comments"))
        XCTAssertTrue(titles.contains("Show versions"))
        XCTAssertTrue(titles.contains("Show backlinks"))

        let none = PaletteCommand.all(for: nil, hasPanelHost: false)
        XCTAssertFalse(none.map(\.title).contains("Save version"))
        XCTAssertFalse(none.map(\.title).contains("Show outline"))
        XCTAssertTrue(none.map(\.title).contains("Restore from Trash"))
    }

    /// Accept/Reject all are offered only for an open note that has pending changes.
    func testAcceptRejectAllShowOnlyWithPendingChanges() {
        let note = URL(fileURLWithPath: "/v/Note.md")
        let withPending = PaletteCommand.all(for: note, hasPanelHost: true, hasPending: true).map(\.title)
        XCTAssertTrue(withPending.contains("Accept all changes"))
        XCTAssertTrue(withPending.contains("Reject all changes"))

        let withoutPending = PaletteCommand.all(for: note, hasPanelHost: true).map(\.title)
        XCTAssertFalse(withoutPending.contains("Accept all changes"))
        XCTAssertFalse(withoutPending.contains("Reject all changes"))

        // Pending changes belong to a note: with no note open there is nothing to accept.
        let noNote = PaletteCommand.all(for: nil, hasPanelHost: false, hasPending: true).map(\.title)
        XCTAssertFalse(noNote.contains("Accept all changes"))
    }

    /// Bold, Italic and Heading are offered only for an open note whose host applies them.
    func testFormattingShowsOnlyForANoteWithAHost() {
        let note = URL(fileURLWithPath: "/v/Note.md")
        let titles = PaletteCommand.all(for: note, hasPanelHost: false, hasFormatHost: true).map(\.title)
        XCTAssertTrue(titles.contains("Bold"))
        XCTAssertTrue(titles.contains("Italic"))
        XCTAssertTrue(titles.contains("Heading"))

        let noHost = PaletteCommand.all(for: note, hasPanelHost: false).map(\.title)
        XCTAssertFalse(noHost.contains("Bold"))

        let noNote = PaletteCommand.all(for: nil, hasPanelHost: false, hasFormatHost: true).map(\.title)
        XCTAssertFalse(noNote.contains("Italic"))
    }

    func testSearchListsNamesThenTextWithTheMatchingLine() {
        let index = VaultIndex(notes: [
            NoteInput(path: "courses.md", body: "- lait", mtimeUnix: 0),
            NoteInput(path: "Notes.md", body: "# Plan\n\n- [ ] **Book** the flight\n", mtimeUnix: 0),
        ])

        let byName = index.search(query: "cou", limit: 10)
        XCTAssertEqual(byName.map(\.name), ["courses"])
        XCTAssertEqual(byName.first?.kind, .byName)

        let byText = index.search(query: "book", limit: 10)
        XCTAssertEqual(byText.map(\.name), ["Notes"])
        XCTAssertEqual(byText.first?.kind, .byText)
        XCTAssertEqual(byText.first?.snippet?.text.contains("Book"), true)
        XCTAssertTrue(index.search(query: "nothing", limit: 10).isEmpty)
    }
}
