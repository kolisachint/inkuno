import Foundation
import Observation

// MARK: - Vault operations: Trash, rename/move/duplicate, versions, pending review
//
// Mirrors the desktop's on-disk behavior (nothing is destroyed or overwritten):
// - Trash is the vault's `.trash` folder (`platform/vault.rs` `trash`, `trashed`, `restore`).
// - A version lives in `.inkuno/revisions/<note-rel-path>/NNNN.md` (`platform/revisions.rs`).
// - Pending changes live in `.inkuno/pending/<note-rel-path>.json` (`platform/sidecar.rs`).
// ponytail: the Rust core (InkunoCore via UniFFI) will own naming, history and pending rules;
// this extension is the Swift bridge until then.

extension VaultStore {
    // MARK: Layout

    /// The vault's trash folder (created on first use).
    var trashDir: URL? {
        vaultURL?.appendingPathComponent(".trash", isDirectory: true)
    }

    func pendingFile(for note: URL) -> URL? {
        guard let root = vaultURL else { return nil }
        return RevisionFormat.folder(root: root, dir: ".inkuno/pending", note: note)
            .map { $0.appendingPathExtension("json") }
    }

    func revisionDir(for note: URL) -> URL? {
        guard let root = vaultURL else { return nil }
        return RevisionFormat.folder(root: root, dir: ".inkuno/revisions", note: note)
    }

    // MARK: Trash

    /// Contents of the vault's trash, most recently modified first
    /// (`platform/vault.rs` `trashed`).
    func trashedNotes() throws -> [VaultEntry] {
        guard let root = vaultURL, let bin = trashDir else { throw VaultError.noVault }
        guard FileManager.default.fileExists(atPath: bin.path) else { return [] }
        let items = try coordinateRead(bin) {
            try FileManager.default.contentsOfDirectory(at: $0, includingPropertiesForKeys: [.contentModificationDateKey], options: [])
        }
        let notes = items.filter { $0.pathExtension.lowercased() == "md" }
        return notes.map { url in
            let modified = try? url.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate
            return VaultEntry(url: url, kind: .note, modified: modified)
        }
        .sorted { ($0.modified ?? .distantPast) > ($1.modified ?? .distantPast) }
    }

    func trashCount() -> Int {
        (try? trashedNotes().count) ?? 0
    }

    /// The original folder of a trashed note: the vault root (the trash does not keep where the
    /// note came from). Used only for display ("in Notes").
    func trashOrigin(of url: URL) -> String {
        guard let root = vaultURL else { return "" }
        let stem = url.deletingPathExtension().lastPathComponent
        let sibling = root.appendingPathComponent(url.lastPathComponent)
        if !FileManager.default.fileExists(atPath: sibling.path),
           stem == url.deletingPathExtension().lastPathComponent {
            return root.lastPathComponent
        }
        return root.lastPathComponent
    }

    /// Moves the note or folder to the vault's trash under a free name: `Name.md`, then
    /// `2 Name.md`, `3 Name.md`… (`platform/vault.rs` `trash`). Nothing is destroyed. The note's
    /// history and pending list move to their bins with it (`revisions.rs` `trash_along`,
    /// `sidecar.rs` `trash_along`), moved aside under a free name when a stale entry sits there.
    @discardableResult
    func trashItem(at url: URL) throws -> URL {
        guard let root = vaultURL, let bin = trashDir else { throw VaultError.noVault }
        try FileManager.default.createDirectory(at: bin, withIntermediateDirectories: true)
        let name = url.lastPathComponent
        let free = (1...).lazy.map { n in
            n == 1 ? bin.appendingPathComponent(name) : bin.appendingPathComponent("\(n) \(name)")
        }.first { !FileManager.default.fileExists(atPath: $0.path) }!
        try coordinateMove(from: url, to: free)
        moveAsideHistoryAndPending(from: url, trashedName: free.lastPathComponent, root: root)
        return free
    }

    /// Takes a trashed note back to the vault root under its name, numbered when taken
    /// (`platform/vault.rs` `restore`): `Plan.md`, then `Plan 2.md`. Nothing is overwritten.
    /// Its history and pending list come back with it when no history of that name is there
    /// (`revisions.rs` `restore_along`, `sidecar.rs` `restore_along`).
    @discardableResult
    func restoreFromTrash(at url: URL) throws -> URL {
        guard let root = vaultURL else { throw VaultError.noVault }
        let stem = url.deletingPathExtension().lastPathComponent
        let ext = url.pathExtension.isEmpty ? "" : ".\(url.pathExtension)"
        let free = (1...).lazy.map { n in
            n == 1 ? root.appendingPathComponent("\(stem)\(ext)")
                   : root.appendingPathComponent("\(stem) \(n)\(ext)")
        }.first { !FileManager.default.fileExists(atPath: $0.path) }!
        try coordinateMove(from: url, to: free)
        restoreHistoryAndPending(trashedName: url.lastPathComponent, to: free, root: root)
        return free
    }

