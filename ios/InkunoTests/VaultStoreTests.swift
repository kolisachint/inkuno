import Foundation
import XCTest
@testable import Inkuno

@MainActor
final class VaultStoreTests: XCTestCase {
    private var root: URL!
    private var defaults: UserDefaults!
    private var store: VaultStore!

    override func setUp() async throws {
        try await super.setUp()
        root = FileManager.default.temporaryDirectory
            .appendingPathComponent("inkuno-tests-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defaults = UserDefaults(suiteName: "inkuno-tests-\(UUID().uuidString)")
        store = VaultStore(vaultURL: root, defaults: defaults)
    }

    override func tearDown() async throws {
        try? FileManager.default.removeItem(at: root)
        try await super.tearDown()
    }

    private func touch(_ relativePath: String, _ contents: String = "") throws -> URL {
        let url = root.appendingPathComponent(relativePath)
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try Data(contents.utf8).write(to: url)
        return url
    }

    private func setModified(_ url: URL, daysAgo: Double, from now: Date) throws {
        try FileManager.default.setAttributes(
            [.modificationDate: now.addingTimeInterval(-daysAgo * 86_400)], ofItemAtPath: url.path
        )
    }

    // MARK: Listing

    func testListsFoldersBeforeNotesAndSkipsHiddenAndOtherFiles() throws {
        _ = try touch("Projects/plan.md")
        _ = try touch("Ideas.md", "x")
        _ = try touch("picture.png")
        _ = try touch(".trash/gone.md")
        _ = try touch(".hidden.md")

        let entries = try store.entries(in: root)

        XCTAssertEqual(entries.map(\.title), ["Projects", "Ideas"])
        XCTAssertEqual(entries.map(\.kind), [.folder, .note])
    }

    func testICloudPlaceholderIsListedAsANoteWithItsRealName() throws {
        _ = try touch(".Later.md.icloud")
        _ = try touch(".Something.txt.icloud")

        let entries = try store.entries(in: root)

        XCTAssertEqual(entries.count, 1)
        XCTAssertEqual(entries.first?.kind, .cloudPlaceholder)
        XCTAssertEqual(entries.first?.url.lastPathComponent, "Later.md")
        XCTAssertEqual(entries.first?.title, "Later")
    }

    func testAllNotesWalksSubfolders() throws {
        _ = try touch("a.md")
        _ = try touch("sub/b.md")
        _ = try touch("sub/deeper/c.md")

        let titles = try store.allNotes().map(\.title).sorted()

        XCTAssertEqual(titles, ["a", "b", "c"])
    }

    // MARK: Reading and writing

    func testWriteThenReadRoundTripsAndOverwritesAtomically() throws {
        let url = root.appendingPathComponent("Note.md")
        try store.writeNote("first", to: url)
        XCTAssertEqual(try store.readNote(at: url), "first")

        try store.writeNote("second, with ünïcode ✓", to: url)
        XCTAssertEqual(try store.readNote(at: url), "second, with ünïcode ✓")
        XCTAssertEqual(try store.entries(in: root).map(\.title), ["Note"], "no temporary files left behind")
    }

    func testCreateRefusesAnExistingName() throws {
        let url = root.appendingPathComponent("Taken.md")
        try store.createNote(at: url, text: "keep me")

        XCTAssertThrowsError(try store.createNote(at: url, text: "overwrite")) { error in
            guard case VaultError.nameTaken = error else {
                return XCTFail("expected nameTaken, got \(error)")
            }
        }
        XCTAssertEqual(try store.readNote(at: url), "keep me")
    }

    func testReadingAPlaceholderWithoutItsFileThrowsNotDownloaded() throws {
        _ = try touch(".Cloud.md.icloud")
        let url = root.appendingPathComponent("Cloud.md")

        XCTAssertThrowsError(try store.readNote(at: url)) { error in
            guard case VaultError.notDownloaded = error else {
                return XCTFail("expected notDownloaded, got \(error)")
            }
        }
    }

    func testNewNoteURLSkipsTakenNames() throws {
        _ = try touch("Untitled.md")
        _ = try touch("Untitled 2.md")

        XCTAssertEqual(store.newNoteURL(in: root).lastPathComponent, "Untitled 3.md")
    }

    // MARK: Daily and scratch notes

    func testOpenTodayCreatesTheDailyNoteOnce() throws {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "UTC")!
        let date = calendar.date(from: DateComponents(year: 2026, month: 11, day: 24, hour: 9))!

        let first = try store.openToday(now: date, calendar: calendar)
        try store.writeNote("hello", to: first)
        let second = try store.openToday(now: date, calendar: calendar)

        XCTAssertEqual(first.lastPathComponent, "2026-11-24.md")
        XCTAssertEqual(first, second)
        XCTAssertEqual(try store.readNote(at: second), "hello", "opening again must not truncate the note")
    }

    func testDailyNoteFileNameUsesLocalCalendarDate() {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "Pacific/Auckland")!
        let date = calendar.date(from: DateComponents(year: 2027, month: 1, day: 5, hour: 1))!

        XCTAssertEqual(VaultStore.dailyNoteFileName(for: date, calendar: calendar), "2027-01-05.md")
    }

    // MARK: Recall

    func testRecallOffersTheDailyNoteOfAMonthAgo() throws {
        let now = Date()
        let month = Calendar.current.date(byAdding: .month, value: -1, to: now)!
        let daily = try touch(VaultStore.dailyNoteFileName(for: month), "# Day\nfirst line\n")
        _ = try touch("yesterday.md", "# Today\n")
        try setModified(daily, daysAgo: 30, from: now)

        let recalled = try store.recalledNotes(now: now)

        XCTAssertTrue(recalled.contains { $0.path == daily.lastPathComponent }, "core recall: the daily note of a month ago")
        XCTAssertEqual(recalled.first { $0.path == daily.lastPathComponent }?.line, "first line")
    }

    // MARK: Index

    func testBacklinksFollowTheWritesThisStoreMakes() throws {
        let plan = try touch("Plan.md", "# Plan\n")
        let linker = try touch("Notes/Ideas.md", "see [[plan]]\n")

        XCTAssertEqual(try store.backlinks(to: plan).map(\.url), [linker])

        try store.writeNote("no link now\n", to: linker)

        XCTAssertEqual(try store.backlinks(to: plan).count, 0, "the write updates the index")
    }

    // MARK: Folder state

    func testFreshStoreHasNoVaultUntilOneIsChosen() {
        let empty = VaultStore(defaults: UserDefaults(suiteName: "inkuno-empty-\(UUID().uuidString)")!)

        XCTAssertFalse(empty.hasVault)
        XCTAssertEqual(empty.displayName, "Not chosen")
    }

    func testUnresolvableBookmarkIsForgotten() {
        let suite = "inkuno-bad-bookmark-\(UUID().uuidString)"
        let bookmarkDefaults = UserDefaults(suiteName: suite)!
        bookmarkDefaults.set(Data("not a bookmark".utf8), forKey: VaultStore.bookmarkKey)

        let restored = VaultStore(defaults: bookmarkDefaults)

        XCTAssertFalse(restored.hasVault)
        XCTAssertNotNil(restored.lastError)
        XCTAssertNil(bookmarkDefaults.data(forKey: VaultStore.bookmarkKey))
        bookmarkDefaults.removePersistentDomain(forName: suite)
    }
}
