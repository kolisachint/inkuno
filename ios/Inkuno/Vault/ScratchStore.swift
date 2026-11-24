import Foundation
import InkunoCore

/// Scratch notes: raw thoughts kept outside the vault, in `Application Support/Scratch` of the app
/// container. Mirrors the desktop (`platform/scratch.rs`): named by the moment they were made
/// (`scratchName`), listed newest first, titled by their first line (`scratchTitle`), a blank one is
/// dropped when it is left, and a trashed one goes to `.trash` inside the folder, never destroyed.
/// A note is written to the vault only when the person keeps it there (not in v1).
///
/// The folder is a parameter so the tests use a temporary one. The app uses `ScratchStore.directory`.
enum ScratchStore {
    /// The scratch folder of the app: `Application Support/Scratch`, created on first use.
    static var directory: URL {
        let support = (try? FileManager.default.url(
            for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true
        )) ?? FileManager.default.temporaryDirectory
        return support.appendingPathComponent("Scratch", isDirectory: true)
    }

    /// Whether `url` is a scratch note of `folder`.
    static func holds(_ url: URL, in folder: URL = directory) -> Bool {
        url.deletingLastPathComponent().standardizedFileURL == folder.standardizedFileURL
    }

    /// A new scratch note, created empty, named after `now`. Taken names get a number (`stem 2`), so
    /// nothing is overwritten.
    static func create(in folder: URL = directory, now: Date = .now, calendar: Calendar = .current) throws -> URL {
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let stem = scratchName(
            today: CoreDates.ymd(now, calendar: calendar),
            now: CoreDates.hms(now, calendar: calendar)
        )
        var url = folder.appendingPathComponent(stem).appendingPathExtension("md")
        var number = 2
        while FileManager.default.fileExists(atPath: url.path) {
            url = folder.appendingPathComponent("\(stem) \(number)").appendingPathExtension("md")
            number += 1
        }
        try Data().write(to: url, options: .withoutOverwriting)
        return url
    }

    /// The scratch notes of `folder`, newest first.
    static func list(in folder: URL = directory) -> [URL] {
        let names = (try? FileManager.default.contentsOfDirectory(atPath: folder.path)) ?? []
        return names
            .filter { $0.hasSuffix(".md") && !$0.hasPrefix(".") }
            .map { folder.appendingPathComponent($0) }
            .sorted { newer($0.deletingPathExtension().lastPathComponent, $1.deletingPathExtension().lastPathComponent) }
    }

    /// The title of a scratch note: its first non-blank line (`scratchTitle`), or "New note" when blank or
    /// unreadable.
    static func title(of url: URL) -> String {
        let text = (try? String(contentsOf: url, encoding: .utf8)) ?? ""
        return scratchTitle(text: text)
    }

    /// Removes the scratch note when its text is blank. Returns whether it was removed. Anything written
    /// is kept: only a blank file goes.
    @discardableResult
    static func discardIfBlank(_ url: URL) -> Bool {
        guard let text = try? String(contentsOf: url, encoding: .utf8),
              text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        else { return false }
        return (try? FileManager.default.removeItem(at: url)) != nil
    }

    /// Removes the blank scratch notes left by a crash before they were left (`clean_blank`).
    static func cleanBlank(in folder: URL = directory) {
        for url in list(in: folder) {
            discardIfBlank(url)
        }
    }

    /// Moves a scratch note into `folder/.trash` under its name, numbered when taken (core `trashName`).
    /// Returns the new URL.
    @discardableResult
    static func trash(_ url: URL, in folder: URL = directory) throws -> URL {
        let bin = folder.appendingPathComponent(scratchTrashFolder(), isDirectory: true)
        try FileManager.default.createDirectory(at: bin, withIntermediateDirectories: true)
        let taken = (try? FileManager.default.contentsOfDirectory(atPath: bin.path)) ?? []
        let free = bin.appendingPathComponent(trashName(name: url.lastPathComponent, taken: taken))
        try FileManager.default.moveItem(at: url, to: free)
        return free
    }

    /// Newest first: by moment, then by collision number (`scratch::newest_first`). Names that are not scratch
    /// names sort last, by name.
    static func newer(_ a: String, _ b: String) -> Bool {
        let (keyA, numberA) = parts(a)
        let (keyB, numberB) = parts(b)
        if keyA != keyB { return keyA > keyB }
        if numberA != numberB { return numberA > numberB }
        return a > b
    }

    /// Splits a scratch name into its moment and its collision number: `2026-11-14-103200 2` gives
    /// (`2026-11-14-103200`, 2). A name that is not a scratch name gives ("", 0).
    private static func parts(_ stem: String) -> (String, Int) {
        let base: String
        let number: Int
        if let space = stem.firstIndex(of: " ") {
            base = String(stem[..<space])
            guard let parsed = Int(stem[stem.index(after: space)...]) else { return ("", 0) }
            number = parsed
        } else {
            base = stem
            number = 1
        }
        let fields = base.split(separator: "-", omittingEmptySubsequences: false)
        let digits = [4, 2, 2, 6]
        guard fields.count == 4, zip(fields, digits).allSatisfy({ field, length in
            field.count == length && field.allSatisfy(\.isASCII) && field.allSatisfy(\.isNumber)
        }) else { return ("", 0) }
        return (base, number)
    }
}
