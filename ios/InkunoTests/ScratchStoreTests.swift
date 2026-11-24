import XCTest
@testable import Inkuno

/// Scratch notes: naming by the moment, newest first, blank notes dropped, trash kept under its name.
/// Runs on a temporary folder, never on the app's Application Support.
final class ScratchStoreTests: XCTestCase {
    private var folder: URL!
    private var calendar: Calendar!

    override func setUpWithError() throws {
        try super.setUpWithError()
        folder = FileManager.default.temporaryDirectory
            .appendingPathComponent("inkuno-scratch-\(UUID().uuidString)", isDirectory: true)
        calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "UTC")!
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: folder)
        try super.tearDownWithError()
    }

    private func moment(_ hour: Int, _ minute: Int, _ second: Int = 0) -> Date {
        calendar.date(from: DateComponents(year: 2026, month: 11, day: 14, hour: hour, minute: minute, second: second))!
    }

    @discardableResult
    private func write(_ name: String, _ text: String) throws -> URL {
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let url = folder.appendingPathComponent(name).appendingPathExtension("md")
        try text.write(to: url, atomically: true, encoding: .utf8)
        return url
    }

    func testANewScratchNoteIsNamedByItsMoment() throws {
        let url = try ScratchStore.create(in: folder, now: moment(10, 32), calendar: calendar)

        XCTAssertEqual(url.lastPathComponent, "2026-11-14-103200.md")
        XCTAssertEqual(try String(contentsOf: url, encoding: .utf8), "", "a new scratch note starts empty")
    }

    func testANoteMadeInTheSameSecondGetsANumber() throws {
        let first = try ScratchStore.create(in: folder, now: moment(10, 32), calendar: calendar)
        let second = try ScratchStore.create(in: folder, now: moment(10, 32), calendar: calendar)

        XCTAssertEqual(first.lastPathComponent, "2026-11-14-103200.md")
        XCTAssertEqual(second.lastPathComponent, "2026-11-14-103200 2.md")
    }

    func testTheListIsNewestFirstWithCollisionsInOrder() throws {
        try write("2026-11-14-093000", "")
        try write("2026-11-14-103200", "a")
        try write("2026-11-14-103200 2", "b")
        try write("2026-11-13-235959", "c")
        try write("2026-11-14-103159", "d")
        try write("Projet", "not a scratch name but a file here")

        let names = ScratchStore.list(in: folder).map { $0.deletingPathExtension().lastPathComponent }

        XCTAssertEqual(names, [
            "2026-11-14-103200 2",
            "2026-11-14-103200",
            "2026-11-14-103159",
            "2026-11-14-093000",
            "2026-11-13-235959",
            "Projet",
        ], "newest first; names that are not scratch names sort last")
    }

    func testTheListSkipsHiddenAndNonNoteFiles() throws {
        try write("2026-11-14-103200", "a")
        try write(".2026-11-14-110000", "hidden")
        try FileManager.default.createDirectory(at: folder.appendingPathComponent(".trash"), withIntermediateDirectories: true)
        try "x".write(to: folder.appendingPathComponent("image.png"), atomically: true, encoding: .utf8)

        XCTAssertEqual(ScratchStore.list(in: folder).map(\.lastPathComponent), ["2026-11-14-103200.md"])
    }

    func testTheTitleIsTheFirstLineWithoutItsHeadingMark() throws {
        let url = try write("2026-11-14-103200", "\n\n  # Idée du matin\ncorps")

        XCTAssertEqual(ScratchStore.title(of: url), "Idée du matin")
    }

    func testABlankNoteIsNewNoteAsTitle() throws {
        let url = try write("2026-11-14-103200", "  \n\t\n")

        XCTAssertEqual(ScratchStore.title(of: url), "New note")
    }

    func testCleanBlankDropsOnlyTheBlankNotes() throws {
        let blank = try write("2026-11-14-103200", " \n  ")
        let kept = try write("2026-11-14-103201", "text")

        ScratchStore.cleanBlank(in: folder)

        XCTAssertFalse(FileManager.default.fileExists(atPath: blank.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: kept.path))
    }

    func testLeavingABlankNoteDropsIt() throws {
        let url = try write("2026-11-14-103200", "   \n")

        XCTAssertTrue(ScratchStore.discardIfBlank(url))
        XCTAssertFalse(FileManager.default.fileExists(atPath: url.path))
    }

    func testLeavingANoteWithTextKeepsIt() throws {
        let url = try write("2026-11-14-103200", "a thought")

        XCTAssertFalse(ScratchStore.discardIfBlank(url))
        XCTAssertEqual(try String(contentsOf: url, encoding: .utf8), "a thought")
    }

    func testTrashMovesTheNoteIntoTheBinUnderItsName() throws {
        let url = try write("2026-11-14-103200", "un")
        let first = try ScratchStore.trash(url, in: folder)
        let again = try write("2026-11-14-103200", "deux")
        let second = try ScratchStore.trash(again, in: folder)

        XCTAssertEqual(first, folder.appendingPathComponent(".trash/2026-11-14-103200.md"))
        XCTAssertEqual(second, folder.appendingPathComponent(".trash/2 2026-11-14-103200.md"))
        XCTAssertEqual(try String(contentsOf: second, encoding: .utf8), "deux")
        XCTAssertEqual(ScratchStore.list(in: folder).count, 0, "the bin is not a note of the list")
    }

    func testAScratchNoteIsOnlyOneOfTheFolder() throws {
        let inside = try write("2026-11-14-103200", "a")

        XCTAssertTrue(ScratchStore.holds(inside, in: folder))
        XCTAssertFalse(ScratchStore.holds(folder.deletingLastPathComponent().appendingPathComponent("Other.md"), in: folder))
    }
}
