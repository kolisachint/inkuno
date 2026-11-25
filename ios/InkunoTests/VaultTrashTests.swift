import Foundation
import XCTest
@testable import Inkuno

@MainActor
final class VaultTrashTests: XCTestCase {
    private var root: URL!
    private var store: VaultStore!

    override func setUp() async throws {
        try await super.setUp()
        root = FileManager.default.temporaryDirectory
            .appendingPathComponent("inkuno-trash-tests-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        store = VaultStore(vaultURL: root, defaults: UserDefaults(suiteName: "inkuno-trash-\(UUID().uuidString)")!)
    }

    override func tearDown() async throws {
        try? FileManager.default.removeItem(at: root)
        store = nil
        try await super.tearDown()
    }

    private func touch(_ relativePath: String, _ contents: String = "") throws -> URL {
        let url = root.appendingPathComponent(relativePath)
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try Data(contents.utf8).write(to: url)
        return url
    }

    // MARK: Trash naming — platform/vault.rs `trash`: `Name`, then `2 Name`, `3 Name`…

    func testTrashMovesTheNoteAndKeepsItsHistory() throws {
        let note = try touch("Idea.md", "# Idea\n")
        _ = try store.saveVersion(of: note)

        let trashed = try store.trashItem(at: note)

        XCTAssertEqual(trashed.lastPathComponent, "Idea.md")
        XCTAssertFalse(FileManager.default.fileExists(atPath: note.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: trashed.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appendingPathComponent(".inkuno/revisions/Idea.md").path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: root.appendingPathComponent(".inkuno/trash/Idea.md/0001.md").path))
    }

    func testTrashPicksAFreeName() throws {
        let first = try touch("Idea.md", "one\n")
        let trashedFirst = try store.trashItem(at: first)
        XCTAssertEqual(trashedFirst.lastPathComponent, "Idea.md")

        let second = try touch("Idea.md", "two\n")
        let trashedSecond = try store.trashItem(at: second)
        XCTAssertEqual(trashedSecond.lastPathComponent, "2 Idea.md")
    }

    func testMoveToTrashNeverDestroys() throws {
        let note = try touch("Keep.md", "precious\n")
        let trashed = try store.trashItem(at: note)

        XCTAssertEqual(try String(contentsOf: trashed, encoding: .utf8), "precious\n")
    }

    // MARK: Restore — platform/vault.rs `restore`: back at the root, numbered when taken

    func testRestoreBringsTheNoteBack() throws {
        let note = try touch("Idea.md", "# Idea\n")
        _ = try store.saveVersion(of: note)
        let trashed = try store.trashItem(at: note)

        let restored = try store.restoreFromTrash(at: trashed)

        XCTAssertEqual(restored, note)
        XCTAssertEqual(try String(contentsOf: restored, encoding: .utf8), "# Idea\n")
        // Its history comes back with it.
        XCTAssertTrue(FileManager.default.fileExists(atPath: root.appendingPathComponent(".inkuno/revisions/Idea.md").path))
    }

    func testRestoreNumbersWhenTheNameIsTaken() throws {
        let note = try touch("Idea.md", "old\n")
        let trashed = try store.trashItem(at: note)
        _ = try touch("Idea.md", "new note\n")

        let restored = try store.restoreFromTrash(at: trashed)

        XCTAssertEqual(restored.lastPathComponent, "Idea 2.md")
        XCTAssertEqual(try String(contentsOf: restored, encoding: .utf8), "old\n")
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("Idea.md"), encoding: .utf8), "new note\n")
    }

    func testTrashListsMostRecentlyModifiedFirst() throws {
        let first = try touch("First.md", "1\n")
        let second = try touch("Second.md", "2\n")
        try store.trashItem(at: first)
        // A later modification sorts first.
        try FileManager.default.setAttributes([.modificationDate: Date().addingTimeInterval(60)], ofItemAtPath: second.path)
        try store.trashItem(at: second)

        let listed = try store.trashedNotes()

        XCTAssertEqual(listed.map(\.title), ["Second", "First"])
    }
}
