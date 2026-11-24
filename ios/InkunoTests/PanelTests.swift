import Foundation
import XCTest
@testable import Inkuno

/// Revision headers: render/parse roundtrip and header-only reads, mirroring
/// `crates/inkuno-logic/src/revision.rs` tests; comments mirroring `markdown/mod.rs`.
final class PanelTests: XCTestCase {
    // MARK: Revision format — revision.rs `writes_the_documented_front_matter`

    func testRendersTheDocumentedFrontMatter() {
        let text = RevisionFormat.render(
            number: 3, parent: 2, author: "sachin", date: "2026-11-14T10:32:00Z",
            kind: .checkpoint, note: "dropped the plugin idea, keep it local-first", text: "# Plan\n"
        )

        XCTAssertEqual(
            text,
            "---\nrevision: 3\nparent: 2\nauthor: sachin\ndate: 2026-11-14T10:32:00Z\n" +
            "kind: checkpoint\nnote: dropped the plugin idea, keep it local-first\n---\n# Plan\n"
        )
    }

    func testFirstRevisionHasNoParent() {
        let text = RevisionFormat.render(
            number: 1, parent: nil, author: "sachin", date: "2026-11-14T10:32:00Z",
            kind: .checkpoint, note: nil, text: "a"
        )

        XCTAssertFalse(text.contains("parent:"))
        XCTAssertNil(try? RevisionFormat.parse(text).get().header.parent)
    }

    func testRoundTripsTrickyNotesAndAuthors() throws {
        for note in ["colons: everywhere: here", "a \"quoted\" word", "two\nlines", "  spaces around  ",
                    "\"starts with a quote", "---", "key: value", "tab\there", "back\\slash",
                    "Ünïcödé 🦀", "control\u{1}char"] {
            let text = RevisionFormat.render(
                number: 2, parent: 1, author: "Sachin Kolisetty", date: "2026-11-14T10:32:00Z",
                kind: .auto, note: note, text: "body\n---\nmore\nCRLF\r\nline\r\n"
            )
            let parsed = try RevisionFormat.parse(text).get()
            XCTAssertEqual(parsed.header.number, 2)
            XCTAssertEqual(parsed.header.parent, 1)
            XCTAssertEqual(parsed.header.kind, .auto)
            XCTAssertEqual(parsed.header.note, note)
            XCTAssertEqual(parsed.text, "body\n---\nmore\nCRLF\r\nline\r\n")
        }
    }

    func testEmptyNoteIsNoNote() {
        let text = RevisionFormat.render(
            number: 1, parent: nil, author: "x", date: "d",
            kind: .checkpoint, note: "", text: "x"
        )

        XCTAssertFalse(text.contains("note:"))
    }

    func testRefusesFilesThatAreNotRevisions() {
        XCTAssertThrowsError(try RevisionFormat.parse("# just a note\n").get())
        XCTAssertThrowsError(try RevisionFormat.parse("").get())
        XCTAssertThrowsError(try RevisionFormat.parse("---\nrevision: 1\n").get())
        XCTAssertThrowsError(try RevisionFormat.parse("---\nauthor: x\n---\n").get())
        XCTAssertThrowsError(try RevisionFormat.parse("---\nrevision: 0\n---\n").get())
        XCTAssertThrowsError(try RevisionFormat.parse("---\nrevision: 1\nparent: two\n---\n").get())
        XCTAssertThrowsError(try RevisionFormat.parse("---\nrevision: 1\nkind: manual\n---\n").get())
        // A cut inside the header is unclosed; unknown keys and missing author/date default.
        XCTAssertThrowsError(try RevisionFormat.parse("---\nrevision: 2\nauthor: ana\n").get())
        let parsed = try? RevisionFormat.parse("---\nrevision: 5\nfuture: whatever\n---\nbody").get()
        XCTAssertEqual(parsed?.header.author, "unknown")
        XCTAssertEqual(parsed?.text, "body")
    }

