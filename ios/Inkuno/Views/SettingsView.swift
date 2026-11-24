import SwiftUI
import UniformTypeIdentifiers

/// Settings, as a Form: notes folder, appearance, editor, about.
/// Trash is not in this skeleton yet.
struct SettingsView: View {
    @Environment(VaultStore.self) private var vault
    @AppStorage(AppearanceKey.theme) private var theme: ThemePreference = .system
    @AppStorage(AppearanceKey.noteTextSize) private var textSize: NoteTextSize = .standard
    @AppStorage(AppearanceKey.pageWidth) private var pageWidth: PageWidth = .normal
    @AppStorage(AppearanceKey.showMarkdownOnLiveLine) private var showMarkdownOnLiveLine = true

    @State private var showFolderImporter = false
    @State private var error: String?

    var body: some View {
        Form {
            Section("Notes folder") {
                LabeledContent("Folder", value: vault.displayName)
                Button("Change folder…") {
                    showFolderImporter = true
                }
            }

            Section("Appearance") {
                Picker("Theme", selection: $theme) {
                    ForEach(ThemePreference.allCases) { option in
                        Text(option.title).tag(option)
                    }
                }
                .pickerStyle(.segmented)

                Picker("Text size", selection: $textSize) {
                    ForEach(NoteTextSize.allCases) { option in
                        Text(option.title).tag(option)
                    }
                }

                Picker("Page width", selection: $pageWidth) {
                    ForEach(PageWidth.allCases) { option in
                        Text(option.title).tag(option)
                    }
                }
                .pickerStyle(.segmented)
            }

            Section("Editor") {
                Toggle("Show Markdown on the live line", isOn: $showMarkdownOnLiveLine)
            }

            if let error {
                Section {
                    Text(error)
                        .font(InkType.caption)
                        .foregroundStyle(InkColor.red)
                }
            }

            Section("About") {
                LabeledContent("Version", value: Self.versionText)
            }
        }
        .navigationTitle("Settings")
        .navigationBarTitleDisplayMode(.inline)
        .fileImporter(isPresented: $showFolderImporter, allowedContentTypes: [.folder]) { result in
            switch result {
            case .success(let url):
                do {
                    try vault.chooseVault(url)
                    error = nil
                } catch {
                    self.error = error.localizedDescription
                }
            case .failure(let failure):
                error = failure.localizedDescription
            }
        }
    }

    private static var versionText: String {
        let info = Bundle.main.infoDictionary
        let version = info?["CFBundleShortVersionString"] as? String ?? "?"
        let build = info?["CFBundleVersion"] as? String ?? "?"
        return "\(version) (\(build))"
    }
}
