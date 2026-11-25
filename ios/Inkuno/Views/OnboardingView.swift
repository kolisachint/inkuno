import SwiftUI
import UniformTypeIdentifiers

struct OnboardingView: View {
    @Environment(VaultStore.self) private var vault
    @State private var showImporter = false
    @State private var error: String?

    var body: some View {
        VStack(spacing: 20) {
            Spacer()
            InkMark()
            VStack(spacing: 8) {
                Text("Inkuno")
                    .font(InkType.largeTitle)
                    .foregroundStyle(InkColor.ink)
                Text("Pick the folder that holds your notes. It is the same folder as on the desktop: Markdown files in the Files app.")
                    .font(InkType.body)
                    .multilineTextAlignment(.center)
                    .foregroundStyle(InkColor.soft)
            }
            .padding(.horizontal, 32)

            Button {
                showImporter = true
            } label: {
                Text("Choose folder")
                    .font(.system(size: 17, weight: .semibold))
                    .frame(maxWidth: .infinity, minHeight: 50)
                    .foregroundStyle(.white)
                    .background(InkColor.accent, in: RoundedRectangle(cornerRadius: InkRadius.cta, style: .continuous))
            }
            .padding(.horizontal, 32)
            .padding(.top, 8)

            if let message = error ?? vault.lastError {
                Text(message)
                    .font(InkType.caption)
                    .foregroundStyle(InkColor.red)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 32)
            }
            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(InkColor.ground)
        .fileImporter(isPresented: $showImporter, allowedContentTypes: [.folder]) { result in
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
}
