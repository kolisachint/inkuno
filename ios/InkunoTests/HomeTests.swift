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

    /// A recall card shows the note's date as day and month, as the prototype does ("21 October").
    func testRecallShowsTheDayAndMonth() {
        var parts = DateComponents()
        parts.year = 2026
        parts.month = 11
        parts.day = 25
        parts.hour = 12
        let now = Calendar.current.date(from: parts)!

        XCTAssertEqual(HomeView.recallDate(days: 35, now: now), "21 October")
        XCTAssertEqual(HomeView.recallDate(days: 0, now: now), "25 November")
    }
}
