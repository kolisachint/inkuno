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
                // Blank scratch notes left by a crash go, as the desktop does at start (`clean_blank`).
                .task { ScratchStore.cleanBlank() }
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
    /// The panel segment the palette asked for on the note, sent to that note once the palette has closed.
    @State private var panelRequest: PanelRequest?
    @State private var queuedPanel: PanelRequest?
    /// A screen a palette command opens, pushed once the palette has gone: a push while the sheet is
    /// still dismissing can be dropped by the navigation stack.
    @State private var queuedRoute: Route?

    /// The note on top of the stack, if any: the palette's This-note commands act on it.
    private var currentNote: URL? {
        if case .note(let url) = path.last { return url }
        return nil
    }

    var body: some View {
        if vault.hasVault {
            NavigationStack(path: $path) {
                HomeView(path: $path, showPalette: $showPalette)
                    .navigationDestination(for: Route.self) { route in
                        destination(for: route)
                    }
            }
            .sheet(isPresented: $showPalette, onDismiss: sendQueued) {
                PaletteSheet(
                    onOpen: { route in
                        queuedRoute = route
                        showPalette = false
                    },
                    currentNote: currentNote,
                    onPanelTab: { tab in
                        guard let url = currentNote else { return }
                        queuedPanel = PanelRequest(url: url, tab: tab)
                    }
                )
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
            NoteView(url: url, panelRequest: $panelRequest)
        case .settings:
            SettingsView()
        case .trash:
            TrashView { note in
                showPalette = false
                path.append(.note(note))
            }
        }
    }

    /// Runs when the palette has closed: a panel command opens its segment on the note.
    /// What the palette asked for, once it has gone: a screen to push, or a panel tab to show.
    private func sendQueued() {
        if let route = queuedRoute {
            queuedRoute = nil
            path.append(route)
        }
        if let request = queuedPanel {
            queuedPanel = nil
            panelRequest = request
        }
    }
}