    func testHeaderIsReadWithoutTheText() throws {
        let full = RevisionFormat.render(
            number: 4, parent: 3, author: "ana", date: "2026-11-14T10:32:00Z",
            kind: .auto, note: "kept: local", text: "# Plan\n---\nnot a closing line\n"
        )
        let parsed = try RevisionFormat.parseHeader(full).get()

        XCTAssertEqual(parsed.header.number, 4)
        XCTAssertEqual(String(full[parsed.textStart...]), "# Plan\n---\nnot a closing line\n")
    }

    func testNumbersFilesAndTheNextRevision() {
        XCTAssertEqual(RevisionFormat.fileName(1), "0001.md")
        XCTAssertEqual(RevisionFormat.fileName(12_345), "12345.md")
        XCTAssertEqual(RevisionFormat.number(of: "0003.md"), 3)
        XCTAssertEqual(RevisionFormat.number(of: "0000.md"), nil)
        XCTAssertEqual(RevisionFormat.number(of: "notes.md"), nil)
        XCTAssertEqual(RevisionFormat.number(of: "0003.txt"), nil)
        XCTAssertEqual(RevisionFormat.nextNumber([1, 2, 7]), 8)
        XCTAssertEqual(RevisionFormat.nextNumber([] as [UInt32]), 1)
    }

    func testMapsVaultNotesToTheirFolders() {
        let vault = URL(fileURLWithPath: "/vault")
        XCTAssertEqual(
            RevisionFormat.folder(root: vault, dir: ".inkuno/revisions", note: vault.appendingPathComponent("ideas/plan.md"))?.path,
            "/vault/.inkuno/revisions/ideas/plan.md"
        )
        XCTAssertNil(RevisionFormat.folder(root: vault, dir: ".inkuno/revisions", note: URL(fileURLWithPath: "/elsewhere/x.md")))
        XCTAssertNil(RevisionFormat.folder(root: vault, dir: ".inkuno/revisions", note: vault))
        // `..` after the root would climb out: nothing is mapped.
        XCTAssertNil(RevisionFormat.folder(root: vault, dir: ".inkuno/revisions", note: vault.appendingPathComponent("sub/../../x.md")))
    }

    func testRowName() {
        let checkpoint = RevisionHeader(number: 12, parent: 11, author: "sachin", date: "d", kind: .checkpoint, note: nil)
        XCTAssertEqual(RevisionFormat.rowName(checkpoint, auto: "auto"), "#12 · sachin")
        let auto = RevisionHeader(number: 3, parent: 2, author: "sachin", date: "d", kind: .auto, note: "proposed, pending")
        XCTAssertEqual(RevisionFormat.rowName(auto, auto: "auto"), "#3 · sachin · auto · proposed, pending")
    }

    // MARK: Save / restore roundtrip through VaultStore

    @MainActor
    func testSaveRefusesUnchangedTextAndRestoreSavesCurrentFirst() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("inkuno-panel-tests-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let store = VaultStore(vaultURL: root, defaults: UserDefaults(suiteName: "inkuno-panel-\(UUID().uuidString)")!)
        let note = root.appendingPathComponent("Plan.md")
        try Data("one\n".utf8).write(to: note)

        XCTAssertNotNil(try store.saveVersion(of: note))
        XCTAssertNil(try store.saveVersion(of: note), "unchanged: no file")

        try Data("two\n".utf8).write(to: note)
        _ = try store.saveVersion(of: note)
        let headers = try store.revisionHeaders(for: note).compactMap { try? $0.get() }
        XCTAssertEqual(headers.map(\.number), [1, 2])

        // An unsaved edit, then restore #1: the current text is kept as a version first.
        try Data("two edited\n".utf8).write(to: note)
        try store.restoreVersion(number: 1, for: note)
        XCTAssertEqual(try String(contentsOf: note, encoding: .utf8), "one\n")
        let after = try store.revisionHeaders(for: note).compactMap { try? $0.get() }
        XCTAssertEqual(after.map(\.number), [1, 2, 3, 4])
        XCTAssertEqual(after[2].kind, .auto)
        XCTAssertEqual(after[2].note, "before restoring version 1")
        XCTAssertEqual(try store.readRevisionText(number: 3, for: note), "two edited\n")
        XCTAssertEqual(after[3].note, "version 1 restored")
        XCTAssertEqual(try store.readRevisionText(number: 4, for: note), "one\n")

        XCTAssertThrowsError(try store.restoreVersion(number: 1, for: note),
                             "already this version")
    }

