import Foundation
import InkunoCore
import XCTest
@testable import Inkuno

/// Rename, move, duplicate and copy-link, plus the nameTaken refusals.
@MainActor
final class VaultOpsTests: XCTestCase {
    private var root: URL!
    private var store: VaultStore!

    override func setUp() async throws {
        try await super.setUp()
        root = FileManager.default.temporaryDirectory
            .appendingPathComponent("inkuno-ops-tests-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        store = VaultStore(vaultURL: root, defaults: UserDefaults(suiteName: "inkuno-ops-\(UUID().uuidString)")!)
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

    func testRenameRefusesTakenNames() throws {
        let note = try touch("Draft.md", "a\n")
        _ = try touch("Taken.md", "b\n")

        XCTAssertThrowsError(try store.renameItem(at: note, to: "Taken.md")) { error in
            guard case VaultError.nameTaken = error else { return XCTFail("expected nameTaken, got \(error)") }
        }
        XCTAssertTrue(FileManager.default.fileExists(atPath: note.path), "refused: the note stays")
    }

    func testRenameKeepsTheExtensionAndMovesHistory() throws {
        let note = try touch("Draft.md", "a\n")
        _ = try store.saveVersion(of: note)

        let renamed = try store.renameItem(at: note, to: "Final")

        XCTAssertEqual(renamed.lastPathComponent, "Final.md")
        XCTAssertFalse(FileManager.default.fileExists(atPath: note.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: root.appendingPathComponent(".inkuno/revisions/Final.md/0001.md").path))
    }

    func testMoveRefusesTakenNames() throws {
        let note = try touch("a/Note.md", "a\n")
        let folder = try touch("b/placeholder.md", "").deletingLastPathComponent()
        _ = try touch("b/Note.md", "b\n")

        XCTAssertThrowsError(try store.moveItem(at: note, toFolder: folder)) { error in
            guard case VaultError.nameTaken = error else { return XCTFail("expected nameTaken, got \(error)") }
        }
        XCTAssertTrue(FileManager.default.fileExists(atPath: note.path))
    }

    func testMovePutsTheNoteInTheFolder() throws {
        let note = try touch("a/Note.md", "a\n")
        let folder = root.appendingPathComponent("b", isDirectory: true)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)

        let moved = try store.moveItem(at: note, toFolder: folder)

        XCTAssertEqual(moved, folder.appendingPathComponent("Note.md"))
        XCTAssertFalse(FileManager.default.fileExists(atPath: note.path))
    }

    func testDuplicateNumbersFreeNames() throws {
        let note = try touch("Plan.md", "a\n")

        let first = try store.duplicateItem(at: note)
        let second = try store.duplicateItem(at: note)

        XCTAssertEqual(first.lastPathComponent, "Plan 2.md")
        XCTAssertEqual(second.lastPathComponent, "Plan 3.md")
        XCTAssertEqual(try String(contentsOf: first, encoding: .utf8), "a\n")
    }

    func testCopyLinkWritesARelativeLink() throws {
        let note = try touch("ideas/Plan.md", "# Plan\n")

        XCTAssertEqual(store.copyLink(for: note), "[Plan](ideas/Plan.md)")
    }

    /// A vault named through a symlink and the same vault named by its real path must agree: the relative
    /// path and the pending list (its sidecar) are found from either form. `/tmp` is such a link on macOS.
    func testVaultThroughASymlinkFindsItsNoteAndSidecar() throws {
        let real = FileManager.default.temporaryDirectory
            .appendingPathComponent("inkuno-link-tests-\(UUID().uuidString)", isDirectory: true)
        let vaultReal = real.appendingPathComponent("vault", isDirectory: true)
        let link = FileManager.default.temporaryDirectory
            .appendingPathComponent("inkuno-link-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: vaultReal, withIntermediateDirectories: true)
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: vaultReal)
        defer {
            try? FileManager.default.removeItem(at: real)
            try? FileManager.default.removeItem(at: link)
        }
        let note = vaultReal.appendingPathComponent("Plan.md")
        try Data("a\nold\nb\n".utf8).write(to: note)
        let entry = PendingEntry(id: 1, author: "assistant", at: 2, old: "old\n", new: "new\n",
                                 before: "a\n", after: "b\n", orphanSince: nil)
        let sidecar = vaultReal.appendingPathComponent(".inkuno/pending/Plan.md.json")
        try FileManager.default.createDirectory(at: sidecar.deletingLastPathComponent(), withIntermediateDirectories: true)
        try Data(sidecarRender(entries: [entry]).utf8).write(to: sidecar)

        for (rootURL, noteURL) in [
            (link, link.appendingPathComponent("Plan.md")),
            (vaultReal, vaultReal.appendingPathComponent("Plan.md")),
        ] {
            let store = VaultStore(vaultURL: rootURL, defaults: UserDefaults(suiteName: "inkuno-link-\(UUID().uuidString)")!)
            XCTAssertEqual(store.relativePath(of: noteURL), "Plan.md", "root \(rootURL.path)")
            XCTAssertEqual(store.relativePath(of: note), "Plan.md", "real note path, root \(rootURL.path)")
            XCTAssertEqual(store.pendingCount(for: noteURL), 1, "root \(rootURL.path)")
        }
    }
}
