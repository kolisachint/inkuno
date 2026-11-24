import SwiftUI

/// Settings values, stored with @AppStorage under the keys in `AppearanceKey`.
enum AppearanceKey {
    static let theme = "inkuno.theme"
    static let noteTextSize = "inkuno.noteTextSize"
    static let pageWidth = "inkuno.pageWidth"
    static let showMarkdownOnLiveLine = "inkuno.showMarkdownOnLiveLine"
    /// The name on revisions and comment bylines. Empty by default, never a device name;
    /// `AuthorName.resolved` falls back to "unknown" when it is empty.
    static let authorName = "inkuno.author"
}

/// The name on revisions and comment bylines, stored with the other appearance prefs.
enum AuthorName {
    /// Cleans a typed name like the desktop's `clean_author`: control characters (including
    /// newlines and tabs) become spaces, runs of whitespace collapse, the ends are trimmed.
    /// Empty stays empty, so the caller can fall back to "unknown".
    static func clean(_ raw: String) -> String {
        var flattened = ""
        flattened.reserveCapacity(raw.count)
        for scalar in raw.unicodeScalars {
            if CharacterSet.controlCharacters.contains(scalar) || CharacterSet.whitespacesAndNewlines.contains(scalar) {
                flattened.append(" ")
            } else {
                flattened.append(Character(scalar))
            }
        }
        return flattened.components(separatedBy: .whitespaces).filter { !$0.isEmpty }.joined(separator: " ")
    }

    /// The name on revisions and comments: the setting, cleaned; "unknown" when it is empty.
    static func resolved(in defaults: UserDefaults = .standard) -> String {
        let name = clean(defaults.string(forKey: AppearanceKey.authorName) ?? "")
        return name.isEmpty ? "unknown" : name
    }
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
