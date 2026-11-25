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
    @AppStorage(AppearanceKey.authorName) private var authorName = ""

    @State private var showFolderImporter = false
    @State private var error: String?
    @State private var trashCount = 0

    var body: some View {
        Form {
            Section {
                LabeledContent("Folder", value: vault.displayName)
                Button("Change folder…") {
                    showFolderImporter = true
                }
            } header: {
                Text("Notes folder")
            } footer: {
                Text("The same folder as on your Mac. Changes sync through iCloud Drive.")
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
                .pickerStyle(.segmented)

                Picker("Page width", selection: $pageWidth) {
                    ForEach(PageWidth.allCases) { option in
                        Text(option.title).tag(option)
                    }
                }
                .pickerStyle(.segmented)
            }

            Section("Editor") {
                TextField("Your name", text: $authorName)
                    .textInputAutocapitalization(.words)
                    .autocorrectionDisabled()
                Toggle("Show Markdown on the live line", isOn: $showMarkdownOnLiveLine)
            }

            Section("Files") {
                NavigationLink(value: Route.trash) {
                    Label {
                        HStack {
                            Text("Trash")
                            Spacer(minLength: 8)
                            Text("\(trashCount)")
                                .foregroundStyle(InkColor.faint)
                        }
                    } icon: {
                        Image(systemName: "trash")
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

            Section("About") {
                LabeledContent("Version", value: Self.versionText)
            }
        }
        .navigationTitle("Settings")
        .navigationBarTitleDisplayMode(.large)
        .task { trashCount = vault.trashCount() }
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
