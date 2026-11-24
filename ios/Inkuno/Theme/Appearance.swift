import SwiftUI

/// Settings values, stored with @AppStorage under the keys in `AppearanceKey`.
enum AppearanceKey {
    static let theme = "inkuno.theme"
    static let noteTextSize = "inkuno.noteTextSize"
    static let pageWidth = "inkuno.pageWidth"
    static let showMarkdownOnLiveLine = "inkuno.showMarkdownOnLiveLine"
}

enum ThemePreference: String, CaseIterable, Identifiable {
    case light, dark, system

    var id: String { rawValue }

    var title: String {
        switch self {
        case .light: return "Light"
        case .dark: return "Dark"
        case .system: return "System"
        }
    }

    /// nil follows the system setting.
    var colorScheme: ColorScheme? {
        switch self {
        case .light: return .light
        case .dark: return .dark
        case .system: return nil
        }
    }
}

enum NoteTextSize: Int, CaseIterable, Identifiable {
    case small = 14, medium = 15, standard = 16, large = 18

    var id: Int { rawValue }
    var points: CGFloat { CGFloat(rawValue) }
    var title: String { "\(rawValue)" }
}

enum PageWidth: String, CaseIterable, Identifiable {
    case narrow, normal, wide

    var id: String { rawValue }

    var title: String { rawValue.capitalized }

    /// Horizontal document gutter in points.
    var gutter: CGFloat {
        switch self {
        case .narrow: return InkMetrics.gutterNarrow
        case .normal: return InkMetrics.gutterNormal
        case .wide: return InkMetrics.gutterWide
        }
    }
}
