import SwiftUI
import UIKit
import XCTest
@testable import Inkuno

final class TokensTests: XCTestCase {
    private let light = UITraitCollection(userInterfaceStyle: .light)
    private let dark = UITraitCollection(userInterfaceStyle: .dark)

    /// Resolves a token to sRGB components for a trait collection.
    private func components(_ color: Color, _ traits: UITraitCollection) -> (r: Double, g: Double, b: Double, a: Double) {
        let resolved = UIColor(color).resolvedColor(with: traits)
        var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
        resolved.getRed(&r, green: &g, blue: &b, alpha: &a)
        return (Double(r) * 255, Double(g) * 255, Double(b) * 255, Double(a))
    }

    private func assertHex(_ color: Color, _ traits: UITraitCollection, _ hex: UInt32, alpha: Double = 1,
                           file: StaticString = #filePath, line: UInt = #line) {
        let c = components(color, traits)
        XCTAssertEqual(c.r.rounded(), Double((hex >> 16) & 0xFF), file: file, line: line)
        XCTAssertEqual(c.g.rounded(), Double((hex >> 8) & 0xFF), file: file, line: line)
        XCTAssertEqual(c.b.rounded(), Double(hex & 0xFF), file: file, line: line)
        XCTAssertEqual(c.a, alpha, accuracy: 0.005, file: file, line: line)
    }

    func testSurfacesResolvePerAppearance() {
        assertHex(InkColor.page, light, 0xe4e5e7)
        assertHex(InkColor.page, dark, 0x0b0b0c)
        assertHex(InkColor.ground, light, 0xeff0f1)
        assertHex(InkColor.ground, dark, 0x141517)
        assertHex(InkColor.sheet, light, 0xffffff)
        assertHex(InkColor.sheet, dark, 0x1c1c1e)
    }

    func testInkAndAccentResolvePerAppearance() {
        assertHex(InkColor.ink, light, 0x1f2225)
        assertHex(InkColor.ink, dark, 0xe6e6e6)
        assertHex(InkColor.accent, light, 0x3a85f7)
        assertHex(InkColor.accent, dark, 0x4b8ff8)
        assertHex(InkColor.link, dark, 0x8ab4f8)
    }

    func testTranslucentWashesKeepTheirAlpha() {
        assertHex(InkColor.line, light, 0x000000, alpha: 0.08)
        assertHex(InkColor.line, dark, 0xffffff, alpha: 0.08)
        assertHex(InkColor.selection, light, 0x3a85f7, alpha: 0.24)
        assertHex(InkColor.selection, dark, 0x4b8ff8, alpha: 0.32)
        assertHex(InkColor.scrim, dark, 0x000000, alpha: 0.45)
        assertHex(InkColor.addWash, light, 0x2ea05a, alpha: 0.15)
        assertHex(InkColor.deleteInk, dark, 0xff9d96)
    }

    func testSoftenedDarkValuesFromSpec() {
        assertHex(InkColor.codeLine, dark, 0x36373a)
        assertHex(InkColor.gutterNumber, dark, 0x868686)
        assertHex(InkColor.quote, dark, 0x676767)
    }

    func testThemePreferenceMapsToColorScheme() {
        XCTAssertEqual(ThemePreference.light.colorScheme, .light)
        XCTAssertEqual(ThemePreference.dark.colorScheme, .dark)
        XCTAssertNil(ThemePreference.system.colorScheme)
    }

    func testTypeScaleAndHeadingRatios() {
        XCTAssertEqual(InkType.headingSize(1, base: 16), 16 * 1.9, accuracy: 0.001)
        XCTAssertEqual(InkType.headingSize(2, base: 16), 16 * 1.45, accuracy: 0.001)
        XCTAssertEqual(InkType.headingSize(4, base: 16), 16, accuracy: 0.001)
        XCTAssertEqual(NoteTextSize.allCases.map { $0.points }, [14, 15, 16, 18])
        XCTAssertEqual(PageWidth.allCases.map { $0.gutter }, [30, 22, 14])
    }

    func testMetricsAndRadiiFromSpec() {
        XCTAssertEqual(InkMetrics.rowMinHeight, 44)
        XCTAssertEqual(InkMetrics.gridGap, 12)
        XCTAssertEqual(InkRadius.card, 16)
        XCTAssertEqual(InkRadius.rowGroup, 12)
        XCTAssertEqual(InkRadius.cta, 14)
    }

    func testMotionIsSkippedUnderReduceMotion() {
        XCTAssertNil(InkMotion.resolved(InkMotion.push, reduceMotion: true))
        XCTAssertNotNil(InkMotion.resolved(InkMotion.push, reduceMotion: false))
    }
}
