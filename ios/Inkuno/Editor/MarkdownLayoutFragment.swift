import SwiftUI
import UIKit

// The bullet and task glyphs of the live editor. The stored character ("-", "*", "+") stays, and is clear
// off the caret line; the glyph is drawn over its slot by the layout fragment of its paragraph. A text
// attachment would not do: TextKit 2 draws an attachment only on the U+FFFC character, not on a dash.

/// The glyph of a bullet or task line. Its raw value is the attribute value of `.inkunoGlyph`.
enum MarkdownGlyph: Int {
    case bullet = 1
    case openTask = 2
    case checkedTask = 3

    /// The glyph of a list line, or nil for the other kinds.
    init?(_ kind: LineKind) {
        switch kind {
        case .bullet: self = .bullet
        case .task(checked: false): self = .openTask
        case .task(checked: true): self = .checkedTask
        default: return nil
        }
    }
}

extension NSAttributedString.Key {
    /// On the characters of a table row off the caret: a `TableRowMark`.
    static let inkTableRow = NSAttributedString.Key("inkuno.tableRow")
    /// On the character of a bullet or task glyph: the `MarkdownGlyph` raw value (an `NSNumber`).
    static let inkunoGlyph = NSAttributedString.Key("inkuno.glyph")
}

/// Where a row sits in its table, and how wide the table is. `MarkdownStyler` sets it on every row that is drawn
/// (the delimiter row collapses and has none); `MarkdownLayoutFragment` draws the header fill, the rules and the
/// border from it. The table's first row is its header, its last row is `isLast`.
final class TableRowMark: NSObject {
    let isHeader: Bool
    let isLast: Bool
    /// The table's width in points: the rules and the border span it.
    let width: CGFloat

    init(isHeader: Bool, isLast: Bool, width: CGFloat) {
        self.isHeader = isHeader
        self.isLast = isLast
        self.width = width
    }

    override func isEqual(_ object: Any?) -> Bool {
        guard let other = object as? TableRowMark else { return false }
        return other.isHeader == isHeader && other.isLast == isLast && other.width == width
    }

    override var hash: Int {
        var hasher = Hasher()
        hasher.combine(isHeader)
        hasher.combine(isLast)
        hasher.combine(width)
        return hasher.finalize()
    }
}

/// Hands every paragraph to `MarkdownLayoutFragment`, which draws the glyphs.
final class MarkdownLayoutDelegate: NSObject, NSTextLayoutManagerDelegate {
    func textLayoutManager(_ textLayoutManager: NSTextLayoutManager,
                           textLayoutFragmentFor location: NSTextLocation,
                           in textElement: NSTextElement) -> NSTextLayoutFragment {
        MarkdownLayoutFragment(textElement: textElement, range: textElement.elementRange)
    }
}

/// A paragraph's layout fragment that draws the bullet disc or the task box over the glyph character.
/// The glyph's slot is the kern after the character (`MarkdownStyler`), so the text starts after it.
final class MarkdownLayoutFragment: NSTextLayoutFragment {
    override func draw(at renderingOrigin: CGPoint, in context: CGContext) {
        guard let paragraph = textElement as? NSTextParagraph,
              let line = textLineFragments.first
        else {
            super.draw(at: renderingOrigin, in: context)
            return
        }
        let text = paragraph.attributedString
        context.saveGState()
        if let mark = text.attribute(.inkTableRow, at: 0, effectiveRange: nil) as? TableRowMark {
            drawTableRow(mark, text: text, line: line, origin: renderingOrigin, in: context)
        }
        super.draw(at: renderingOrigin, in: context)
        context.restoreGState()

        var found: (index: Int, glyph: MarkdownGlyph)?
        text.enumerateAttribute(.inkunoGlyph, in: NSRange(location: 0, length: text.length)) { value, range, stop in
            guard let raw = (value as? NSNumber)?.intValue, let glyph = MarkdownGlyph(rawValue: raw) else { return }
            found = (range.location, glyph)
            stop.pointee = true
        }
        guard let found, let font = text.attribute(.font, at: found.index, effectiveRange: nil) as? UIFont else { return }

        // The glyph's x: the width of the indentation before it. Its baseline: the line fragment's.
        let x = renderingOrigin.x + line.typographicBounds.minX
            + text.attributedSubstring(from: NSRange(location: 0, length: found.index)).size().width
        let baseline = renderingOrigin.y + line.typographicBounds.minY + line.glyphOrigin.y
        UIGraphicsPushContext(context)
        defer { UIGraphicsPopContext() }
        Self.draw(found.glyph, font: font, x: x, baseline: baseline)
    }

