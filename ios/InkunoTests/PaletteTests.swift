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