    // MARK: Rename / move / duplicate

    /// Renames a note or folder in place. Refuses when the name is taken, so nothing is
    /// overwritten. The note's history and pending list follow the new name.
    @discardableResult
    func renameItem(at url: URL, to name: String) throws -> URL {
        guard vaultURL != nil else { throw VaultError.noVault }
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw VaultError.nameTaken(url) }
        var target = url.deletingLastPathComponent().appendingPathComponent(trimmed)
        if url.hasDirectoryPath || (try? url.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true {
            target = url.deletingLastPathComponent().appendingPathComponent(trimmed, isDirectory: true)
        } else if target.pathExtension.isEmpty {
            target = target.appendingPathExtension("md")
        }
        guard !FileManager.default.fileExists(atPath: target.path),
              !FileManager.default.fileExists(atPath: Self.placeholderURL(for: target).path)
        else { throw VaultError.nameTaken(target) }
        try coordinateMove(from: url, to: target)
        moveAlongHistoryAndPending(from: url, to: target)
        return target
    }

    /// Moves a note or folder into another folder of the vault. Refuses when taken.
    @discardableResult
    func moveItem(at url: URL, toFolder folder: URL) throws -> URL {
        guard vaultURL != nil else { throw VaultError.noVault }
        let target = folder.appendingPathComponent(url.lastPathComponent)
        guard !FileManager.default.fileExists(atPath: target.path),
              !FileManager.default.fileExists(atPath: Self.placeholderURL(for: target).path)
        else { throw VaultError.nameTaken(target) }
        try coordinateMove(from: url, to: target)
        moveAlongHistoryAndPending(from: url, to: target)
        return target
    }

    /// Duplicates a note next to itself: `Plan.md`, then `Plan 2.md`, `Plan 3.md`…
    /// (`platform/vault.rs` `free_path` numbering). Folders are not duplicated.
    @discardableResult
    func duplicateItem(at url: URL) throws -> URL {
        guard url.pathExtension.lowercased() == "md" else { throw VaultError.coordination("only notes can be duplicated") }
        let folder = url.deletingLastPathComponent()
        let stem = url.deletingPathExtension().lastPathComponent
        let free = (1...).lazy.map { n in
            n == 1 ? folder.appendingPathComponent("\(stem).md")
                   : folder.appendingPathComponent("\(stem) \(n).md")
        }.first { !FileManager.default.fileExists(atPath: $0.path) }!
        let data = try coordinateRead(url) { try Data(contentsOf: $0) }
        try coordinateWrite(free) { try data.write(to: $0, options: .withoutOverwriting) }
        return free
    }

    /// `[Title](relative/path.md)`: a relative link, so it also works on GitHub and the desktop.
    func copyLink(for url: URL) -> String? {
        guard let root = vaultURL else { return nil }
        let rootPath = root.standardized.path
        let notePath = url.standardized.path
        guard notePath.hasPrefix(rootPath + "/") else { return nil }
        let rel = String(notePath.dropFirst(rootPath.count + 1))
        return "[\(url.deletingPathExtension().lastPathComponent)](\(rel))"
    }

    /// Every folder of the vault, for the Move picker.
    func allFolders() throws -> [URL] {
        guard let root = vaultURL else { throw VaultError.noVault }
        var folders = [root]
        var pending = [root]
        while let folder = pending.popLast() {
            for entry in try entries(in: folder) where entry.kind == .folder {
                folders.append(entry.url)
                pending.append(entry.url)
            }
        }
        return folders.sorted { $0.path < $1.path }
    }

    // MARK: Versions

    /// One unreadable revision file: its number and why (`platform/revisions.rs` `Listed`).
    struct UnreadableVersion: Error {
        var number: UInt32
        var underlying: Error
    }