    /// A table row's header fill, its rule under it and, with the row's place in the table, its part of the
    /// border. The row's rectangle is clipped, so the border's rounded corners and the sides of a row are drawn
    /// by every row alike; the top of the header and the bottom of the last row close the border.
    /// Text past the edge of the text container is cut before the cell that does not fit, never inside a glyph.
    // ponytail: a table wider than the container is cut and cannot be scrolled; a per-block scroll view is later.
    private func drawTableRow(_ mark: TableRowMark, text: NSAttributedString, line: NSTextLineFragment,
                              origin: CGPoint, in context: CGContext) {
        let traits = UITraitCollection.current
        func resolved(_ token: Color) -> UIColor { UIColor(token).resolvedColor(with: traits) }

        let left = origin.x + line.typographicBounds.minX
        let height = layoutFragmentFrame.height
        var width = mark.width
        if let container = textLayoutManager?.textContainer, let advance = Self.cellAdvance(text) {
            let room = container.size.width - left
            if width > room {
                let cut = left + floor(room / advance) * advance
                // Text is clipped to the cells that fit; the border closes at the last of them.
                context.clip(to: CGRect(x: -100_000, y: origin.y - 100_000, width: cut + 200_000, height: 200_000))
                width = cut - left
            }
        }

        let top = mark.isHeader ? origin.y + 0.5 : origin.y - 1_000
        let bottom = mark.isLast ? origin.y + height - 0.5 : origin.y + height + 1_000
        let frame = CGRect(x: left + 0.5, y: top, width: width - 1, height: bottom - top)
        let rounded = UIBezierPath(roundedRect: frame, cornerRadius: 8)

        context.saveGState()
        context.clip(to: CGRect(x: left, y: origin.y, width: width, height: height))
        if mark.isHeader {
            resolved(InkColor.tableHeader).setFill()
            rounded.fill()
        }
        resolved(InkColor.tableLine).setStroke()
        rounded.lineWidth = 1
        rounded.stroke()
        if !mark.isLast {
            let rule = UIBezierPath()
            rule.move(to: CGPoint(x: left, y: origin.y + height - 0.5))
            rule.addLine(to: CGPoint(x: left + width, y: origin.y + height - 0.5))
            rule.lineWidth = 1
            resolved(InkColor.tableLine).setStroke()
            rule.stroke()
        }
        context.restoreGState()
    }

    /// The advance of a monospace character in the text's first sized font, which a table row is set in.
    private static func cellAdvance(_ text: NSAttributedString) -> CGFloat? {
        var font: UIFont?
        text.enumerateAttribute(.font, in: NSRange(location: 0, length: text.length)) { value, _, stop in
            guard let found = value as? UIFont, found.pointSize >= 0.1 else { return }
            font = found
            stop.pointee = true
        }
        return font.map { ("0" as NSString).size(withAttributes: [.font: $0]).width }
    }

    /// The disc or box at `x` on `baseline`, in the size of `font`. The disc is centred in its slot, the
    /// box sits at the slot's left edge, as in the prototype.
    private static func draw(_ glyph: MarkdownGlyph, font: UIFont, x: CGFloat, baseline: CGFloat) {
        let size = font.pointSize
        let traits = UITraitCollection.current
        func resolved(_ token: Color) -> UIColor { UIColor(token).resolvedColor(with: traits) }

        switch glyph {
        case .bullet:
            let slot = size
            let diameter = size * 0.36
            let centre = CGPoint(x: x + slot / 2, y: baseline - size * 0.26)
            resolved(InkColor.ink).setFill()
            UIBezierPath(ovalIn: CGRect(x: centre.x - diameter / 2, y: centre.y - diameter / 2,
                                        width: diameter, height: diameter)).fill()
        case .openTask, .checkedTask:
            let side = size * 0.85
            let box = CGRect(x: x, y: baseline - size * 0.36 - side / 2, width: side, height: side)
            let corner = UIBezierPath(roundedRect: box, cornerRadius: size * 0.15)
            guard glyph == .checkedTask else {
                resolved(InkColor.soft).setStroke()
                corner.lineWidth = 1.2
                corner.stroke()
                return
            }
            resolved(InkColor.accent).setFill()
            corner.fill()
            let tick = UIBezierPath()
            tick.move(to: CGPoint(x: box.minX + side * 0.24, y: box.minY + side * 0.52))
            tick.addLine(to: CGPoint(x: box.minX + side * 0.42, y: box.minY + side * 0.70))
            tick.addLine(to: CGPoint(x: box.minX + side * 0.76, y: box.minY + side * 0.30))
            UIColor.white.setStroke()
            tick.lineWidth = max(1, side * 0.12)
            tick.lineCapStyle = .round
            tick.lineJoinStyle = .round
            tick.stroke()
        }
    }
}
