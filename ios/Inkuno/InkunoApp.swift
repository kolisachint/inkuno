import SwiftUI

@main
struct InkunoApp: App {
    @State private var vault = VaultStore()
    @AppStorage(AppearanceKey.theme) private var theme: ThemePreference = .system

    var body: some Scene {
        WindowGroup {
            RootView()
                .environment(vault)
                .preferredColorScheme(theme.colorScheme)
                .tint(InkColor.tint)
        }
    }
}

/// Everything the navigation stack can push.
enum Route: Hashable {
    case tree(URL)
    case note(URL)
    case settings
    case trash
}

/// Onboarding until a folder is chosen; then Home as the stack root, with push routes.
struct RootView: View {
    @Environment(VaultStore.self) private var vault
    @State private var path: [Route] = []
    @State private var showPalette = false

    var body: some View {
        if vault.hasVault {
            NavigationStack(path: $path) {
                HomeView(path: $path, showPalette: $showPalette)
                    .navigationDestination(for: Route.self) { route in
                        destination(for: route)
                    }
            }
            .sheet(isPresented: $showPalette) {
                PaletteSheet { route in
                    showPalette = false
                    path.append(route)
                }
            }
        } else {
            OnboardingView()
        }
    }

    @ViewBuilder
    private func destination(for route: Route) -> some View {
        switch route {
        case .tree(let folder):
            TreeView(folder: folder, path: $path)
        case .note(let url):
            NoteView(url: url)
        case .settings:
            SettingsView()
        case .trash:
            TrashView { note in
                showPalette = false
                path.append(.note(note))
            }
        }
    }
}