    /// The headers of every saved version, oldest first (`platform/revisions.rs` `headers`).
    /// Each file is read on its own: an unreadable one keeps its number and the reason.
    func revisionHeaders(for note: URL) throws -> [Result<RevisionHeader, UnreadableVersion>] {
        guard let dir = revisionDir(for: note) else { return [] }
        guard FileManager.default.fileExists(atPath: dir.path) else { return [] }
        let names = try FileManager.default.contentsOfDirectory(atPath: dir.path)
        let numbers = names.compactMap(RevisionFormat.number(of:)).sorted()
        return numbers.map { number in
            let file = dir.appendingPathComponent(RevisionFormat.fileName(number))
            do {
                let head = try readRevisionHeader(at: file)
                return .success(head)
            } catch {
                return .failure(UnreadableVersion(number: number, underlying: error))
            }
        }
    }

    /// Reads a header line by line, up to its closing `---`, without reading the text
    /// (`platform/revisions.rs` `read_header`, 16 KiB cap).
    func readRevisionHeader(at file: URL) throws -> RevisionHeader {
        let handle = try FileHandle(forReadingFrom: file)
        defer { try? handle.close() }
        var head = Data()
        let limit = 16 * 1024
        while head.count <= limit {
            guard let chunk = try handle.read(upToCount: 1024), !chunk.isEmpty else { break }
            head.append(chunk)
            if let text = String(data: head, encoding: .utf8),
               text.components(separatedBy: "\n").dropFirst().contains(where: { $0.trimmingCharacters(in: .newlines).trimmingCharacters(in: CharacterSet(charactersIn: "\r")) == "---" }) {
                break
            }
        }
        guard let text = String(data: head, encoding: .utf8) else {
            throw VaultError.coordination("unreadable version header")
        }
        switch RevisionFormat.parseHeader(text) {
        case .success(let parsed): return parsed.header
        case .failure: throw VaultError.coordination("unreadable version header")
        }
    }

    func readRevisionText(number: UInt32, for note: URL) throws -> String {
        guard let dir = revisionDir(for: note) else { throw VaultError.noVault }
        let file = dir.appendingPathComponent(RevisionFormat.fileName(number))
        let data = try coordinateRead(file) { try Data(contentsOf: $0) }
        guard let text = String(data: data, encoding: .utf8) else {
            throw VaultError.coordination("unreadable version")
        }
        switch RevisionFormat.parse(text) {
        case .success(let parsed): return parsed.text
        case .failure: throw VaultError.coordination("unreadable version")
        }
    }

    /// Saves the note's text as a new Checkpoint revision (`platform/revisions.rs` `save`).
    /// Refused (no file) when the text is the latest revision's already.
    /// ponytail: the author is "unknown" until Settings gains an author field; dates are UTC RFC 3339.
    @discardableResult
    func saveVersion(of note: URL, memo: String? = nil) throws -> UInt32? {
        guard let dir = revisionDir(for: note) else { throw VaultError.noVault }
        let text = try readNote(at: note)
        let existing = (try? FileManager.default.contentsOfDirectory(atPath: dir.path)) ?? []
        let numbers = existing.compactMap(RevisionFormat.number(of:)).sorted()
        if let latest = numbers.last,
           (try? readRevisionText(number: latest, for: note)) == text {
            return nil
        }
        let number = RevisionFormat.nextNumber(numbers)
        let rev = RevisionFormat.render(
            number: number, parent: numbers.last,
            author: "unknown", date: Self.rfc3339UTC(Date()),
            kind: .checkpoint, note: memo, text: text
        )
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let file = dir.appendingPathComponent(RevisionFormat.fileName(number))
        // Never overwrite: a racing save takes the next number instead.
        guard !FileManager.default.fileExists(atPath: file.path) else {
            return try saveVersion(of: note, memo: memo)
        }
        try coordinateWrite(file) { try Data(rev.utf8).write(to: $0, options: .withoutOverwriting) }
        return number
    }

    /// Restores a version: asks nothing itself (the caller confirms), saves the current text as a
    /// version first, then writes the version's text. Mirrors the desktop restore memos
    /// (`shell/versions.rs` `restore_version`): `before restoring version N`, `version N restored`.
    func restoreVersion(number: UInt32, for note: URL) throws {
        let target = try readRevisionText(number: number, for: note)
        let current = try readNote(at: note)
        guard current != target else {
            throw VaultError.coordination("The note is this version already.")
        }
        _ = try saveVersionRaw(of: note, text: current, kind: .auto, memo: "before restoring version \(number)")
        try writeNote(target, to: note)
        _ = try saveVersionRaw(of: note, text: target, kind: .auto, memo: "version \(number) restored")
    }

