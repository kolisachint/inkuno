import SwiftUI

/// A request from the palette's `>` commands to open a segment of the panel for a note. `id` makes each
/// request distinct, so the same command twice still opens the panel.
struct PanelRequest: Equatable {
    let url: URL
    let tab: PanelTab
    var id = UUID()
}

/// A formatting command the palette asked for on the note at `url`, applied by that note's editor once the palette
/// has closed. `id` makes each request distinct, so the same command twice still applies.
struct FormatRequest: Equatable {
    /// The note the command applies to.
    let url: URL
    /// The formatting command to apply.
    let format: EditorFormat
    /// Makes this request distinct from an earlier one for the same command.
    var id = UUID()
}

/// The note editor: the live Markdown text view, loaded and saved through VaultStore.
/// Autosave waits one second after the last change, and saves again when the view goes away.
/// The nav bar holds the panel (Outline, Comments, Versions, Backlinks) and the more menu. The pending
/// pill shows at the bottom while the note has pending changes. Reviewing them reloads the text.
struct NoteView: View {
    @Environment(VaultStore.self) private var vault
    @Environment(\.dismiss) private var dismiss
    @AppStorage(AppearanceKey.noteTextSize) private var textSize: NoteTextSize = .standard
    @AppStorage(AppearanceKey.pageWidth) private var pageWidth: PageWidth = .normal
    @AppStorage(AppearanceKey.showMarkdownOnLiveLine) private var showMarkdownOnLiveLine = true

    let url: URL
    var createIfMissing = false
    /// The palette's request to open a panel segment (see `RootView`). Taken here when it is for this note.
    @Binding var panelRequest: PanelRequest?
    /// The palette's formatting command for this note (see `RootView`). Taken here when it is for this note.
    @Binding var formatRequest: FormatRequest?
    /// The notice the stack shows over its screens (see `RootView`); Undo after a move to the Trash is one.
    @Binding var notice: InkNotice?
    /// Pushes a note a tapped link opens, with the 0-based line of the link's heading or block (see `RootView`).
    let openLinked: (URL, Int?) -> Void

    @State private var text = ""
    /// The text last read from or written to disk. Saves only when the text differs from it.
    @State private var savedText = ""
    @State private var loaded = false
    @State private var error: String?
    @State private var saveTask: Task<Void, Never>?
    @State private var showSlash = false
    @State private var panelTab: PanelTab?
    @State private var showPending = false
    @State private var commentDraft: CommentDraft?
    /// Set once the note is in the trash: nothing must write the file back to where it was.
    @State private var trashed = false
    @State private var editor = MarkdownEditorController(provider: CoreMarkdownProvider())
    /// The palette a tapped tag or an unwritten day opens, with `linkQuery` as its search. The note it picks
    /// opens once the palette has gone (`pickedNote`).
    @State private var showLinkPalette = false
    /// The search the link palette opens with: the tag or the name that no note answers.
    @State private var linkQuery = ""
    /// The note the link palette picked, opened once the palette has gone.
    @State private var pickedNote: URL?
    /// The line this note opens at, from the link that opened it; taken on the first load.
    @State private var pendingLine: Int?
    /// The comment whose passage was tapped; its popover shows while it is set.
    @State private var openedComment: NoteComment?

    init(url: URL, createIfMissing: Bool = false, panelRequest: Binding<PanelRequest?> = .constant(nil), formatRequest: Binding<FormatRequest?> = .constant(nil), startLine: Int? = nil, notice: Binding<InkNotice?> = .constant(nil), openLinked: @escaping (URL, Int?) -> Void = { _, _ in }) {
        self.url = url
        self.createIfMissing = createIfMissing
        _panelRequest = panelRequest
        _formatRequest = formatRequest
        _pendingLine = State(initialValue: startLine)
        _notice = notice
        self.openLinked = openLinked
    }

