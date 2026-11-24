import XCTest
@testable import Inkuno

/// The Home tile subtitles follow the prototype (`docs/design/prototypes/v3-ios.html` `:636-644`).
final class HomeTests: XCTestCase {
    func testTodayShowsTheNoteTitleOrNotWrittenYet() {
        XCTAssertEqual(HomeView.todaySubtitle(todayTitle: "2026-11-24"), "2026-11-24")
        XCTAssertEqual(HomeView.todaySubtitle(todayTitle: nil), "Not written yet")
    }

    func testNotesShowsTheCount() {
        XCTAssertEqual(HomeView.notesSubtitle(count: 0), "0 notes")
        XCTAssertEqual(HomeView.notesSubtitle(count: 7), "7 notes")
    }
}
