import Foundation
import InkunoCore
import Observation

extension Notification.Name {
    /// Posted with no object after a note or folder moves to or from the vault's trash, so open folder lists reload.
    static let inkunoVaultChanged = Notification.Name("inkunoVaultChanged")
}

// MARK: - Vault operations: Trash, rename/move/duplicate, versions, pending review
//
// Mirrors the desktop's on-disk behavior (nothing is destroyed or overwritten). The paths, names,
// version files and pending lists are the Rust core's (InkunoCore); this extension does the file IO:
// - Trash is the vault's `.trash` folder, its history and pending lists wait in `.inkuno/trash` and
//   `.inkuno/pending-trash`, and the folder it came from is kept in `.inkuno/trash-origin`, with the
//   item's file number, so a record left for a gone item is not read for a later one.
// - A version lives in `.inkuno/revisions/<note-rel-path>/NNNN.md`.
// - Pending changes live in `.inkuno/pending/<note-rel-path>.json`.

extension VaultStore {
    // MARK: Layout

    /// The vault's trash folder (created on first use).
    var trashDir: URL? {
        vaultURL?.appendingPathComponent(vaultTrashFolder(), isDirectory: true)
    }

    /// The pending-changes list beside a note: `.inkuno/pending/<note path>.json` (core `sidecarPath`).
    func pendingFile(for note: URL) -> URL? {
        guard let root = vaultURL, let rel = relativePath(of: note), let path = sidecarPath(notePath: rel) else { return nil }
        return root.appendingPathComponent(path)
    }

    /// The folder of a note's versions: `.inkuno/revisions/<note path>` (core `revisionFolder`).
    func revisionDir(for note: URL) -> URL? {
        guard let root = vaultURL, let rel = relativePath(of: note), let path = revisionFolder(notePath: rel) else { return nil }
        return root.appendingPathComponent(path, isDirectory: true)
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

    /// The folder a trashed item shows under its title, from its origin record: the folder it came from, or
    /// "Notes" for the vault root, as the tree labels it.
    func trashOrigin(of url: URL) -> String {
        guard let root = vaultURL else { return "Notes" }
        let folder = Self.restoreFolder(
            recordText: originRecordText(trashedName: url.lastPathComponent, root: root),
            fileNumber: Self.fileNumber(of: url)
        )
        return folder.isEmpty ? "Notes" : folder
    }

    /// The file number of the item at `url` (its inode on the device). A rename keeps it. Nil when it cannot be read.
    static func fileNumber(of url: URL) -> UInt64? {
        guard let attributes = try? FileManager.default.attributesOfItem(atPath: url.path) else { return nil }
        return (attributes[.systemFileNumber] as? NSNumber)?.uint64Value
    }

    /// The vault-relative folder of `relativePath` (`ideas/plan.md` → `ideas`, `plan.md` → `""`): where a
    /// trashed item goes back to.
    static func originFolder(relativePath: String) -> String {
        (relativePath as NSString).deletingLastPathComponent
    }

    /// The folder a trashed item goes back to, read from its origin record. The record is two lines: the folder
    /// (`trashOriginFolder`) and the item's file number. The folder counts only when the second line is
    /// `fileNumber`, the item's current number. Otherwise, or when the record has no second line, the vault root (`""`).
    static func restoreFolder(recordText: String?, fileNumber: UInt64?) -> String {
        guard let recordText, let fileNumber else { return "" }
        let lines = recordText.split(separator: "\n", omittingEmptySubsequences: false)
        guard lines.count >= 2, lines[1].trimmingCharacters(in: .whitespaces) == String(fileNumber) else { return "" }
        return trashOriginFolder(text: String(lines[0])) ?? ""
    }

    /// The text of a trashed item's origin record (`.inkuno/trash-origin/<name>.txt`), nil when there is none.
    private func originRecordText(trashedName: String, root: URL) -> String? {
        guard let path = trashOriginPath(trashedName: trashedName),
              let data = try? Data(contentsOf: root.appendingPathComponent(path)) else { return nil }
        return String(decoding: data, as: UTF8.self)
    }

    /// Writes the origin record of a trashed item: the folder it came from, vault-relative (`""` for the root), then
    /// the item's file number on the second line. A record already under that name is stale: a new item got this
    /// trashed name, so it is replaced, not moved aside.
    private func writeOriginRecord(_ folder: String, fileNumber: UInt64, trashedName: String, root: URL) throws {
        guard let path = trashOriginPath(trashedName: trashedName) else { return }
        let file = root.appendingPathComponent(path)
        try FileManager.default.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        let text = "\(folder)\n\(fileNumber)\n"
        try coordinateWrite(file) { try Data(text.utf8).write(to: $0, options: .atomic) }
    }

    /// The folder a trashed item goes back to: its origin folder when the record names one that is not a file
    /// (created when it is gone, nothing is destroyed), else the vault root.
    private func restoreDestination(of url: URL, root: URL) -> URL {
        let origin = Self.restoreFolder(
            recordText: originRecordText(trashedName: url.lastPathComponent, root: root),
            fileNumber: Self.fileNumber(of: url)
        )
        guard !origin.isEmpty else { return root }
        let folder = root.appendingPathComponent(origin, isDirectory: true)
        var isDirectory: ObjCBool = false
        if FileManager.default.fileExists(atPath: folder.path, isDirectory: &isDirectory) {
            return isDirectory.boolValue ? folder : root
        }
        guard (try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)) != nil else {
            return root
        }
        return folder
    }