    /// A scratch note lives outside the vault (`ScratchStore`): no versions, pending list or vault trash.
    private var isScratch: Bool {
        ScratchStore.holds(url)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let error {
                Text(error)
                    .font(InkType.caption)
                    .foregroundStyle(InkColor.red)
                    .padding(.horizontal, pageWidth.gutter)
                    .padding(.top, 8)
                if !loaded {
                    Button("Try again", action: load)
                        .padding(.horizontal, pageWidth.gutter)
                }
            }
            LiveMarkdownEditor(
                text: $text,
                controller: editor,
                textSize: textSize.points,
                showMarkers: showMarkdownOnLiveLine,
                isEditable: loaded,
                onSlash: { showSlash = true },
                onComment: { line, passage in
                    commentDraft = CommentDraft(line: line, passage: passage)
                }
            )
            .padding(.horizontal, pageWidth.gutter)
        }
        .background(InkColor.sheet)
        .navigationTitle(url.deletingPathExtension().lastPathComponent)
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    flush()
                    panelTab = .outline
                } label: {
                    Image(systemName: "sidebar.trailing")
                }
                .accessibilityLabel("Panel")
                .disabled(!loaded)
            }
            ToolbarItem(placement: .topBarTrailing) {
                Menu {
                    if !isScratch {
                        Button(action: saveVersion) {
                            Label("Save version", systemImage: "clock.arrow.circlepath")
                        }
                    }
                    Toggle(isOn: $showMarkdownOnLiveLine) {
                        Label("Show Markdown", systemImage: "chevron.left.forwardslash.chevron.right")
                    }
                    Divider()
                    Button(role: .destructive, action: moveToTrash) {
                        Label("Move to Trash", systemImage: "trash")
                    }
                } label: {
                    Image(systemName: "ellipsis.circle")
                }
                .accessibilityLabel("More")
                .disabled(!loaded)
            }
        }
        .overlay(alignment: .bottom) {
            VStack(spacing: 8) {
                if !isScratch {
                    PendingPill(noteURL: url) {
                        flush()
                        showPending = true
                    }
                }
            }
            .padding(.bottom, 16)
        }
        .onChange(of: text) { _, _ in scheduleSave() }
        .onChange(of: panelRequest) { _, request in openRequested(request) }
        .onChange(of: formatRequest) { _, request in applyRequested(request) }
        .onAppear {
            openRequested(panelRequest)
            applyRequested(formatRequest)
            wireEditor()
        }
        .onReceive(NotificationCenter.default.publisher(for: .inkunoNoteChanged)) { note in
            guard (note.object as? URL) == url else { return }
            reloadFromDisk()
        }
        .onDisappear(perform: leave)
        .task { load() }
        .sheet(isPresented: $showSlash) {
            SlashSheet { item in
                showSlash = false
                editor.insertBlock(item)
            }
            .presentationDetents([.medium])
            .presentationDragIndicator(.visible)
        }
        .sheet(item: $panelTab) { tab in
            PanelSheet(noteURL: url, text: text, initialTab: tab, onSelectLine: { editor.reveal(line: $0) })
        }
        .sheet(isPresented: $showPending) {
            PendingSheet(noteURL: url, notice: $notice)
        }
        .sheet(item: $commentDraft) { draft in
            CommentComposer(draft: draft) { said in
                _ = editor.commentLine(draft.line, said: said)
            }
        }
        .sheet(isPresented: $showLinkPalette, onDismiss: openPickedNote) {
            PaletteSheet(
                onOpen: { route in
                    if case .note(let picked) = route {
                        pickedNote = picked
                    }
                    showLinkPalette = false
                },
                initialQuery: linkQuery
            )
        }
        .sheet(item: $openedComment) { comment in
            CommentPopover(
                comment: comment,
                onReply: { reply(to: comment, said: $0) },
                onResolve: { resolve(comment) }
            )
            .presentationDetents([.medium, .large])
            .presentationDragIndicator(.visible)
        }
    }

    private func load() {
        error = nil
        do {
            if createIfMissing {
                try vault.createIfMissing(at: url)
            }
            let content = try vault.readNote(at: url)
            savedText = content
            text = content
            editor.update(text: content)
            if let line = pendingLine {
                // The text view lays out the new text after this turn, so a scroll now would find no layout.
                // It runs on the next main-queue turn instead.
                let reveal = editor
                DispatchQueue.main.async { reveal.reveal(line: line) }
                pendingLine = nil
            }
            loaded = true
        } catch {
            self.error = error.localizedDescription
        }
    }

    /// Debounced save: one second after the last change.
    private func scheduleSave() {
        guard loaded, text != savedText else { return }
        saveTask?.cancel()
        saveTask = Task {
            try? await Task.sleep(for: .seconds(1))
            guard !Task.isCancelled else { return }
            save()
        }
    }

    private func flush() {
        saveTask?.cancel()
        save()
    }

    private func save() {
        guard loaded, !trashed, text != savedText else { return }
        do {
            try vault.writeNote(text, to: url)
            savedText = text
        } catch {
            self.error = error.localizedDescription
        }
    }

    /// Leaving the note: the text is saved, and a scratch note left blank is dropped (`clean_blank`).
    private func leave() {
        guard !trashed else { return }
        flush()
        if isScratch {
            ScratchStore.discardIfBlank(url)
        }
    }

    /// The file changed on disk under the editor: after accept or reject, or a comment resolved from the
    /// panel. The caller has saved its own text first, so the disk holds the newest.
    private func reloadFromDisk() {
        guard loaded, !trashed else { return }
        do {
            let content = try vault.readNote(at: url)
            guard content != text else { return }
            savedText = content
            text = content
            editor.update(text: content)
        } catch {
            self.error = error.localizedDescription
        }
    }

    /// Opens the panel segment a palette command asked for. The palette has closed by now (RootView sends the
    /// request from the palette's dismissal), so the panel can present.
    private func openRequested(_ request: PanelRequest?) {
        guard let request, request.url == url else { return }
        panelRequest = nil
        flush()
        panelTab = request.tab
    }

    /// Applies the formatting the palette asked for, in the editor, as an ordinary edit: it can be undone.
    private func applyRequested(_ request: FormatRequest?) {
        guard let request, request.url == url, loaded else { return }
        formatRequest = nil
        editor.apply(request.format)
    }

    /// Hands the editor what a tap on a link, a tag or a day opens.
    private func wireEditor() {
        editor.onOpenLink = { openLink($0, markup: $1) }
        editor.onOpenDate = { openDay($0) }
        editor.onOpenComment = { openedComment = $0 }
    }

    /// Adds a reply to `comment` and writes the note, as the panel does; true when it was written. The popover
    /// then shows the comment with the reply.
    private func reply(to comment: NoteComment, said: String) -> Bool {
        flush()
        do {
            let updated = try CommentFormat.reply(in: text, comment: comment, said: said)
            try vault.writeNote(updated, to: url)
            NotificationCenter.default.post(name: .inkunoNoteChanged, object: url)
            openedComment = CommentFormat.parse(updated).first { $0.startUtf16 == comment.startUtf16 }
            return true
        } catch {
            self.error = error.localizedDescription
            return false
        }
    }

    /// Resolves `comment` (its markup goes, the passage stays) and writes the note; the popover closes.
    private func resolve(_ comment: NoteComment) {
        flush()
        do {
            let updated = try CommentFormat.resolve(in: text, comment: comment)
            try vault.writeNote(updated, to: url)
            NotificationCenter.default.post(name: .inkunoNoteChanged, object: url)
            openedComment = nil
        } catch {
            self.error = error.localizedDescription
        }
    }

    /// Opens the note a link names: a wiki link by its name and heading (`markup` is `[[Note#Heading]]`), a Markdown
    /// link by its path (relative to this note's folder, or to the vault for a leading "/"), and a tag as the
    /// palette's `#tag` search.
    private func openLink(_ link: InkunoCore.LinkInfo, markup: String) {
        switch link.kind {
        case .wiki:
            let inner = String(markup.dropFirst(2).dropLast(2))
            guard let target = InkunoCore.linkTarget(inner: inner),
                  let found = try? vault.searchIndex().resolve(name: target.note),
                  let noteURL = vault.noteURL(path: found.path)
            else {
                showPalette(query: link.target)
                return
            }
            showNote(noteURL, anchor: target.anchor)
        case .note:
            let isFromRoot = link.target.hasPrefix("/")
            let folder = isFromRoot ? vault.vaultURL : url.deletingLastPathComponent()
            let path = isFromRoot ? String(link.target.dropFirst()) : link.target
            let anchor = link.anchor.map { InkunoCore.LinkAnchor.heading(title: $0) } ?? .whole
            guard let noteURL = folder?.appendingPathComponent(path) else { return }
            showNote(noteURL, anchor: anchor)
        case .tag:
            showPalette(query: "#" + link.target)
        case .url, .heading, .footnote:
            break
        }
    }

    /// Opens the daily note of the day `word` names (`CoreMarkdownProvider.day(named:)`), created when it is not
    /// written yet. A word that names no day opens the palette with it.
    private func openDay(_ word: String) {
        guard let day = CoreMarkdownProvider.day(named: word),
              let date = Calendar.current.date(from: DateComponents(
                  year: Int(day.date.year), month: Int(day.date.month), day: Int(day.date.day)
              ))
        else {
            showPalette(query: word)
            return
        }
        do {
            showNote(try vault.openToday(now: date), anchor: .whole)
        } catch {
            flash(error.localizedDescription)
        }
    }

    /// Opens the note at `noteURL` on the stack above this one, scrolled to the line of `anchor` (its heading or
    /// block). Back returns here.
    private func showNote(_ noteURL: URL, anchor: InkunoCore.LinkAnchor) {
        guard FileManager.default.fileExists(atPath: noteURL.path) else {
            flash("No note at “\(noteURL.lastPathComponent)”.")
            return
        }
        var line: Int?
        if let content = try? vault.readNote(at: noteURL),
           let offset = InkunoCore.linkPosition(text: content, anchor: anchor) {
            line = content.utf16.prefix(Int(offset)).filter { $0 == 10 }.count
        }
        openLinked(noteURL, line)
    }

    /// Opens the palette with `query`, for a tag or a name that no note answers.
    private func showPalette(query: String) {
        linkQuery = query
        showLinkPalette = true
    }

    /// Runs once the palette has gone: a note it picked opens over this one.
    private func openPickedNote() {
        guard let picked = pickedNote else { return }
        pickedNote = nil
        showNote(picked, anchor: .whole)
    }

    private func saveVersion() {
        flush()
        do {
            if try vault.saveVersion(of: url) == nil {
                flash("Already saved: nothing changed.")
            } else {
                flash("Version saved.")
            }
        } catch {
            self.error = error.localizedDescription
        }
    }

    /// Moves the note to the Trash and closes it. A vault note's notice offers Undo, which restores it to the
    /// folder it came from. The notice belongs to the stack, so it stays once this screen has closed.
    private func moveToTrash() {
        flush()
        let title = url.deletingPathExtension().lastPathComponent
        do {
            if isScratch {
                try ScratchStore.trash(url)
                notice = InkNotice("Moved “\(title)” to Trash", duration: 4)
            } else {
                let trashedURL = try vault.trashItem(at: url)
                notice = InkNotice("Moved “\(title)” to Trash", duration: 4, actionTitle: "Undo") {
                    do {
                        try vault.restoreFromTrash(at: trashedURL)
                    } catch {
                        notice = InkNotice(error.localizedDescription)
                    }
                }
            }
            trashed = true
            dismiss()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func flash(_ message: String) {
        notice = InkNotice(message, duration: 2.2)
    }
}
