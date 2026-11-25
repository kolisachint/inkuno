import InkunoCore
import SwiftUI

/// Note search and `>` commands, as a full sheet. An empty query shows the six most recent notes
/// (by mtime). A query is searched in the vault index (core `search`: names, then text, then `#tags`);
/// ">" switches to commands. `currentNote` (with `onPanelTab`) enables the This-note commands when a
/// note is open; without it those commands are hidden.
struct PaletteSheet: View {
    @Environment(VaultStore.self) private var vault
    @Environment(\.dismiss) private var dismiss

    /// Called with the route to push; the presenter dismisses the sheet.
    let onOpen: (Route) -> Void
    var currentNote: URL? = nil
    var onPanelTab: ((PanelTab) -> Void)? = nil
    /// Applies a formatting command to the open note (Bold, Italic, Heading). Without it those commands are hidden.
    var onFormat: ((EditorFormat) -> Void)? = nil
    /// The query the search field starts with, such as `#tag` from a tapped tag.
    var initialQuery = ""

    @State private var query = ""
    @State private var notes: [VaultEntry] = []
    /// The listed notes by their vault-relative path, to pair a search hit with its row.
    @State private var entriesByPath: [String: VaultEntry] = [:]
    @State private var error: String?
    /// The pending changes of `currentNote`, read on load: the Accept/Reject all commands show only when it is not 0.
    @State private var pendingCount = 0

    private var isCommandMode: Bool {
        query.hasPrefix(">")
    }

    /// The query without the ">" prefix, trimmed.
    private var needle: String {
        (isCommandMode ? String(query.dropFirst()) : query).trimmingCharacters(in: .whitespaces)
    }

    private var recent: [VaultEntry] {
        notes.sorted { ($0.modified ?? .distantPast) > ($1.modified ?? .distantPast) }
            .prefix(6)
            .map { $0 }
    }

    /// A note found by its text, with the line that holds the word.
    private struct TextHit: Identifiable {
        var entry: VaultEntry
        var snippet: String?
        var id: URL { entry.url }
    }

    /// The index's matches for the query, in its order (names and tags first, then text).
    private var matches: [(entry: VaultEntry, kind: SearchKind, snippet: String?)] {
        guard let index = vault.index, !needle.isEmpty else { return [] }
        return index.search(query: needle, limit: UInt32(notes.count + 8)).compactMap { hit in
            entriesByPath[hit.path].map { (entry: $0, kind: hit.kind, snippet: hit.snippet?.text) }
        }
    }

    private var nameHits: [VaultEntry] {
        matches.filter { $0.kind != .byText }.map(\.entry)
    }

    private var contentHits: [TextHit] {
        matches.filter { $0.kind == .byText }.map { TextHit(entry: $0.entry, snippet: $0.snippet) }
    }

    private var showCreateRow: Bool {
        !needle.isEmpty && !notes.contains { $0.title.lowercased() == needle.lowercased() }
    }

    private var commands: [PaletteCommand] {
        PaletteCommand.all(
            for: currentNote,
            hasPanelHost: onPanelTab != nil,
            hasPending: pendingCount > 0,
            hasFormatHost: onFormat != nil
        )
            .filter { needle.isEmpty || $0.title.localizedStandardContains(needle) }
    }

