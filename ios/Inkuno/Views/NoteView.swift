import SwiftUI

/// Placeholder editor: a plain TextEditor that loads and saves the note through VaultStore.
/// The live Markdown editor is a later spike (TextKit 2, driven by the Rust core).
struct NoteView: View {
    @Environment(VaultStore.self) private var vault
    @AppStorage(AppearanceKey.noteTextSize) private var textSize: NoteTextSize = .standard
    @AppStorage(AppearanceKey.pageWidth) private var pageWidth: PageWidth = .normal

    let url: URL
    var createIfMissing = false

    @State private var text = ""
    /// The text last read from or written to disk. Saves only when the text differs from it.
    @State private var savedText = ""
    @State private var loaded = false
    @State private var error: String?
    @State private var saveTask: Task<Void, Never>?

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
            TextEditor(text: $text)
                .font(.system(size: textSize.points))
                .lineSpacing(InkType.bodyLineSpacing(for: textSize.points))
                .foregroundStyle(InkColor.ink)
                .scrollContentBackground(.hidden)
                .padding(.horizontal, pageWidth.gutter - 5) // TextEditor adds its own inset of about 5pt
                .disabled(!loaded)
        }
        .background(InkColor.sheet)
        .navigationTitle(url.deletingPathExtension().lastPathComponent)
        .navigationBarTitleDisplayMode(.inline)
        .onChange(of: text) { _, _ in scheduleSave() }
        .onDisappear(perform: flush)
        .task { load() }
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

    /// Debounced save: a short pause after the last keystroke.
    private func scheduleSave() {
        guard loaded, text != savedText else { return }
        saveTask?.cancel()
        saveTask = Task {
            try? await Task.sleep(for: .milliseconds(600))
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