    private func saveVersionRaw(of note: URL, text: String, kind: RevisionKind, memo: String?) throws {
        guard let dir = revisionDir(for: note) else { throw VaultError.noVault }
        let existing = (try? FileManager.default.contentsOfDirectory(atPath: dir.path)) ?? []
        let numbers = existing.compactMap(RevisionFormat.number(of:)).sorted()
        if let latest = numbers.last,
           (try? readRevisionText(number: latest, for: note)) == text {
            return
        }
        let number = RevisionFormat.nextNumber(numbers)
        let rev = RevisionFormat.render(
            number: number, parent: numbers.last,
            author: "unknown", date: Self.rfc3339UTC(Date()),
            kind: kind, note: memo, text: text
        )
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let file = dir.appendingPathComponent(RevisionFormat.fileName(number))
        guard !FileManager.default.fileExists(atPath: file.path) else {
            try saveVersionRaw(of: note, text: text, kind: kind, memo: memo)
            return
        }
        try coordinateWrite(file) { try Data(rev.utf8).write(to: $0, options: .withoutOverwriting) }
    }

    static func rfc3339UTC(_ date: Date) -> String {
        let formatter = ISO8601DateFormatter()
        formatter.timeZone = TimeZone(identifier: "UTC")
        formatter.formatOptions = [.withInternetDateTime]
        return formatter.string(from: date)
    }

    // MARK: Pending changes

    /// The pending changes beside the note (`platform/sidecar.rs` `read`).
    func pendingEntries(for note: URL) -> [PendingChange] {
        guard let file = pendingFile(for: note),
              let data = try? Data(contentsOf: file) else { return [] }
        return PendingList.parse(data)
    }

    func pendingCount(for note: URL) -> Int {
        pendingEntries(for: note).count
    }

    private func writePendingEntries(_ entries: [PendingChange], for note: URL) throws {
        guard let file = pendingFile(for: note) else { throw VaultError.noVault }
        if entries.isEmpty {
            if FileManager.default.fileExists(atPath: file.path) {
                try FileManager.default.removeItem(at: file)
            }
            return
        }
        try FileManager.default.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        try coordinateWrite(file) { try PendingList.encode(entries).write(to: $0, options: .atomic) }
    }

    /// Accepts one pending change into the note file. Returns false when the change's text is
    /// gone from the note; the entry is kept then.
    @discardableResult
    func acceptPendingEntry(id: UInt32, for note: URL) throws -> Bool {
        var entries = pendingEntries(for: note)
        guard let index = entries.firstIndex(where: { $0.id == id }) else { return false }
        let text = try readNote(at: note)
        guard let next = PendingList.accept(entries[index], in: text) else { return false }
        if next != text { try writeNote(next, to: note) }
        entries.remove(at: index)
        try writePendingEntries(entries, for: note)
        return true
    }

    /// Rejecting drops the entry; the note file is untouched.
    func rejectPendingEntry(id: UInt32, for note: URL) throws {
        var entries = pendingEntries(for: note)
        entries.removeAll { $0.id == id }
        try writePendingEntries(entries, for: note)
    }

    /// Accept all / Reject all. Accept applies each change whose text is still there and keeps
    /// the rest; reject drops them all.
    func resolveAllPending(for note: URL, accept: Bool) throws {
        if !accept {
            try writePendingEntries([], for: note)
            return
        }
        var entries = pendingEntries(for: note)
        var text = try readNote(at: note)
        var kept: [PendingChange] = []
        for entry in entries {
            if let next = PendingList.accept(entry, in: text) {
                text = next
            } else {
                kept.append(entry)
            }
        }
        entries = kept
        try writeNote(text, to: note)
        try writePendingEntries(entries, for: note)
    }

    // MARK: History follows the note

    /// Rename/move: the revision folder and the pending list move with the note
    /// (`revisions.rs` `move_along`, `sidecar.rs` `move_along`).
    private func moveAlongHistoryAndPending(from: URL, to: URL) {
        guard let root = vaultURL else { return }
        if let src = RevisionFormat.folder(root: root, dir: ".inkuno/revisions", note: from),
           let dst = RevisionFormat.folder(root: root, dir: ".inkuno/revisions", note: to),
           FileManager.default.fileExists(atPath: src.path), !FileManager.default.fileExists(atPath: dst.path) {
            try? FileManager.default.createDirectory(at: dst.deletingLastPathComponent(), withIntermediateDirectories: true)
            try? FileManager.default.moveItem(at: src, to: dst)
        }
        if let src = pendingFile(for: from), let dst = pendingFile(for: to),
           FileManager.default.fileExists(atPath: src.path), !FileManager.default.fileExists(atPath: dst.path) {
            try? FileManager.default.createDirectory(at: dst.deletingLastPathComponent(), withIntermediateDirectories: true)
            try? FileManager.default.moveItem(at: src, to: dst)
        }
    }

