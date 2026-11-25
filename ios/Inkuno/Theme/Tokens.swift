import SwiftUI
import UIKit

// Design tokens from docs/design/prototypes/v3-ios.html (summarised in /tmp/inkuno-ios/spec.md).
// Every colour is a dynamic UIColor-backed Color, so it follows the trait collection, which
// `.preferredColorScheme` sets from the Theme setting.

/// Builds an sRGB UIColor from 0xRRGGBB and an alpha.
private func rgb(_ hex: UInt32, _ alpha: Double = 1) -> UIColor {
    UIColor(
        red: CGFloat((hex >> 16) & 0xFF) / 255,
        green: CGFloat((hex >> 8) & 0xFF) / 255,
        blue: CGFloat(hex & 0xFF) / 255,
        alpha: CGFloat(alpha)
    )
}

/// A Color that resolves to `light` or `dark` from the current interface style.
private func dyn(_ light: UIColor, _ dark: UIColor) -> Color {
    Color(uiColor: UIColor { traits in
        traits.userInterfaceStyle == .dark ? dark : light
    })
}

enum InkColor {
    // Surfaces
    static let page = dyn(rgb(0xe4e5e7), rgb(0x0b0b0c))
    static let ground = dyn(rgb(0xeff0f1), rgb(0x141517))
    static let sheet = dyn(rgb(0xffffff), rgb(0x1c1c1e))

    // Ink
    static let ink = dyn(rgb(0x1f2225), rgb(0xe6e6e6))
    static let sideInk = dyn(rgb(0x1f2225, 0.85), rgb(0xe6e6e6, 0.82))
    static let soft = dyn(rgb(0x5d6064), rgb(0xa3a5a8))
    static let faint = dyn(rgb(0x9c9fa0), rgb(0x6f7174))
    static let icon = dyn(rgb(0x3f4143), rgb(0xc9cacb))

    // Interaction washes
    static let line = dyn(rgb(0x000000, 0.08), rgb(0xffffff, 0.08))
    static let hover = dyn(rgb(0x000000, 0.045), rgb(0xffffff, 0.055))
    static let selected = dyn(rgb(0x000000, 0.072), rgb(0xffffff, 0.085))
    static let pressed = dyn(rgb(0x000000, 0.09), rgb(0xffffff, 0.11))

    // Accent and links
    static let accent = dyn(rgb(0x3a85f7), rgb(0x4b8ff8))
    static let selection = dyn(rgb(0x3a85f7, 0.24), rgb(0x4b8ff8, 0.32))
    static let link = dyn(rgb(0x105592), rgb(0x8ab4f8))
    static let tint = dyn(rgb(0x3a85f7), rgb(0x3a85f7))

    // Code, tables, quotes
    static let codeBackground = dyn(rgb(0xfbfbfb), rgb(0x212123))
    static let codeLine = dyn(rgb(0xeaebeb), rgb(0x36373a))
    static let gutter = dyn(rgb(0xf8f8f8), rgb(0x1f1f21))
    static let gutterNumber = dyn(rgb(0xb9babc), rgb(0x868686))
    static let tableHeader = dyn(rgb(0xf8f8f8), rgb(0x232325))
    static let tableLine = dyn(rgb(0x000000, 0.09), rgb(0xffffff, 0.09))
    static let chip = dyn(rgb(0xf6f6f7), rgb(0x262628))
    static let chipLine = dyn(rgb(0xe2e3e3), rgb(0x353638))
    static let popover = dyn(rgb(0xfafafa), rgb(0x2a2a2c))
    static let quote = dyn(rgb(0xc9cacb), rgb(0x676767))

    // Controls
    static let segment = dyn(rgb(0xebeced), rgb(0x202022))
    static let segmentOn = dyn(rgb(0xffffff), rgb(0x3a3a3c))
    static let field = dyn(rgb(0x8a8d91), rgb(0x6f7174))

    // Syntax highlighting
    static let syntaxComment = dyn(rgb(0x8a8d93), rgb(0x8d9096))
    static let syntaxKeyword = dyn(rgb(0xb2565c), rgb(0xe06c75))
    static let syntaxString = dyn(rgb(0x89713a), rgb(0xd8b25c))
    static let syntaxNumber = dyn(rgb(0x9759b3), rgb(0xc58af9))
    static let syntaxType = dyn(rgb(0x3878b1), rgb(0x6cb6ff))
    static let syntaxFunction = dyn(rgb(0x5f7e43), rgb(0x98c379))

    // Callout alerts
    static let alertNote = dyn(rgb(0x2f6fd6), rgb(0x6ea6ff))
    static let alertTip = dyn(rgb(0x2f8a57), rgb(0x58c48a))
    static let alertImportant = dyn(rgb(0x7c4dd6), rgb(0xa98bff))
    static let alertWarning = dyn(rgb(0xa8650a), rgb(0xe0a23c))
    static let alertCaution = dyn(rgb(0xc93a3a), rgb(0xff7b72))

