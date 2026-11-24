import InkunoCore
import SwiftUI

/// The bottom sheet for the right panel: Outline, Comments, Versions, Backlinks.
/// Self-contained: reads and writes through VaultStore; the editor presents it and passes `onSelectLine`
/// to scroll the editor to a line. A write here posts `.inkunoNoteChanged`, so the editor reloads the text.
struct PanelSheet: View {
    let noteURL: URL
    let text: String
    var onSelectLine: (Int) -> Void = { _ in }

    @Environment(VaultStore.self) private var vault
    @Environment(\.dismiss) private var dismiss

    @State private var tab: PanelTab

    /// `initialTab` picks the segment shown first: the palette's "Show comments" and the like.
    init(noteURL: URL, text: String, initialTab: PanelTab = .outline, onSelectLine: @escaping (Int) -> Void = { _ in }) {
        self.noteURL = noteURL
        self.text = text
        self.onSelectLine = onSelectLine
        _tab = State(initialValue: initialTab)
    }

    @State private var comments: [NoteComment] = []
    /// The note's text after this sheet's last write: `text` is the editor's, which is stale until it reloads.
    @State private var written: String?
    @State private var versions: [Result<RevisionHeader, VaultStore.UnreadableVersion>] = []
    @State private var versionsError: String?
    @State private var backlinks: [(url: URL, title: String)] = []
    @State private var restoreCandidate: RevisionHeader?
    @State private var error: String?
    @State private var notice: String?

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                Picker("Panel", selection: $tab) {
                    ForEach(PanelTab.allCases) { tab in
                        Text(tab.title).tag(tab)
                    }
                }
                .pickerStyle(.segmented)
                .padding(.horizontal, InkMetrics.cardMarginH)
                .padding(.top, 12)
                .padding(.bottom, 8)

