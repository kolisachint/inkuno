import Foundation
import InkunoCore
import Observation

enum VaultError: LocalizedError {
    case noVault
    /// The note is in iCloud and not on disk yet; a download has been requested.
    case notDownloaded(URL)
    /// A file already exists at this URL, so a create is refused. Nothing is overwritten.
    case nameTaken(URL)
    case coordination(String)
    case bookmark(String)

    var errorDescription: String? {
        switch self {
        case .noVault:
            return "No notes folder is chosen yet."
        case .notDownloaded(let url):
            return "\(url.lastPathComponent) is still in iCloud. The download has started; try again in a moment."
        case .nameTaken(let url):
            return "\(url.lastPathComponent) already exists."
        case .coordination(let reason):
            return "The file could not be accessed: \(reason)"
        case .bookmark(let reason):
            return "The notes folder could not be remembered: \(reason)"
        }
    }
}

/// The user's notes folder: a folder in the Files app, kept with a security-scoped bookmark.
/// The folder is the same one the desktop app uses.
///
/// The desktop's rules (names, trash, versions, pending changes, search, recall) are the Rust core's,
/// through InkunoCore. File access stays here, with file coordination, on the main actor: fine for a few
/// hundred notes, not for large vaults.
@Observable
@MainActor
final class VaultStore {
    static let bookmarkKey = "inkuno.vaultBookmark"

    /// The chosen folder, or nil when onboarding has not finished.
    private(set) var vaultURL: URL?
    /// The last problem worth showing the user (for example a bookmark that no longer resolves).
    private(set) var lastError: String?
    /// The vault's notes as the core indexes them: names, links, tags and bodies. Backlinks, recall and
    /// the palette's search read it. Built by `rebuildIndex`, kept current by this store's writes.
    private(set) var index: VaultIndex?

    private let defaults: UserDefaults
    /// The folder whose security-scoped access this store started and must stop.
    private var scopedURL: URL?

