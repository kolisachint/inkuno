import Foundation
import XCTest
@testable import Inkuno

/// Palette ranking: empty query shows recent, fuzzy name match beats body contains,
/// mirroring `crates/inkuno-logic/src/rank.rs`.
final class PaletteTests: XCTestCase {
    func testEmptyQueryMatchesEverything() {
        XCTAssertNotNil(PaletteRank.fuzzy(query: "", name: "Courses"))
    }

    func testSubstringBeatsSubsequenceAndEarlierBeatsLater() {
        let courses = PaletteRank.fuzzy(query: "cou", name: "courses")!
        let lesCourses = PaletteRank.fuzzy(query: "cou", name: "les courses")!
        XCTAssertGreaterThan(courses, lesCourses)

        let sub = PaletteRank.fuzzy(query: "crs", name: "courses")
        let full = PaletteRank.fuzzy(query: "ours", name: "courses")
        XCTAssertNotNil(sub)
        XCTAssertGreaterThan(full!, sub!)
    }

    func testNoMatchIsNil() {
        XCTAssertNil(PaletteRank.fuzzy(query: "xyz", name: "courses"))
    }

    func testFoldsAccents() {
        XCTAssertEqual(PaletteRank.fold("Thème Été"), "theme ete")
        XCTAssertNotNil(PaletteRank.fuzzy(query: "theme", name: "Thème"))
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

    func testSnippetsShowTheMatchingLine() {
        XCTAssertEqual(
            PaletteSheet.snippet(in: "# Plan\n\n- [ ] **Book** the flight\n", query: "book"),
            "- [ ] **Book** the flight"
        )
        XCTAssertNil(PaletteSheet.snippet(in: "nothing here\n", query: "book"))
    }
}
