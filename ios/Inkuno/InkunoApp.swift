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
    /// A note a link in another note opened: its line is scrolled to once it loads.
    case linked(URL, line: Int?)
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
    /// The formatting the palette asked for on the note, sent to that note once the palette has closed.
    @State private var formatRequest: FormatRequest?
    /// The formatting command the palette queued, sent to `formatRequest` once the palette has closed.
    @State private var queuedFormat: FormatRequest?
    /// A screen a palette command opens, pushed once the palette has gone: a push while the sheet is
    /// still dismissing can be dropped by the navigation stack.
    @State private var queuedRoute: Route?
    /// The notice over the stack (Restored, Moved to Trash with Undo, Saved). It outlives the screen that
    /// raised it, so an Undo still shows after the note that was trashed has closed.
    @State private var notice: InkNotice?

    /// The note on top of the stack, if any: the palette's This-note commands act on it.
    private var currentNote: URL? {
        switch path.last {
        case .note(let url), .linked(let url, _): return url
        default: return nil
        }
    }

    var body: some View {
        if vault.hasVault {
            NavigationStack(path: $path) {
                HomeView(path: $path, showPalette: $showPalette)
                    .navigationDestination(for: Route.self) { route in
                        destination(for: route)
                    }
            }
            .inkNotice($notice)
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
                    },
                    onFormat: { format in
                        guard let url = currentNote else { return }
                        queuedFormat = FormatRequest(url: url, format: format)
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
            TreeView(folder: folder, path: $path, showPalette: $showPalette, notice: $notice)
        case .note(let url):
            NoteView(url: url, panelRequest: $panelRequest, formatRequest: $formatRequest, notice: $notice, openLinked: openLinkedNote)
        case .linked(let url, let line):
            NoteView(url: url, panelRequest: $panelRequest, formatRequest: $formatRequest, startLine: line, notice: $notice, openLinked: openLinkedNote)
        case .settings:
            SettingsView()
        case .trash:
            TrashView(notice: $notice) { note in
                showPalette = false
                path.append(.note(note))
            }
        }
    }

    /// A link in a note opened another note: it is pushed, so Back returns to the note the link was tapped in.
    private func openLinkedNote(_ url: URL, _ line: Int?) {
        path.append(.linked(url, line: line))
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
        if let request = queuedFormat {
            queuedFormat = nil
            formatRequest = request
        }
    }
}