    /// Moves the note or folder to the vault's trash under a free name: `Name.md`, then
    /// `2 Name.md`, `3 Name.md`… (`platform/vault.rs` `trash`). Nothing is destroyed. The note's
    /// history and pending list move to their bins with it (`revisions.rs` `trash_along`,
    /// `sidecar.rs` `trash_along`), moved aside under a free name when a stale entry sits there.
    @discardableResult
    func trashItem(at url: URL) throws -> URL {
        guard let root = vaultURL, let bin = trashDir else { throw VaultError.noVault }
        try FileManager.default.createDirectory(at: bin, withIntermediateDirectories: true)
        let free = bin.appendingPathComponent(trashName(name: url.lastPathComponent, taken: Self.names(in: bin)))
        let isFolder = Self.isFolder(url)
        let origin = relativePath(of: url).map(Self.originFolder(relativePath:)) ?? ""
        try coordinateMove(from: url, to: free)
        moveAsideHistoryAndPending(from: url, trashedName: free.lastPathComponent, root: root)
        // The folder it came from is kept beside the trash, with the item's file number, so Restore can put it
        // back. Failing to write it only means the item restores to the root.
        if let number = Self.fileNumber(of: free) {
            try? writeOriginRecord(origin, fileNumber: number, trashedName: free.lastPathComponent, root: root)
        }
        indexRemoved(at: url, isFolder: isFolder)
        NotificationCenter.default.post(name: .inkunoVaultChanged, object: nil)
        return free
    }

    /// Takes a trashed item back to the folder it came from (its origin record; the root when the record is
    /// missing or invalid), under its name, numbered when taken (`platform/vault.rs` `restore`): `Plan.md`, then
    /// `Plan 2.md`. Nothing is overwritten. Its history and pending list come back with it when no history of
    /// that name is there (`revisions.rs` `restore_along`, `sidecar.rs` `restore_along`). The origin record is
    /// removed once the item is back.
    @discardableResult
    func restoreFromTrash(at url: URL) throws -> URL {
        guard let root = vaultURL else { throw VaultError.noVault }
        let folder = restoreDestination(of: url, root: root)
        let stem = url.deletingPathExtension().lastPathComponent
        let ext = url.pathExtension.isEmpty ? "" : ".\(url.pathExtension)"
        let free = (1...).lazy.map { n in
            n == 1 ? folder.appendingPathComponent("\(stem)\(ext)")
                   : folder.appendingPathComponent("\(stem) \(n)\(ext)")
        }.first { !FileManager.default.fileExists(atPath: $0.path) }!
        let isFolder = Self.isFolder(url)
        try coordinateMove(from: url, to: free)
        restoreHistoryAndPending(trashedName: url.lastPathComponent, to: free, root: root)
        if let path = trashOriginPath(trashedName: url.lastPathComponent) {
            try? FileManager.default.removeItem(at: root.appendingPathComponent(path))
        }
        if isFolder {
            indexRemoved(at: url, isFolder: true)
        } else {
            indexNote(at: free, text: (try? readNote(at: free)) ?? "")
        }
        NotificationCenter.default.post(name: .inkunoVaultChanged, object: nil)
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
        let isFolder = Self.isFolder(url)
        try coordinateMove(from: url, to: target)
        moveAlongHistoryAndPending(from: url, to: target)
        indexMoved(from: url, to: target, isFolder: isFolder)
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
        let isFolder = Self.isFolder(url)
        try coordinateMove(from: url, to: target)
        moveAlongHistoryAndPending(from: url, to: target)
        indexMoved(from: url, to: target, isFolder: isFolder)
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
        indexNote(at: free, text: String(decoding: data, as: UTF8.self))
        return free
    }

