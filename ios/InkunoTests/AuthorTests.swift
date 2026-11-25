import Foundation
import XCTest
@testable import Inkuno

/// The author name in Settings › Editor: cleaned like the desktop's `clean_author`
/// (`crates/inkuno-logic/src/revision.rs`), and used for revisions and comment bylines.
final class AuthorTests: XCTestCase {
    private var defaults: UserDefaults!
    private var suiteName = ""

    override func setUp() {
        super.setUp()
        suiteName = "inkuno-author-\(UUID().uuidString)"
        defaults = UserDefaults(suiteName: suiteName)!
    }

    override func tearDown() {
        defaults.removePersistentDomain(forName: suiteName)
        defaults = nil
        super.tearDown()
    }

    // MARK: Cleaning

    func testCleanTrimsAndCollapsesWhitespace() {
        XCTAssertEqual(AuthorName.clean("  Ana   Kim  "), "Ana Kim")
        XCTAssertEqual(AuthorName.clean(""), "")
        XCTAssertEqual(AuthorName.clean("   "), "")
    }

    func testCleanStripsNewlinesAndTabs() {
        XCTAssertEqual(AuthorName.clean("bo\nb"), "bo b")
        XCTAssertEqual(AuthorName.clean("a\tb"), "a b")
        XCTAssertEqual(AuthorName.clean("a\r\nb"), "a b")
    }

    func testCleanKeepsANormalName() {
        XCTAssertEqual(AuthorName.clean("Sachin Kolisetty"), "Sachin Kolisetty")
    }

    // MARK: Resolution

    func testResolvedFallsBackToUnknownWhenEmpty() {
        XCTAssertEqual(AuthorName.resolved(in: defaults), "unknown")
        defaults.set("", forKey: AppearanceKey.authorName)
        XCTAssertEqual(AuthorName.resolved(in: defaults), "unknown")
        defaults.set("   ", forKey: AppearanceKey.authorName)
        XCTAssertEqual(AuthorName.resolved(in: defaults), "unknown")
    }

    func testResolvedCleansTheStoredName() {
        defaults.set("  Ana Kim  ", forKey: AppearanceKey.authorName)
        XCTAssertEqual(AuthorName.resolved(in: defaults), "Ana Kim")
    }

    // MARK: Comment authors

    func testRelativeTimeReadsTheStampAndKeepsAnUnknownOne() {
        let now = Date(timeIntervalSince1970: 1_795_514_520 + 3_600)

        XCTAssertNotEqual(CommentFormat.relativeTime("2026-11-24T10:02Z", now: now), "2026-11-24T10:02Z")
        XCTAssertEqual(CommentFormat.relativeTime("09:12", now: now), "09:12")
    }

    func testAnUnknownAuthorShowsAsUnknown() {
        XCTAssertEqual(CommentFormat.displayName(nil), "Unknown")
        XCTAssertEqual(CommentFormat.displayName("Ana"), "Ana")
    }

    // MARK: Revisions carry the name

    @MainActor
    func testSavedVersionsCarryTheSettingsName() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("inkuno-author-tests-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let store = VaultStore(vaultURL: root)

        let savedKey = UserDefaults.standard.string(forKey: AppearanceKey.authorName)
        defer {
            if let savedKey {
                UserDefaults.standard.set(savedKey, forKey: AppearanceKey.authorName)
            } else {
                UserDefaults.standard.removeObject(forKey: AppearanceKey.authorName)
            }
        }

        let note = root.appendingPathComponent("Idea.md")
        try Data("# Idea\n".utf8).write(to: note)

        // No name set: the desktop's fallback.
        UserDefaults.standard.removeObject(forKey: AppearanceKey.authorName)
        _ = try store.saveVersion(of: note)
        var headers = try store.revisionHeaders(for: note).compactMap { try? $0.get() }
        XCTAssertEqual(headers.last?.author, "unknown")

        // A name set: cleaned onto the revision.
        UserDefaults.standard.set("  Ana Kim  ", forKey: AppearanceKey.authorName)
        try Data("# Idea\nmore\n".utf8).write(to: note)
        _ = try store.saveVersion(of: note)
        headers = try store.revisionHeaders(for: note).compactMap { try? $0.get() }
        XCTAssertEqual(headers.last?.author, "Ana Kim")
    }
}
