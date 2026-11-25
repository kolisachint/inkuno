import Foundation
import InkunoCore
import XCTest
@testable import Inkuno

/// Sidecar lists (version 2) and accept/reject, through the core (`sidecar.rs`, `pending.rs`).
final class PendingTests: XCTestCase {
    private let list = """
        {"version": 2, "changes": [{"id": 1, "author": "assistant", "at": 7, \
        "old": "old\\n", "new": "new\\n", "before": "a\\n", "after": "b\\n"}]}
        """

    func testParsesVersionTwoLists() {
        let entries = sidecarParse(json: list)

        XCTAssertEqual(entries, [PendingEntry(
            id: 1, author: "assistant", at: 7, old: "old\n", new: "new\n",
            before: "a\n", after: "b\n", orphanSince: nil
        )])
    }

    func testUnreadableOrMissingListsAreEmpty() {
        XCTAssertEqual(sidecarParse(json: "not json"), [])
        XCTAssertEqual(sidecarParse(json: "{\"changes\": [{\"id\": \"x\"}]}"), [])
        XCTAssertEqual(sidecarParse(json: "{\"version\": 2}"), [])
    }

    func testListWithoutOrphanFieldsReadsWithNoStamp() {
        let old = "{\"version\": 2, \"changes\": [{\"id\": 1, \"author\": \"\", \"at\": 2, " +
            "\"old\": \"\", \"new\": \"x\\n\", \"before\": \"a\\n\", \"after\": \"b\\n\"}]}"
        let entries = sidecarParse(json: old)

        XCTAssertEqual(entries.count, 1)
        XCTAssertNil(entries[0].orphanSince)
    }

    func testEncodeWritesVersionTwoWithTrailingNewline() throws {
        let entry = PendingEntry(id: 2, author: "assistant", at: 0, old: "o\n", new: "n\n",
                                 before: "", after: "a\n", orphanSince: 9)
        let text = sidecarRender(entries: [entry])
        let back = sidecarParse(json: text)

        XCTAssertEqual(back, [entry])
        XCTAssertTrue(text.hasSuffix("\n"))
        let json = try JSONSerialization.jsonObject(with: Data(text.utf8)) as? [String: Any]
        XCTAssertEqual(json?["version"] as? Int, 2)
    }

    func testSidecarPathsMirrorTheNote() {
        XCTAssertEqual(sidecarPath(notePath: "ideas/plan.md"), ".inkuno/pending/ideas/plan.md.json")
        XCTAssertNil(sidecarPath(notePath: "../x.md"))
    }

    @MainActor
    func testAcceptReplacesOldWithNew() {
        let entry = PendingEntry(id: 1, author: "a", at: 2, old: "old\n", new: "new\n",
                                 before: "a\n", after: "b\n", orphanSince: nil)

        XCTAssertEqual(VaultStore.textAccepting(entry, in: "a\nold\nb\n"), "a\nnew\nb\n")
        // Context gone: the core does not place it (`pending.rs` `place`), so the entry is kept as an orphan.
        XCTAssertNil(VaultStore.textAccepting(entry, in: "x\nold\ny\n"))
        // Gone entirely: nil, so the caller keeps the entry.
        XCTAssertNil(VaultStore.textAccepting(entry, in: "nothing here\n"))
    }

    func testRejectNeverTouchesTheText() {
        let entry = PendingEntry(id: 1, author: "a", at: 0, old: "old\n", new: "new\n",
                                 before: "", after: "", orphanSince: nil)
        let shown = composePending(base: "a\nold\nb\n", entries: [entry])

        XCTAssertEqual(rejectChange(text: shown, index: 0), "a\nold\nb\n")
    }

    @MainActor
    func testAcceptInsertsAPureInsertion() {
        let entry = PendingEntry(id: 1, author: "a", at: 2, old: "", new: "x\n",
                                 before: "a\n", after: "b\n", orphanSince: nil)

        XCTAssertEqual(VaultStore.textAccepting(entry, in: "a\nb\n"), "a\nx\nb\n")
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
        let entry = PendingEntry(id: 1, author: "assistant", at: 2, old: "old\n", new: "new\n",
                                 before: "a\n", after: "b\n", orphanSince: nil)
        let sidecar = root.appendingPathComponent(".inkuno/pending/Plan.md.json")
        try FileManager.default.createDirectory(at: sidecar.deletingLastPathComponent(), withIntermediateDirectories: true)
        try Data(sidecarRender(entries: [entry]).utf8).write(to: sidecar)

        XCTAssertEqual(store.pendingCount(for: note), 1)
        XCTAssertTrue(try store.acceptPendingEntry(id: 1, for: note))
        XCTAssertEqual(try String(contentsOf: note, encoding: .utf8), "a\nnew\nb\n")
        XCTAssertEqual(store.pendingCount(for: note), 0)
        XCTAssertFalse(FileManager.default.fileExists(atPath: sidecar.path), "empty list: no file")

        try Data(sidecarRender(entries: [entry]).utf8).write(to: sidecar)
        try store.rejectPendingEntry(id: 1, for: note)
        XCTAssertEqual(try String(contentsOf: note, encoding: .utf8), "a\nnew\nb\n", "reject leaves the file")
        XCTAssertEqual(store.pendingCount(for: note), 0)

        // The note as it was when the change was saved: accept-all applies it.
        try Data("a\nold\nb\n".utf8).write(to: note)
        try Data(sidecarRender(entries: [entry]).utf8).write(to: sidecar)
        try store.resolveAllPending(for: note, accept: true)
        XCTAssertEqual(try String(contentsOf: note, encoding: .utf8), "a\nnew\nb\n")
        XCTAssertEqual(store.pendingCount(for: note), 0)
    }
}