    var body: some View {
        NavigationStack {
            List {
                if isCommandMode {
                    commandSection
                } else if needle.isEmpty {
                    Section {
                        ForEach(recent) { entry in
                            noteButton(entry)
                        }
                    } header: {
                        InkSectionTitle("Recent")
                    }
                } else if needle.hasPrefix("@") {
                    Section {
                        Button("Today's note") { openToday() }
                        ForEach(contentHits) { hit in
                            noteButton(hit.entry, snippet: hit.snippet)
                        }
                    } header: {
                        InkSectionTitle("Names")
                    } footer: {
                        if nameHits.isEmpty, contentHits.isEmpty {
                            Text("No matches")
                        }
                    }
                } else {
                    if !nameHits.isEmpty || showCreateRow {
                        Section {
                            ForEach(nameHits) { entry in
                                noteButton(entry)
                            }
                            if showCreateRow {
                                Button("Create «\(needle)»") { create() }
                            }
                        } header: {
                            InkSectionTitle("Names")
                        } footer: {
                            if nameHits.isEmpty, contentHits.isEmpty {
                                Text("No matches")
                            }
                        }
                    }
                    if !contentHits.isEmpty {
                        Section {
                            ForEach(contentHits) { hit in
                                noteButton(hit.entry, snippet: hit.snippet)
                            }
                        } header: {
                            InkSectionTitle("Content")
                        }
                    }
                    if nameHits.isEmpty, contentHits.isEmpty, !showCreateRow {
                        Section {
                            Text("No matches")
                                .font(InkType.body)
                                .foregroundStyle(InkColor.faint)
                        }
                    }
                }

                if let error {
                    Section {
                        Text(error)
                            .font(InkType.caption)
                            .foregroundStyle(InkColor.red)
                    }
                }
            }
            .listStyle(.insetGrouped)
            .scrollContentBackground(.hidden)
            .background(InkColor.ground)
            .searchable(text: $query, placement: .navigationBarDrawer(displayMode: .always), prompt: "Search notes, or type > for commands")
            .navigationTitle("Search")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }
                }
            }
            .task {
                query = initialQuery
                reload()
            }
        }
    }

    // MARK: Commands

    @ViewBuilder
    private var commandSection: some View {
        let groups = Dictionary(grouping: commands, by: \.group)
        ForEach(PaletteCommand.Group.allCases) { group in
            if let rows = groups[group], !rows.isEmpty {
                Section {
                    ForEach(rows) { command in
                        Button { run(command) } label: {
                            // A command is not a link: ink text, with its symbol in the icon colour, as the note rows.
                            HStack(spacing: 12) {
                                Image(systemName: command.symbol)
                                    .frame(width: InkMetrics.markerIconSize)
                                    .foregroundStyle(InkColor.icon)
                                    .accessibilityHidden(true)
                                Text(command.title)
                                    .font(InkType.rowTitle)
                                    .foregroundStyle(InkColor.ink)
                            }
                            .frame(minHeight: InkMetrics.rowMinHeight)
                        }
                    }
                } header: {
                    InkSectionTitle(group.title)
                }
            }
        }
        if commands.isEmpty {
            Section {
                Text("No matches")
                    .font(InkType.body)
                    .foregroundStyle(InkColor.faint)
            }
        }
    }

    private func noteButton(_ entry: VaultEntry, snippet: String? = nil) -> some View {
        Button {
            onOpen(.note(entry.url))
        } label: {
            NoteRow(entry: entry, snippet: snippet)
        }
    }

    // MARK: Data

    private func reload() {
        pendingCount = currentNote.map { vault.pendingCount(for: $0) } ?? 0
        do {
            notes = try vault.allNotes()
            entriesByPath = Dictionary(uniqueKeysWithValues: notes.compactMap { entry in
                vault.relativePath(of: entry.url).map { ($0, entry) }
            })
            try vault.rebuildIndex()
            error = nil
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func run(_ command: PaletteCommand) {
        do {
            switch command.action {
            case .today:
                onOpen(.note(try vault.openToday()))
            case .newNote:
                guard let root = vault.vaultURL else { throw VaultError.noVault }
                onOpen(.note(try vault.createNote(at: vault.newNoteURL(in: root))))
            case .settings:
                onOpen(.settings)
            case .trash:
                onOpen(.trash)
            case .saveVersion:
                guard let note = currentNote else { throw VaultError.noVault }
                if try vault.saveVersion(of: note) == nil {
                    error = "Already saved: nothing changed."
                }
            case .showPanel(let tab):
                onPanelTab?(tab)
                dismiss()
            case .format(let format):
                onFormat?(format)
                dismiss()
            case .resolveAll(let accept):
                // The same call as the review sheet's Accept all / Reject all (PendingSheet.resolveAll).
                guard let note = currentNote else { throw VaultError.noVault }
                try vault.resolveAllPending(for: note, accept: accept)
                NotificationCenter.default.post(name: .inkunoNoteChanged, object: note)
                dismiss()
            }
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func openToday() {
        do {
            onOpen(.note(try vault.openToday()))
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func create() {
        do {
            guard let root = vault.vaultURL else { throw VaultError.noVault }
            // A name cannot contain "/" or ":" on the Files side.
            let name = String(needle.filter { $0 != "/" && $0 != ":" })
            let url = try vault.createNote(at: root.appendingPathComponent(name).appendingPathExtension("md"))
            onOpen(.note(url))
        } catch {
            self.error = error.localizedDescription
        }
    }
}

/// One `>` command of the palette.
struct PaletteCommand: Identifiable {
    /// The sections of the `>` commands, in the order the palette lists them.
    enum Group: String, CaseIterable, Identifiable {
        /// New note and Today.
        case notes
        /// Bold, Italic and Heading for the open note. Hidden without a format host (`hasFormatHost`).
        case formatting
        /// Save version and the panel tabs, for the open note.
        case thisNote
        /// Accept and reject all changes, Open Settings and Restore from Trash.
        case window

        var id: String { rawValue }

        var title: String {
            switch self {
            case .notes: return "Notes"
            case .formatting: return "Formatting"
            case .thisNote: return "This note"
            case .window: return "Window"
            }
        }
    }

    enum Action {
        case today, newNote, settings, trash
        case saveVersion
        case showPanel(PanelTab)
        /// Accept (true) or reject (false) every pending change of the current note.
        case resolveAll(accept: Bool)
        /// Bold, Italic or Heading of the current note, applied by the host (`onFormat`).
        case format(EditorFormat)
    }

    var id: String { title }
    var group: Group
    var title: String
    var action: Action

    /// The SF Symbol of the row.
    var symbol: String {
        switch action {
        case .today: return "calendar"
        case .newNote: return "square.and.pencil"
        case .settings: return "gearshape"
        case .trash: return "arrow.uturn.backward"
        case .saveVersion: return "clock.arrow.circlepath"
        case .resolveAll(let accept): return accept ? "checkmark" : "xmark"
        case .format(let format):
            switch format {
            case .bold: return "bold"
            case .italic: return "italic"
            case .heading: return "number"
            }
        case .showPanel(let tab):
            switch tab {
            case .outline: return "list.bullet.indent"
            case .comments: return "text.bubble"
            case .versions: return "clock.arrow.circlepath"
            case .backlinks: return "link"
            }
        }
    }

    /// The full command list. The This-note commands (Save version, Outline/Comments/Versions/
    /// Backlinks) appear only when a note is open; the panel ones also need a host for the sheet.
    /// Accept/Reject all changes (Window) appear only when the open note has `hasPending` changes.
    /// Bold, Italic and Heading (Formatting) appear only for an open note with `hasFormatHost`.
    static func all(for currentNote: URL?, hasPanelHost: Bool, hasPending: Bool = false, hasFormatHost: Bool = false) -> [PaletteCommand] {
        var list = [
            PaletteCommand(group: .notes, title: "Today", action: .today),
            PaletteCommand(group: .notes, title: "New note", action: .newNote),
        ]
        if currentNote != nil, hasPending {
            list += [
                PaletteCommand(group: .window, title: "Accept all changes", action: .resolveAll(accept: true)),
                PaletteCommand(group: .window, title: "Reject all changes", action: .resolveAll(accept: false)),
            ]
        }
        if currentNote != nil, hasFormatHost {
            list += [
                PaletteCommand(group: .formatting, title: "Bold", action: .format(.bold)),
                PaletteCommand(group: .formatting, title: "Italic", action: .format(.italic)),
                PaletteCommand(group: .formatting, title: "Heading", action: .format(.heading)),
            ]
        }
        list += [
            PaletteCommand(group: .window, title: "Open Settings", action: .settings),
            PaletteCommand(group: .window, title: "Restore from Trash", action: .trash),
        ]
        if currentNote != nil {
            list.append(PaletteCommand(group: .thisNote, title: "Save version", action: .saveVersion))
            if hasPanelHost {
                for tab in PanelTab.allCases {
                    list.append(PaletteCommand(group: .thisNote, title: "Show \(tab.title.lowercased())", action: .showPanel(tab)))
                }
            }
        }
        return list
    }
}
