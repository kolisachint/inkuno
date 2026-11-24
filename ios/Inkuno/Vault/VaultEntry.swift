import Foundation

/// One row of the notes folder: a subfolder, a note, or a note that iCloud has not downloaded yet.
struct VaultEntry: Identifiable, Hashable, Sendable {
    enum Kind: Sendable {
        case folder
        case note
        /// iCloud placeholder (`.Name.md.icloud`, or a file whose download status is not downloaded).
        /// `url` is the real file name, which does not exist on disk until the download finishes.
        case cloudPlaceholder
    }

    /// The real URL: the folder itself, or the `.md` file (even for a placeholder).
    let url: URL
    let kind: Kind
    let modified: Date?

    var id: URL { url }

    /// Folder name, or the note name without `.md`.
    var title: String {
        kind == .folder ? url.lastPathComponent : url.deletingPathExtension().lastPathComponent
    }

    var isNote: Bool { kind != .folder }
}
