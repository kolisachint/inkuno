import SwiftUI

/// A request from the palette's `>` commands to open a segment of the panel for a note. `id` makes each
/// request distinct, so the same command twice still opens the panel.
struct PanelRequest: Equatable {
    let url: URL
    let tab: PanelTab
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

    @State private var text = ""
    /// The text last read from or written to disk. Saves only when the text differs from it.
    @State private var savedText = ""
    @State private var loaded = false
    @State private var error: String?
    @State private var notice: String?
    @State private var saveTask: Task<Void, Never>?
    @State private var showSlash = false
    @State private var panelTab: PanelTab?
    @State private var showPending = false
    @State private var commentDraft: CommentDraft?
    /// Set once the note is in the trash: nothing must write the file back to where it was.
    @State private var trashed = false
    @State private var editor = MarkdownEditorController(provider: CoreMarkdownProvider())

    init(url: URL, createIfMissing: Bool = false, panelRequest: Binding<PanelRequest?> = .constant(nil)) {
        self.url = url
        self.createIfMissing = createIfMissing
        _panelRequest = panelRequest
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
                if let notice {
                    Text(notice)
                        .font(InkType.caption)
                        .foregroundStyle(InkColor.toastInk)
                        .padding(.horizontal, 14)
                        .padding(.vertical, 8)
                        .background(InkColor.toast, in: Capsule())
                }
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
        .onAppear { openRequested(panelRequest) }
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
            PendingSheet(noteURL: url)
        }
        .sheet(item: $commentDraft) { draft in
            CommentComposer(draft: draft) { said in
                _ = editor.commentLine(draft.line, said: said)
            }
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

    private func moveToTrash() {
        flush()
        do {
            if isScratch {
                try ScratchStore.trash(url)
            } else {
                try vault.trashItem(at: url)
            }
            trashed = true
            dismiss()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func flash(_ message: String) {
        notice = message
        Task { @MainActor in
            try? await Task.sleep(for: .seconds(2.2))
            if notice == message { notice = nil }
        }
    }
}