    /// Trash: history and pending lists wait in their bins under the trashed name
    /// (`revisions.rs` `trash_history`, `sidecar.rs` `trash_along`). A stale bin entry is moved
    /// aside under a free name, never deleted.
    private func moveAsideHistoryAndPending(from: URL, trashedName: String, root: URL) {
        let revBin = root.appendingPathComponent(".inkuno/trash", isDirectory: true)
        if let src = RevisionFormat.folder(root: root, dir: ".inkuno/revisions", note: from),
           FileManager.default.fileExists(atPath: src.path) {
            try? FileManager.default.createDirectory(at: revBin, withIntermediateDirectories: true)
            let slot = revBin.appendingPathComponent(trashedName)
            if FileManager.default.fileExists(atPath: slot.path) {
                let aside = freeName(in: revBin, name: trashedName)
                try? FileManager.default.moveItem(at: slot, to: aside)
            }
            try? FileManager.default.moveItem(at: src, to: slot)
        }
        let pendBin = root.appendingPathComponent(".inkuno/pending-trash", isDirectory: true)
        if let src = pendingFile(for: from), FileManager.default.fileExists(atPath: src.path) {
            try? FileManager.default.createDirectory(at: pendBin, withIntermediateDirectories: true)
            let slot = pendBin.appendingPathComponent(trashedName + ".json")
            if FileManager.default.fileExists(atPath: slot.path) {
                let aside = freeName(in: pendBin, name: trashedName + ".json")
                try? FileManager.default.moveItem(at: slot, to: aside)
            }
            try? FileManager.default.moveItem(at: src, to: slot)
        }
    }

    /// Restore: the bin entries of that exact name come back with the note
    /// (`revisions.rs` `restore_along`, `sidecar.rs` `restore_along`).
    private func restoreHistoryAndPending(trashedName: String, to: URL, root: URL) {
        let revBin = root.appendingPathComponent(".inkuno/trash", isDirectory: true)
        let revSrc = revBin.appendingPathComponent(trashedName)
        if FileManager.default.fileExists(atPath: revSrc.path),
           let dst = RevisionFormat.folder(root: root, dir: ".inkuno/revisions", note: to),
           !FileManager.default.fileExists(atPath: dst.path) {
            try? FileManager.default.createDirectory(at: dst.deletingLastPathComponent(), withIntermediateDirectories: true)
            try? FileManager.default.moveItem(at: revSrc, to: dst)
        }
        let pendBin = root.appendingPathComponent(".inkuno/pending-trash", isDirectory: true)
        let pendSrc = pendBin.appendingPathComponent(trashedName + ".json")
        if FileManager.default.fileExists(atPath: pendSrc.path),
           let dst = pendingFile(for: to), !FileManager.default.fileExists(atPath: dst.path) {
            try? FileManager.default.createDirectory(at: dst.deletingLastPathComponent(), withIntermediateDirectories: true)
            try? FileManager.default.moveItem(at: pendSrc, to: dst)
        }
    }

    /// First free name in `dir`: `Name.md`, `2 Name.md`, `3 Name.md`…
    /// (`platform/revisions.rs` `free_in`, mirrored for both bins).
    private func freeName(in dir: URL, name: String) -> URL {
        (1...).lazy.map { n in
            n == 1 ? dir.appendingPathComponent(name) : dir.appendingPathComponent("\(n) \(name)")
        }.first { !FileManager.default.fileExists(atPath: $0.path) }!
    }

    // MARK: Coordination helpers used by this extension

    private func coordinateMove(from: URL, to: URL) throws {
        var coordinationError: NSError?
        var moveError: Error?
        NSFileCoordinator().coordinate(writingItemAt: from, options: [], writingItemAt: to, options: [], error: &coordinationError) { readURL, writeURL in
            do {
                try FileManager.default.moveItem(at: readURL, to: writeURL)
            } catch {
                moveError = error
            }
        }
        if let coordinationError { throw VaultError.coordination(coordinationError.localizedDescription) }
        if let moveError { throw moveError }
    }
}