                ScrollView {
                    switch tab {
                    case .outline: outlineBody
                    case .comments: commentsBody
                    case .versions: versionsBody
                    case .backlinks: backlinksBody
                    }
                }
            }
            .background(InkColor.sheet)
            .navigationTitle(noteURL.deletingPathExtension().lastPathComponent)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }
                }
            }
            .alert("Inkuno", isPresented: Binding(get: { error != nil }, set: { if !$0 { error = nil } })) {
                Button("OK", role: .cancel) {}
            } message: {
                Text(error ?? "")
            }
            .alert(
                "Restore this version?",
                isPresented: Binding(get: { restoreCandidate != nil }, set: { if !$0 { restoreCandidate = nil } }),
                presenting: restoreCandidate
            ) { candidate in
                Button("Cancel", role: .cancel) {}
                Button("Restore") { restore(candidate) }
            } message: { _ in
                Text("The current text is saved as a version first, so nothing is lost.")
            }
            .task { reload() }
            .onChange(of: text) { _, _ in reload() }
        }
        .presentationDetents([.medium, .large])
        .overlay(alignment: .bottom) {
            if let notice {
                Text(notice)
                    .font(InkType.caption)
                    .foregroundStyle(InkColor.toastInk)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 8)
                    .background(InkColor.toast, in: Capsule())
                    .padding(.bottom, 16)
            }
        }
    }

    // MARK: Outline

    private var outline: [HeadingInfo] {
        headings(text: text)
    }

    @ViewBuilder
    private var outlineBody: some View {
        if outline.isEmpty {
            Text("No headings in this note.")
                .font(InkType.body)
                .foregroundStyle(InkColor.faint)
                .frame(maxWidth: .infinity)
                .padding(.top, 32)
        } else {
            LazyVStack(spacing: 0) {
                ForEach(outline, id: \.line) { heading in
                    Button {
                        dismiss()
                        onSelectLine(Int(heading.line))
                    } label: {
                        HStack {
                            Text(heading.title)
                                .font(InkType.rowTitle)
                                .foregroundStyle(InkColor.ink)
                                .lineLimit(2)
                            Spacer(minLength: 8)
                        }
                        .padding(.horizontal, InkMetrics.rowPaddingH + CGFloat((Int(heading.level) - 1) * 16))
                        .frame(minHeight: InkMetrics.rowMinHeight, alignment: .leading)
                    }
                    .buttonStyle(.plain)
                    Divider().padding(.leading, InkMetrics.rowPaddingH)
                }
            }
            .padding(.bottom, 24)
        }
    }

    // MARK: Comments

    @ViewBuilder
    private var commentsBody: some View {
        if comments.isEmpty {
            Text("No comments on this note.")
                .font(InkType.body)
                .foregroundStyle(InkColor.faint)
                .frame(maxWidth: .infinity)
                .padding(.top, 32)
        } else {
            LazyVStack(spacing: 12) {
                ForEach(comments) { comment in
                    CommentCard(
                        comment: comment,
                        onReply: { reply(comment, said: $0) },
                        onResolve: { resolve(comment) },
                        onGoToLine: {
                            dismiss()
                            onSelectLine(comment.line)
                        }
                    )
                    .padding(.horizontal, InkMetrics.cardMarginH)
                }
            }
            .padding(.top, 4)
            .padding(.bottom, 24)
        }
        Text("Comments are added from a block's long-press menu.")
            .font(InkType.caption)
            .foregroundStyle(InkColor.faint)
            .padding(.horizontal, InkMetrics.cardMarginH + 4)
            .padding(.bottom, 24)
    }

    // MARK: Versions

    @ViewBuilder
    private var versionsBody: some View {
        Button {
            saveVersion()
        } label: {
            Text("Save version")
                .font(.system(size: 17, weight: .semibold))
                .foregroundStyle(.white)
                .frame(maxWidth: .infinity, minHeight: 50)
                .background(InkColor.accent, in: RoundedRectangle(cornerRadius: InkRadius.cta, style: .continuous))
        }
        .buttonStyle(.plain)
        .padding(.horizontal, InkMetrics.cardMarginH)
        .padding(.vertical, 4)

        if let versionsError {
            Text(versionsError)
                .font(InkType.caption)
                .foregroundStyle(InkColor.red)
                .padding(.horizontal, InkMetrics.cardMarginH + 4)
        }
        if versions.isEmpty {
            Text("No saved versions yet.")
                .font(InkType.body)
                .foregroundStyle(InkColor.faint)
                .frame(maxWidth: .infinity)
                .padding(.top, 24)
        } else {
            LazyVStack(spacing: 0) {
                ForEach(versions.indices, id: \.self) { index in
                    switch versions[index] {
                    case .success(let header):
                        Button {
                            restoreCandidate = header
                        } label: {
                            HStack(spacing: 12) {
                                Image(systemName: "clock.arrow.circlepath")
                                    .frame(width: InkMetrics.markerIconSize)
                                    .foregroundStyle(InkColor.icon)
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(versionTitle(header))
                                        .font(InkType.rowTitle)
                                        .foregroundStyle(header.kind == .auto ? InkColor.soft : InkColor.ink)
                                    if let detail = versionDetail(header) {
                                        Text(detail)
                                            .font(InkType.caption)
                                            .foregroundStyle(InkColor.faint)
                                    }
                                }
                                Spacer(minLength: 8)
                                Image(systemName: "chevron.right")
                                    .font(.system(size: 13, weight: .semibold))
                                    .foregroundStyle(InkColor.faint)
                            }
                            .frame(minHeight: InkMetrics.rowMinHeight)
                            .padding(.horizontal, InkMetrics.rowPaddingH)
                            .padding(.vertical, 4)
                        }
                        .buttonStyle(.plain)
                        Divider().padding(.leading, InkMetrics.rowPaddingH)
                    case .failure(let unreadable):
                        let number = unreadable.number
                        HStack(spacing: 12) {
                            Image(systemName: "exclamationmark.triangle")
                                .foregroundStyle(InkColor.red)
                            Text("#\(number) · cannot be read")
                                .font(InkType.rowTitle)
                                .foregroundStyle(InkColor.faint)
                            Spacer(minLength: 8)
                        }
                        .frame(minHeight: InkMetrics.rowMinHeight)
                        .padding(.horizontal, InkMetrics.rowPaddingH)
                        Divider().padding(.leading, InkMetrics.rowPaddingH)
                    }
                }
            }
            .padding(.bottom, 24)
        }
    }

    // MARK: Backlinks

    /// The row title, as the core's `revisionRowName` builds it without the author: "#2 · auto", or the memo
    /// ("#3 · Before the review"). The author is not in it: `versionDetail` shows it when it is known.
    // ponytail: mirrors `revision.rs` row_name minus the author; keep the two in step.
    private func versionTitle(_ header: RevisionHeader) -> String {
        var title = "#\(header.number)"
        if header.kind == .auto {
            title += " · auto"
        }
        if let note = header.note, !note.isEmpty {
            title += " · \(note)"
        }
        return title
    }

    /// The author (when it is not "unknown") and the local time, joined with " · ". Nil when neither is known.
    private func versionDetail(_ header: RevisionHeader) -> String? {
        let when = header.date.isEmpty
            ? nil
            : revisionLocalDate(date: header.date, utcOffsetSeconds: Int64(TimeZone.current.secondsFromGMT()))
        let parts = [header.author == "unknown" || header.author.isEmpty ? nil : header.author, when].compactMap { $0 }
        return parts.isEmpty ? nil : parts.joined(separator: " · ")
    }

    @ViewBuilder
    private var backlinksBody: some View {
        if backlinks.isEmpty {
            Text("No notes link here yet.")
                .font(InkType.body)
                .foregroundStyle(InkColor.faint)
                .frame(maxWidth: .infinity)
                .padding(.top, 32)
        } else {
            LazyVStack(spacing: 0) {
                ForEach(backlinks, id: \.url) { link in
                    NavigationLink(value: Route.note(link.url)) {
                        HStack(spacing: 12) {
                            Image(systemName: "doc.text")
                                .frame(width: InkMetrics.markerIconSize)
                                .foregroundStyle(InkColor.icon)
                            Text(link.title)
                                .font(InkType.rowTitle)
                                .foregroundStyle(InkColor.ink)
                                .lineLimit(1)
                            Spacer(minLength: 8)
                            Image(systemName: "chevron.right")
                                .font(.system(size: 13, weight: .semibold))
                                .foregroundStyle(InkColor.faint)
                        }
                        .frame(minHeight: InkMetrics.rowMinHeight)
                        .padding(.horizontal, InkMetrics.rowPaddingH)
                        .padding(.vertical, 4)
                    }
                    .buttonStyle(.plain)
                    Divider().padding(.leading, InkMetrics.rowPaddingH)
                }
            }
            .padding(.bottom, 24)
        }
    }

    // MARK: Data

    private var current: String {
        written ?? text
    }

    private func reload() {
        comments = CommentFormat.parse(current)
        do {
            versions = try vault.revisionHeaders(for: noteURL).reversed()
            versionsError = nil
        } catch {
            versions = []
            versionsError = error.localizedDescription
        }
        backlinks = (try? vault.backlinks(to: noteURL)) ?? []
    }

    private func saveVersion() {
        do {
            if try vault.saveVersion(of: noteURL) == nil {
                flash("Already saved: nothing changed.")
            } else {
                flash("Version saved.")
            }
            reload()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func restore(_ header: RevisionHeader) {
        do {
            try vault.restoreVersion(number: header.number, for: noteURL)
            NotificationCenter.default.post(name: .inkunoNoteChanged, object: noteURL)
            restoreCandidate = nil
            flash("Restored #\(header.number). The previous text is kept as a version.")
            reload()
        } catch {
            restoreCandidate = nil
            self.error = error.localizedDescription
        }
    }

    private func resolve(_ comment: NoteComment) {
        do {
            let updated = try CommentFormat.resolve(in: current, comment: comment)
            try vault.writeNote(updated, to: noteURL)
            written = updated
            NotificationCenter.default.post(name: .inkunoNoteChanged, object: noteURL)
            flash("Comment resolved.")
            comments = CommentFormat.parse(updated)
        } catch {
            self.error = error.localizedDescription
        }
    }

    /// Adds the reply to the comment and writes the note; true when it was written.
    private func reply(_ comment: NoteComment, said: String) -> Bool {
        do {
            let updated = try CommentFormat.reply(in: current, comment: comment, said: said)
            try vault.writeNote(updated, to: noteURL)
            written = updated
            NotificationCenter.default.post(name: .inkunoNoteChanged, object: noteURL)
            comments = CommentFormat.parse(updated)
            return true
        } catch {
            self.error = error.localizedDescription
            return false
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

/// The four segments of the panel sheet.
enum PanelTab: String, CaseIterable, Identifiable {
    case outline, comments, versions, backlinks

    var id: String { rawValue }

    var title: String {
        switch self {
        case .outline: return "Outline"
        case .comments: return "Comments"
        case .versions: return "Versions"
        case .backlinks: return "Backlinks"
        }
    }
}
