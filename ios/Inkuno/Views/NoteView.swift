import SwiftUI

/// The note editor: the live Markdown text view, loaded and saved through VaultStore.
/// Autosave waits one second after the last change, and saves again when the view goes away.
struct NoteView: View {
    @Environment(VaultStore.self) private var vault
    @AppStorage(AppearanceKey.noteTextSize) private var textSize: NoteTextSize = .standard
    @AppStorage(AppearanceKey.pageWidth) private var pageWidth: PageWidth = .normal
    @AppStorage(AppearanceKey.showMarkdownOnLiveLine) private var showMarkdownOnLiveLine = true

    let url: URL
    var createIfMissing = false

    @State private var text = ""
    /// The text last read from or written to disk. Saves only when the text differs from it.
    @State private var savedText = ""
    @State private var loaded = false
    @State private var error: String?
    @State private var saveTask: Task<Void, Never>?
    @State private var showSlash = false
    @State private var editor = MarkdownEditorController()

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
                onSlash: { showSlash = true }
            )
            .padding(.horizontal, pageWidth.gutter)
        }
        .background(InkColor.sheet)
        .navigationTitle(url.deletingPathExtension().lastPathComponent)
        .navigationBarTitleDisplayMode(.inline)
        .onChange(of: text) { _, _ in scheduleSave() }
        .onDisappear(perform: flush)
        .task { load() }
        .sheet(isPresented: $showSlash) {
            SlashSheet { item in
                showSlash = false
                editor.insertBlock(item)
            }
            .presentationDetents([.medium])
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
        guard loaded, text != savedText else { return }
        do {
            try vault.writeNote(text, to: url)
            savedText = text
        } catch {
            self.error = error.localizedDescription
        }
    }
}
