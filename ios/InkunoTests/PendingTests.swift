import Foundation
import XCTest
@testable import Inkuno

/// Sidecar lists (version 2) and accept/reject, mirroring `platform/sidecar.rs`.
final class PendingTests: XCTestCase {
    private let list = """
        {"version": 2, "changes": [{"id": 1, "author": "assistant", "at": 7, \
        "old": "old\\n", "new": "new\\n", "before": "a\\n", "after": "b\\n"}]}
        """

    func testParsesVersionTwoLists() {
        let entries = PendingList.parse(Data(list.utf8))

        XCTAssertEqual(entries, [PendingChange(
            id: 1, author: "assistant", at: 7, old: "old\n", new: "new\n",
            before: "a\n", after: "b\n", orphanSince: nil
        )])
    }

    func testUnreadableOrMissingListsAreEmpty() {
        XCTAssertEqual(PendingList.parse(Data("not json".utf8)), [])
        XCTAssertEqual(PendingList.parse(Data("{\"changes\": [{\"id\": \"x\"}]}".utf8)), [])
        XCTAssertEqual(PendingList.parse(Data("{\"version\": 2}".utf8)), [])
    }

    func testListWithoutOrphanFieldsReadsWithNoStamp() {
        let old = "{\"version\": 2, \"changes\": [{\"id\": 1, \"author\": \"\", \"at\": 2, " +
            "\"old\": \"\", \"new\": \"x\\n\", \"before\": \"a\\n\", \"after\": \"b\\n\"}]}"
        let entries = PendingList.parse(Data(old.utf8))

        XCTAssertEqual(entries.count, 1)
        XCTAssertNil(entries[0].orphanSince)
    }

    func testEncodeWritesVersionTwoWithTrailingNewline() {
        let entry = PendingChange(id: 2, author: "assistant", at: 0, old: "o\n", new: "n\n",
                                  before: "", after: "a\n", orphanSince: 9)
        let data = PendingList.encode([entry])
        let back = PendingList.parse(data)

        XCTAssertEqual(back, [entry])
        XCTAssertTrue(data.last == UInt8(ascii: "\n"))
        let json = (try? JSONSerialization.jsonObject(with: data) as? [String: Any])
        XCTAssertEqual(json?["version"] as? Int, 2)
    }

    func testAcceptReplacesOldWithNew() {
        let entry = PendingChange(id: 1, author: "a", at: 2, old: "old\n", new: "new\n",
                                  before: "a\n", after: "b\n", orphanSince: nil)

        XCTAssertEqual(PendingList.accept(entry, in: "a\nold\nb\n"), "a\nnew\nb\n")
        // Context gone: first occurrence still wins.
        XCTAssertEqual(PendingList.accept(entry, in: "x\nold\ny\n"), "x\nnew\ny\n")
        // Gone entirely: nil, so the caller keeps the entry.
        XCTAssertNil(PendingList.accept(entry, in: "nothing here\n"))
    }

    func testRejectNeverTouchesTheText() {
        let entry = PendingChange(id: 1, author: "a", at: 0, old: "old\n", new: "new\n",
                                  before: "", after: "", orphanSince: nil)

        XCTAssertEqual(PendingList.reject(entry, in: "a\nold\nb\n"), "a\nold\nb\n")
    }

    func testAcceptInsertsAPureInsertion() {
        let entry = PendingChange(id: 1, author: "a", at: 2, old: "", new: "x\n",
                                  before: "a\n", after: "b\n", orphanSince: nil)

        XCTAssertEqual(PendingList.accept(entry, in: "a\nb\n"), "a\nx\nb\n")
    }

    @MainActor
    func testStoreRoundTripsAndAcceptRejectUpdateTheNote() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("inkuno-pending-tests-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let store = VaultStore(vaultURL: root, defaults: UserDefaults(suiteName: "inkuno-pending-\(UUID().uuidString)")!)
        let note = root.appendingPathComponent("Plan.md")
        try Data("a\nold\nb\n".utf8).write(to: note)
        let entry = PendingChange(id: 1, author: "assistant", at: 2, old: "old\n", new: "new\n",
                                  before: "a\n", after: "b\n", orphanSince: nil)
        let sidecar = root.appendingPathComponent(".inkuno/pending/Plan.md.json")
        try FileManager.default.createDirectory(at: sidecar.deletingLastPathComponent(), withIntermediateDirectories: true)
        try PendingList.encode([entry]).write(to: sidecar)

        XCTAssertEqual(store.pendingCount(for: note), 1)
        XCTAssertTrue(try store.acceptPendingEntry(id: 1, for: note))
        XCTAssertEqual(try String(contentsOf: note, encoding: .utf8), "a\nnew\nb\n")
        XCTAssertEqual(store.pendingCount(for: note), 0)
        XCTAssertFalse(FileManager.default.fileExists(atPath: sidecar.path), "empty list: no file")

        try PendingList.encode([entry]).write(to: sidecar)
        try store.rejectPendingEntry(id: 1, for: note)
        XCTAssertEqual(try String(contentsOf: note, encoding: .utf8), "a\nnew\nb\n", "reject leaves the file")
        XCTAssertEqual(store.pendingCount(for: note), 0)

        // The note as it was when the change was saved: accept-all applies it.
        try Data("a\nold\nb\n".utf8).write(to: note)
        try PendingList.encode([entry]).write(to: sidecar)
        try store.resolveAllPending(for: note, accept: true)
        XCTAssertEqual(try String(contentsOf: note, encoding: .utf8), "a\nnew\nb\n")
        XCTAssertEqual(store.pendingCount(for: note), 0)
    }
}