    /// The app's store: restores the folder from the saved bookmark.
    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        #if DEBUG
        // Debug builds only: `-InkunoVaultPath <absolute path>` opens that folder, with no picker or bookmark.
        if let path = defaults.string(forKey: Self.debugVaultPathKey) {
            vaultURL = URL(fileURLWithPath: path, isDirectory: true)
            return
        }
        #endif
        restoreBookmark()
    }

    #if DEBUG
    static let debugVaultPathKey = "InkunoVaultPath"
    #endif

    /// A store on a known folder with no bookmark and no security scope. Used by tests.
    init(vaultURL: URL, defaults: UserDefaults = .standard) {
        self.defaults = defaults
        self.vaultURL = vaultURL
    }

    var hasVault: Bool { vaultURL != nil }

    /// The vault-relative path of a file inside the vault (`ideas/plan.md`); nil outside it.
    func relativePath(of url: URL) -> String? {
        guard let root = vaultURL else { return nil }
        let rootPath = root.standardized.path
        let path = url.standardized.path
        guard path.hasPrefix(rootPath + "/") else { return nil }
        return String(path.dropFirst(rootPath.count + 1))
    }

    var displayName: String {
        vaultURL?.lastPathComponent ?? "Not chosen"
    }

    // MARK: Folder choice and bookmark

    /// Remembers a folder the user picked in the Files app. Replaces any earlier choice.
    func chooseVault(_ url: URL) throws {
        stopScopedAccess()
        let granted = url.startAccessingSecurityScopedResource()
        do {
            let bookmark = try url.bookmarkData()
            defaults.set(bookmark, forKey: Self.bookmarkKey)
        } catch {
            if granted { url.stopAccessingSecurityScopedResource() }
            throw VaultError.bookmark(error.localizedDescription)
        }
        scopedURL = granted ? url : nil
        vaultURL = url
        index = nil
        lastError = nil
    }

    private func restoreBookmark() {
        guard let bookmark = defaults.data(forKey: Self.bookmarkKey) else { return }
        var stale = false
        do {
            let url = try URL(resolvingBookmarkData: bookmark, options: [], relativeTo: nil, bookmarkDataIsStale: &stale)
            let granted = url.startAccessingSecurityScopedResource()
            scopedURL = granted ? url : nil
            vaultURL = url
            index = nil
            if stale {
                // The bookmark still resolves but is out of date: save a fresh one while access is granted.
                if let fresh = try? url.bookmarkData() {
                    defaults.set(fresh, forKey: Self.bookmarkKey)
                } else {
                    lastError = "The notes folder bookmark could not be refreshed. If notes are missing, choose the folder again in Settings."
                }
            }
        } catch {
            // Unresolvable (folder deleted, or the bookmark came from another device): forget it and ask again.
            defaults.removeObject(forKey: Self.bookmarkKey)
            vaultURL = nil
            lastError = "The notes folder could not be opened. Choose it again."
        }
    }

    private func stopScopedAccess() {
        scopedURL?.stopAccessingSecurityScopedResource()
        scopedURL = nil
    }

    // MARK: Listing

    /// Folders first, then notes. Hidden entries (`.trash`, `.DS_Store`) are skipped; other files are ignored.
    func entries(in folder: URL) throws -> [VaultEntry] {
        try coordinateRead(folder) { try Self.scan($0) }
    }

    /// Every note under the vault, found recursively. Used by the palette and the recall cards.
    func allNotes() throws -> [VaultEntry] {
        guard let root = vaultURL else { throw VaultError.noVault }
        var notes: [VaultEntry] = []
        var pending = [root]
        while let folder = pending.popLast() {
            for entry in try entries(in: folder) {
                switch entry.kind {
                case .folder: pending.append(entry.url)
                case .note, .cloudPlaceholder: notes.append(entry)
                }
            }
        }
        return notes
    }

    // MARK: Index, recall and search

    /// The index of the vault, built on first use and then kept current by this store's writes.
    func searchIndex() throws -> VaultIndex {
        if let index { return index }
        return try rebuildIndex()
    }

    /// Reads every note of the vault and builds the index from them. A note that cannot be read is
    /// indexed with an empty body. Call it when the files may have changed outside this store.
    @discardableResult
    func rebuildIndex() throws -> VaultIndex {
        let inputs = try allNotes().compactMap { entry -> NoteInput? in
            guard let path = relativePath(of: entry.url) else { return nil }
            return NoteInput(
                path: path,
                body: (try? readNote(at: entry.url)) ?? "",
                mtimeUnix: Int64((entry.modified ?? .distantPast).timeIntervalSince1970)
            )
        }
        let built = VaultIndex(notes: inputs)
        index = built
        return built
    }

    /// The URL of a vault-relative note path (`ideas/plan.md`).
    func noteURL(path: String) -> URL? {
        vaultURL?.appendingPathComponent(path)
    }

    /// Notes to reread today, at most three (core `recall`): the daily notes of a week, a month and a year
    /// ago, then an old note. `now` is the time of the day.
    func recalledNotes(now: Date = .now) throws -> [RecalledNote] {
        try searchIndex().recall(today: CoreDates.ymd(now), nowUnix: Int64(now.timeIntervalSince1970))
    }

    /// The notes that link to `note` by name (core `backlinks`), for the Backlinks panel.
    func backlinks(to note: URL) throws -> [(url: URL, title: String)] {
        guard let path = relativePath(of: note) else { return [] }
        return try searchIndex().backlinks(path: path).compactMap { ref in
            noteURL(path: ref.path).map { (url: $0, title: ref.name) }
        }
    }

    /// Keeps the index current after a note's text is written: the note is put in again under its path
    /// (core `update`). Nothing happens before the index is built.
    func indexNote(at url: URL, text: String) {
        guard let index, let path = relativePath(of: url) else { return }
        index.update(note: NoteInput(path: path, body: text, mtimeUnix: Self.nowUnix()))
    }

    /// Keeps the index current after `url` left its place: a note is taken out of the index; a folder's
    /// notes change paths, so the index is dropped and built again on its next use.
    func indexRemoved(at url: URL, isFolder: Bool) {
        guard index != nil else { return }
        if isFolder {
            index = nil
        } else if let path = relativePath(of: url) {
            index?.remove(path: path)
        }
    }

    // MARK: Reading and writing

    /// Reads a note. A placeholder whose file is not on disk yet starts its iCloud download and throws `notDownloaded`.
    func readNote(at url: URL) throws -> String {
        if !FileManager.default.fileExists(atPath: url.path) {
            if FileManager.default.fileExists(atPath: Self.placeholderURL(for: url).path) {
                startDownload(of: url)
                throw VaultError.notDownloaded(url)
            }
        }
        let data = try coordinateRead(url) { try Data(contentsOf: $0) }
        return String(decoding: data, as: UTF8.self)
    }

    /// Saves a note atomically (temporary file, then rename), through file coordination.
    func writeNote(_ text: String, to url: URL) throws {
        let data = Data(text.utf8)
        try coordinateWrite(url) { try data.write(to: $0, options: .atomic) }
        indexNote(at: url, text: text)
    }

    /// Creates a note. Refuses when the name is taken, so nothing is overwritten.
    @discardableResult
    func createNote(at url: URL, text: String = "") throws -> URL {
        guard !FileManager.default.fileExists(atPath: url.path),
              !FileManager.default.fileExists(atPath: Self.placeholderURL(for: url).path)
        else { throw VaultError.nameTaken(url) }
        let data = Data(text.utf8)
        try coordinateWrite(url) { try data.write(to: $0, options: .withoutOverwriting) }
        indexNote(at: url, text: text)
        return url
    }

    /// `Untitled.md`, `Untitled 2.md`, … in `folder`: the first name not taken.
    func newNoteURL(in folder: URL) -> URL {
        var candidate = folder.appendingPathComponent("Untitled.md")
        var index = 2
        while FileManager.default.fileExists(atPath: candidate.path)
            || FileManager.default.fileExists(atPath: Self.placeholderURL(for: candidate).path) {
            candidate = folder.appendingPathComponent("Untitled \(index).md")
            index += 1
        }
        return candidate
    }

    /// Opens today's daily note in the vault root, creating it empty if needed.
    func openToday(now: Date = .now, calendar: Calendar = .current) throws -> URL {
        guard let root = vaultURL else { throw VaultError.noVault }
        let url = root.appendingPathComponent(Self.dailyNoteFileName(for: now, calendar: calendar))
        return try createIfMissing(at: url)
    }

    /// The daily note's file name: `2026-11-24.md`, named by the core (`daily_note_name`).
    static func dailyNoteFileName(for date: Date, calendar: Calendar = .current) -> String {
        dailyNoteName(today: CoreDates.ymd(date, calendar: calendar)) + ".md"
    }

    /// The iCloud placeholder that stands in for `url` while it is not downloaded: `folder/.Name.md.icloud`.
    static func placeholderURL(for url: URL) -> URL {
        url.deletingLastPathComponent().appendingPathComponent("." + url.lastPathComponent + ".icloud")
    }

    /// Creates an empty note at `url` unless a file is already there. Returns `url` either way.
    @discardableResult
    func createIfMissing(at url: URL) throws -> URL {
        if FileManager.default.fileExists(atPath: url.path) { return url }
        return try createNote(at: url)
    }

    /// Asks iCloud to download a placeholder's file. The caller retries reading afterwards.
    func startDownload(of url: URL) {
        do {
            // ponytail: the real file name is passed, not the placeholder; this matches what iCloud expects on iOS.
            try FileManager.default.startDownloadingUbiquitousItem(at: url)
        } catch {
            lastError = "The iCloud download could not start: \(error.localizedDescription)"
        }
    }

    // MARK: Coordination

    func coordinateRead<T>(_ url: URL, _ body: (URL) throws -> T) throws -> T {
        var coordinationError: NSError?
        var output: Result<T, Error>?
        NSFileCoordinator().coordinate(readingItemAt: url, options: [], error: &coordinationError) { coordinated in
            output = Result { try body(coordinated) }
        }
        if let coordinationError { throw VaultError.coordination(coordinationError.localizedDescription) }
        guard let output else { throw VaultError.coordination("no result") }
        return try output.get()
    }

    func coordinateWrite<T>(_ url: URL, _ body: (URL) throws -> T) throws -> T {
        var coordinationError: NSError?
        var output: Result<T, Error>?
        NSFileCoordinator().coordinate(writingItemAt: url, options: [], error: &coordinationError) { coordinated in
            output = Result { try body(coordinated) }
        }
        if let coordinationError { throw VaultError.coordination(coordinationError.localizedDescription) }
        guard let output else { throw VaultError.coordination("no result") }
        return try output.get()
    }

    // MARK: Scanning

    private static let scanKeys: [URLResourceKey] = [
        .isDirectoryKey,
        .contentModificationDateKey,
        .isUbiquitousItemKey,
        .ubiquitousItemDownloadingStatusKey,
    ]

    private static func scan(_ folder: URL) throws -> [VaultEntry] {
        let items = try FileManager.default.contentsOfDirectory(
            at: folder, includingPropertiesForKeys: scanKeys, options: []
        )
        var folders: [VaultEntry] = []
        var notes: [VaultEntry] = []
        for item in items {
            let name = item.lastPathComponent
            if let realName = placeholderRealName(name) {
                // An iCloud placeholder for a note: `.Name.md.icloud` stands for `Name.md`.
                guard realName.lowercased().hasSuffix(".md") else { continue }
                let real = folder.appendingPathComponent(realName)
                let modified = try? item.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate
                notes.append(VaultEntry(url: real, kind: .cloudPlaceholder, modified: modified))
                continue
            }
            if name.hasPrefix(".") { continue }
            let values = try item.resourceValues(forKeys: Set(scanKeys))
            if values.isDirectory == true {
                folders.append(VaultEntry(url: item, kind: .folder, modified: values.contentModificationDate))
            } else if item.pathExtension.lowercased() == "md" {
                let notDownloaded = values.isUbiquitousItem == true
                    && values.ubiquitousItemDownloadingStatus == .notDownloaded
                notes.append(VaultEntry(
                    url: item,
                    kind: notDownloaded ? .cloudPlaceholder : .note,
                    modified: values.contentModificationDate
                ))
            }
        }
        folders.sort { $0.title.localizedStandardCompare($1.title) == .orderedAscending }
        notes.sort { $0.title.localizedStandardCompare($1.title) == .orderedAscending }
        return folders + notes
    }

    /// `.Name.md.icloud` → `Name.md`; any other name → nil.
    private static func placeholderRealName(_ name: String) -> String? {
        guard name.hasPrefix("."), name.hasSuffix(".icloud"), name.count > ".icloud".count + 1 else { return nil }
        return String(name.dropFirst().dropLast(".icloud".count))
    }
}