    /// `[Title](relative/path.md)`: a relative link, so it also works on GitHub and the desktop.
    func copyLink(for url: URL) -> String? {
        guard let rel = relativePath(of: url) else { return nil }
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
        let numbers = names.compactMap { revisionNumberOf(name: $0) }.sorted()
        return numbers.map { number in
            let file = dir.appendingPathComponent(revisionFileName(number: number))
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
        do {
            return try revisionParseHeader(file: text)
        } catch {
            throw VaultError.coordination("unreadable version header")
        }
    }

    func readRevisionText(number: UInt32, for note: URL) throws -> String {
        guard let dir = revisionDir(for: note) else { throw VaultError.noVault }
        let file = dir.appendingPathComponent(revisionFileName(number: number))
        let data = try coordinateRead(file) { try Data(contentsOf: $0) }
        guard let text = String(data: data, encoding: .utf8) else {
            throw VaultError.coordination("unreadable version")
        }
        do {
            return try revisionParse(file: text).text
        } catch {
            throw VaultError.coordination("unreadable version")
        }
    }

    /// Saves the note's text as a new Checkpoint revision (`platform/revisions.rs` `save`).
    /// Refused (no file) when the text is the latest revision's already.
    /// The author is the Settings name, cleaned; "unknown" when it is empty. Dates are UTC RFC 3339.
    @discardableResult
    func saveVersion(of note: URL, memo: String? = nil) throws -> UInt32? {
        guard let dir = revisionDir(for: note) else { throw VaultError.noVault }
        let text = try readNote(at: note)
        let existing = (try? FileManager.default.contentsOfDirectory(atPath: dir.path)) ?? []
        let numbers = existing.compactMap { revisionNumberOf(name: $0) }.sorted()
        if let latest = numbers.last,
           (try? readRevisionText(number: latest, for: note)) == text {
            return nil
        }
        let number = revisionNextNumber(saved: numbers)
        let rev = revisionRender(revision: Revision(
            number: number, parent: numbers.last,
            author: AuthorName.resolved(), date: revisionDate(unix: Self.nowUnix()),
            kind: .checkpoint, note: memo, text: text
        ))
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let file = dir.appendingPathComponent(revisionFileName(number: number))
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
        let numbers = existing.compactMap { revisionNumberOf(name: $0) }.sorted()
        if let latest = numbers.last,
           (try? readRevisionText(number: latest, for: note)) == text {
            return
        }
        let number = revisionNextNumber(saved: numbers)
        let rev = revisionRender(revision: Revision(
            number: number, parent: numbers.last,
            author: AuthorName.resolved(), date: revisionDate(unix: Self.nowUnix()),
            kind: kind, note: memo, text: text
        ))
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let file = dir.appendingPathComponent(revisionFileName(number: number))
        guard !FileManager.default.fileExists(atPath: file.path) else {
            try saveVersionRaw(of: note, text: text, kind: kind, memo: memo)
            return
        }
        try coordinateWrite(file) { try Data(rev.utf8).write(to: $0, options: .withoutOverwriting) }
    }

    /// The current time in Unix seconds, for the core's date writer (`revision_date`).
    static func nowUnix() -> Int64 {
        Int64(Date().timeIntervalSince1970)
    }

    // MARK: Pending changes

    /// The pending changes beside the note (core `sidecarParse`). A missing or unreadable list is empty.
    func pendingEntries(for note: URL) -> [PendingEntry] {
        guard let file = pendingFile(for: note),
              let data = try? Data(contentsOf: file) else { return [] }
        return sidecarParse(json: String(decoding: data, as: UTF8.self))
    }

    func pendingCount(for note: URL) -> Int {
        pendingEntries(for: note).count
    }

    private func writePendingEntries(_ entries: [PendingEntry], for note: URL) throws {
        guard let file = pendingFile(for: note) else { throw VaultError.noVault }
        if entries.isEmpty {
            if FileManager.default.fileExists(atPath: file.path) {
                try FileManager.default.removeItem(at: file)
            }
            return
        }
        try FileManager.default.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        let json = sidecarRender(entries: entries)
        try coordinateWrite(file) { try Data(json.utf8).write(to: $0, options: .atomic) }
    }

    /// The text with one entry accepted: the entry is written over the text (core `composePending`), then
    /// its one area is accepted (core `acceptChange`). Nil when its text is not in `text`, so the caller
    /// keeps the entry.
    static func textAccepting(_ entry: PendingEntry, in text: String) -> String? {
        guard placedPending(base: text, entries: [entry]) == [true] else { return nil }
        return acceptChange(text: composePending(base: text, entries: [entry]), index: 0)
    }

    /// Accepts one pending change into the note file. Returns false when the change's text is
    /// gone from the note; the entry is kept then.
    @discardableResult
    func acceptPendingEntry(id: UInt32, for note: URL) throws -> Bool {
        var entries = pendingEntries(for: note)
        guard let index = entries.firstIndex(where: { $0.id == id }) else { return false }
        let text = try readNote(at: note)
        guard let next = Self.textAccepting(entries[index], in: text) else { return false }
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

    /// Accept all / Reject all. Accept writes every change whose text is still there (core
    /// `composePending`, then `acceptAllChanges`) and keeps the rest; reject drops them all.
    func resolveAllPending(for note: URL, accept: Bool) throws {
        if !accept {
            try writePendingEntries([], for: note)
            return
        }
        let entries = pendingEntries(for: note)
        let text = try readNote(at: note)
        let placed = placedPending(base: text, entries: entries)
        let accepted = acceptAllChanges(text: composePending(base: text, entries: entries))
        if accepted != text { try writeNote(accepted, to: note) }
        let kept = zip(entries, placed).filter { !$0.1 }.map(\.0)
        try writePendingEntries(kept, for: note)
    }

    // MARK: History follows the note

    /// Rename/move: the revision folder and the pending list move with the note
    /// (`revisions.rs` `move_along`, `sidecar.rs` `move_along`).
    private func moveAlongHistoryAndPending(from: URL, to: URL) {
        guard let root = vaultURL else { return }
        if let src = revisionDir(for: from),
           let dst = revisionDir(for: to),
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

    /// Trash: history and pending lists wait in their bins under the trashed name (core
    /// `vaultRevisionBin`, `trashedSidecarPath`). A stale bin entry is moved aside, never deleted.
    private func moveAsideHistoryAndPending(from: URL, trashedName: String, root: URL) {
        if let src = revisionDir(for: from), FileManager.default.fileExists(atPath: src.path) {
            moveIntoBin(src, bin: root.appendingPathComponent(vaultRevisionBin(), isDirectory: true), name: trashedName)
        }
        if let src = pendingFile(for: from), FileManager.default.fileExists(atPath: src.path),
           let slot = trashedSidecarPath(trashedName: trashedName) {
            let target = root.appendingPathComponent(slot)
            moveIntoBin(src, bin: target.deletingLastPathComponent(), name: target.lastPathComponent)
        }
    }

    /// Restore: the bin entries of that exact name come back with the note
    /// (`revisions.rs` `restore_along`, `sidecar.rs` `restore_along`).
    private func restoreHistoryAndPending(trashedName: String, to: URL, root: URL) {
        let revSrc = root.appendingPathComponent(vaultRevisionBin(), isDirectory: true).appendingPathComponent(trashedName)
        if FileManager.default.fileExists(atPath: revSrc.path),
           let dst = revisionDir(for: to),
           !FileManager.default.fileExists(atPath: dst.path) {
            try? FileManager.default.createDirectory(at: dst.deletingLastPathComponent(), withIntermediateDirectories: true)
            try? FileManager.default.moveItem(at: revSrc, to: dst)
        }
        if let slot = trashedSidecarPath(trashedName: trashedName) {
            let pendSrc = root.appendingPathComponent(slot)
            if FileManager.default.fileExists(atPath: pendSrc.path),
               let dst = pendingFile(for: to), !FileManager.default.fileExists(atPath: dst.path) {
                try? FileManager.default.createDirectory(at: dst.deletingLastPathComponent(), withIntermediateDirectories: true)
                try? FileManager.default.moveItem(at: pendSrc, to: dst)
            }
        }
    }

    /// Moves `src` into `bin` as `name`. An entry already there moves aside under a free name first, so
    /// nothing is replaced.
    private func moveIntoBin(_ src: URL, bin: URL, name: String) {
        try? FileManager.default.createDirectory(at: bin, withIntermediateDirectories: true)
        let slot = bin.appendingPathComponent(name)
        if FileManager.default.fileExists(atPath: slot.path) {
            let aside = bin.appendingPathComponent(trashName(name: name, taken: Self.names(in: bin)))
            try? FileManager.default.moveItem(at: slot, to: aside)
        }
        try? FileManager.default.moveItem(at: src, to: slot)
    }

    /// Whether `url` is a folder (the URL of a listed folder or a file that may have gone).
    static func isFolder(_ url: URL) -> Bool {
        url.hasDirectoryPath || (try? url.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true
    }

    /// Keeps the index current after `url` moved to `target` (a folder's notes are indexed again on use).
    func indexMoved(from url: URL, to target: URL, isFolder: Bool) {
        indexRemoved(at: url, isFolder: isFolder)
        if !isFolder { indexNote(at: target, text: (try? readNote(at: target)) ?? "") }
    }

    /// The names of the entries of a folder (empty when it cannot be read).
    static func names(in folder: URL) -> [String] {
        (try? FileManager.default.contentsOfDirectory(atPath: folder.path)) ?? []
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