    // MARK: Comments — markdown/mod.rs `comment_on` shape

    func testParsesWhoTimeAndText() {
        let comments = CommentFormat.parse("Ship it<!-- comment: Ben, 09:12: Keep it to Quiet for v1. -->\nplain\n")

        XCTAssertEqual(comments.count, 1)
        XCTAssertEqual(comments[0].passage, "Ship it")
        XCTAssertEqual(comments[0].who, "Ben")
        XCTAssertEqual(comments[0].time, "09:12")
        XCTAssertEqual(comments[0].text, "Keep it to Quiet for v1.")
        XCTAssertEqual(comments[0].line, 0)
    }

    func testParsesBareCommentAndSkipsFences() {
        let comments = CommentFormat.parse("```\ncode<!-- comment: x -->\n```\nnote<!-- comment: read this -->\n")

        XCTAssertEqual(comments.count, 1)
        XCTAssertEqual(comments[0].passage, "note")
        XCTAssertEqual(comments[0].text, "read this")
    }

    func testResolveKeepsThePassage() {
        let text = "Ship it<!-- comment: Ben: Keep it. -->\nnext\n"
        let comments = CommentFormat.parse(text)

        XCTAssertEqual(CommentFormat.resolve(in: text, comment: comments[0]), "Ship it\nnext\n")
    }

    func testResolveOfALoneCommentTakesTheLine() {
        let text = "keep\n<!-- comment: whole line -->\nend\n"
        let comments = CommentFormat.parse(text)
        XCTAssertEqual(comments[0].passage, "")

        XCTAssertEqual(CommentFormat.resolve(in: text, comment: comments[0]), "keep\nend\n")
    }

    // MARK: Outline

    func testParsesHeadingsOutsideFences() {
        let headings = Outline.parse("# Title\n\n## Sub\n```\n# code\n```\n### Deep\nno space#no\n")

        XCTAssertEqual(headings.map(\.level), [1, 2, 3])
        XCTAssertEqual(headings.map(\.text), ["Title", "Sub", "Deep"])
        XCTAssertEqual(headings.map(\.line), [0, 2, 6])
    }

    // MARK: Comment written from the block menu

    func testCommentOnALineWritesTheMarkup() {
        XCTAssertEqual(CommentFormat.commented(line: "Ship it", said: "Keep it short", inTable: false),
                       "Ship it<!-- comment: Keep it short -->")
    }

    func testCommentSpacesOutTheCloseMarkInTheText() {
        let written = CommentFormat.commented(line: "Ship it", said: "a --> b", inTable: false)

        XCTAssertEqual(written, "Ship it<!-- comment: a -- > b -->")
        XCTAssertEqual(CommentFormat.parse(written ?? "").map(\.text), ["a -- > b"])
    }

    func testCommentRefusesABlankLineAnEmptyTextAndCommentMarks() {
        XCTAssertNil(CommentFormat.commented(line: "   ", said: "x", inTable: false))
        XCTAssertNil(CommentFormat.commented(line: "Ship it", said: "  ", inTable: false))
        XCTAssertNil(CommentFormat.commented(line: "{==marked==}", said: "x", inTable: false))
    }

    func testCommentOnATableLineEscapesThePipe() {
        XCTAssertEqual(CommentFormat.commented(line: "| a |", said: "x | y", inTable: true),
                       "| a |<!-- comment: x \\| y -->")
    }
}