    // Toast
    static let toast = dyn(rgb(0x1f2225), rgb(0xe6e6e6))
    static let toastInk = dyn(rgb(0xffffff), rgb(0x1c1c1e))

    // Diff and pending changes
    static let addWash = dyn(rgb(0x2ea05a, 0.15), rgb(0x58c48a, 0.16))
    static let addInk = dyn(rgb(0x1f7a43), rgb(0x7fe0a6))
    static let deleteWash = dyn(rgb(0xc93a3a, 0.11), rgb(0xff7b72, 0.15))
    static let deleteInk = dyn(rgb(0xb3322f), rgb(0xff9d96))

    // Comments
    static let commentWash = dyn(rgb(0xe8b01e, 0.26), rgb(0xe8b01e, 0.22))
    static let commentDot = dyn(rgb(0xc98a00), rgb(0xf0c040))

    // Highlight, scrim, destructive
    static let highlight = dyn(rgb(0x3a85f7, 0.16), rgb(0x4b8ff8, 0.24))
    static let scrim = dyn(rgb(0x000000, 0.12), rgb(0x000000, 0.45))
    static let red = dyn(rgb(0xc93a3a), rgb(0xff7b72))
}

enum InkType {
    // ponytail: Inter and JetBrains Mono are not bundled yet; system fonts stand in
    // (SF and SF Mono). Bundle the fonts when the design is signed off.
    static let largeTitle = Font.system(size: 34, weight: .bold)
    static let sectionHeader = Font.system(size: 13, weight: .semibold)
    static let body = Font.system(size: 16)
    static let rowTitle = Font.system(size: 17)
    static let caption = Font.system(size: 13)
    static let mono = Font.system(size: 16, design: .monospaced)

    /// Section headers are 13pt, uppercase, with .04em tracking.
    static let sectionTracking: CGFloat = 0.52

    /// Markdown heading size for `level` (1 to 6) against a base size.
    /// H1 1.9em/700, H2 1.45em/650, H3 1.15em/600, H4 to H6 1em/600.
    static func heading(_ level: Int, base: CGFloat = 16) -> Font {
        let size = headingSize(level, base: base)
        return .system(size: size, weight: headingWeight(level))
    }

    static func headingSize(_ level: Int, base: CGFloat = 16) -> CGFloat {
        switch level {
        case 1: return base * 1.9
        case 2: return base * 1.45
        case 3: return base * 1.15
        default: return base
        }
    }

    private static func headingWeight(_ level: Int) -> Font.Weight {
        switch level {
        case 1: return .bold
        case 2: return .semibold // 650 in the prototype; closest SwiftUI weight
        default: return .semibold
        }
    }

    /// Body line-height 1.6: CSS line-height turns into extra spacing between lines.
    /// ponytail: approximate. The editor should measure the real line box.
    static func bodyLineSpacing(for size: CGFloat) -> CGFloat {
        size * 0.6
    }
}

enum InkMetrics {
    static let rowMinHeight: CGFloat = 44
    static let rowPaddingH: CGFloat = 14
    static let rowPaddingV: CGFloat = 8

    static let cardMarginH: CGFloat = 16
    static let cardMarginBottom: CGFloat = 12
    static let cardPaddingH: CGFloat = 16
    static let cardPaddingV: CGFloat = 14

    static let gridGap: CGFloat = 12
    static let tileMinHeight: CGFloat = 92
    static let recallWidth: CGFloat = 168
    static let markerIconSize: CGFloat = 17

    /// Document gutter for the page width setting: narrow 30, normal 22, wide 14.
    static let gutterNarrow: CGFloat = 30
    static let gutterNormal: CGFloat = 22
    static let gutterWide: CGFloat = 14
}

enum InkRadius {
    static let card: CGFloat = 16
    static let rowGroup: CGFloat = 12
    static let segment: CGFloat = 9
    static let button: CGFloat = 7
    static let cta: CGFloat = 14
    static let ctaSmall: CGFloat = 10
    static let toast: CGFloat = 12
    static let code: CGFloat = 10
    static let alert: CGFloat = 10
    static let table: CGFloat = 8
    static let field: CGFloat = 10
    static let taskBox: CGFloat = 6
    // pill = 999: use Capsule()
}

enum InkMotion {
    /// Push transition, cubic-bezier(.2,.8,.2,1) over .32s.
    static let push = Animation.timingCurve(0.2, 0.8, 0.2, 1, duration: 0.32)
    static let sheet = Animation.easeOut(duration: 0.275)
    static let popover = Animation.easeOut(duration: 0.18)
    static let toast = Animation.easeOut(duration: 0.2)

    /// Returns nil when Reduce Motion is on, so callers skip the animation.
    static func resolved(_ animation: Animation, reduceMotion: Bool) -> Animation? {
        reduceMotion ? nil : animation
    }
}
