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
    /// On every character of a callout line: a `CalloutMark`.
    static let inkCallout = NSAttributedString.Key("inkuno.callout")
    /// On the first character of a callout's `[!TYPE]` marker off the caret line: the label to draw there (an `NSAttributedString`).
    static let inkCalloutLabel = NSAttributedString.Key("inkuno.calloutLabel")
    /// On every character of a line that holds a comment's passage: an `NSNumber` (true). The line gets a dot in the margin.
    static let inkComment = NSAttributedString.Key("inkuno.comment")
}

/// A line of a callout: a `> [!TYPE]` quote and the quote lines after it. `MarkdownStyler` sets it; `MarkdownLayoutFragment`
/// draws the tinted background and the bar in the kind's colour.
final class CalloutMark: NSObject {
    /// The callout's TYPE, lowercased: `note`, `tip`, `important`, `warning` or `caution`.
    let kind: String
    /// The space between the bar and the text: the prototype's callout padding, in points.
    static let padding: CGFloat = 14

    /// A callout line of the given kind.
    init(kind: String) {
        self.kind = kind
    }

    /// Two marks are equal when they have the same kind, so an unchanged line keeps its attribute.
    override func isEqual(_ object: Any?) -> Bool {
        guard let other = object as? CalloutMark else { return false }
        return other.kind == kind
    }

    /// The hash of the kind, consistent with `isEqual`.
    override var hash: Int {
        kind.hashValue
    }

    /// The colour of a kind, from the prototype's alert colours. An unknown kind is a note.
    static func color(for kind: String) -> Color {
        switch kind {
        case "tip": return InkColor.alertTip
        case "important": return InkColor.alertImportant
        case "warning": return InkColor.alertWarning
        case "caution": return InkColor.alertCaution
        default: return InkColor.alertNote
        }
    }
}

/// Where a row sits in its table, and how wide the table is. `MarkdownStyler` sets it on every row that is drawn
/// (the delimiter row collapses and has none); `MarkdownLayoutFragment` draws the header fill, the rules and the
/// border from it. The table's first row is its header, its last row is `isLast`.
final class TableRowMark: NSObject {
    let isHeader: Bool
    let isLast: Bool
    /// The table's width in points: the rules and the border span it, and the cells scroll within it.
    let width: CGFloat
    /// How far the table's text is scrolled to the left, in points: 0 when it fits, and kept within the table's width.
    let offset: CGFloat

    init(isHeader: Bool, isLast: Bool, width: CGFloat, offset: CGFloat) {
        self.isHeader = isHeader
        self.isLast = isLast
        self.width = width
        self.offset = offset
    }

    override func isEqual(_ object: Any?) -> Bool {
        guard let other = object as? TableRowMark else { return false }
        return other.isHeader == isHeader && other.isLast == isLast && other.width == width && other.offset == offset
    }

    override var hash: Int {
        var hasher = Hasher()
        hasher.combine(isHeader)
        hasher.combine(isLast)
        hasher.combine(width)
        hasher.combine(offset)
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
    /// The diameter of a comment's dot, as in the prototype.
    static let commentDotSize: CGFloat = 7
    /// The editor's right text container inset, where a comment's dot is drawn: the dot never overlaps a full-width line.
    /// `MarkdownEditorController` sets the inset to this width.
    static let commentDotMargin: CGFloat = 14

    /// The width of the text container. A callout's box spans it, and a table's visible part is measured in it.
    private var containerWidth: CGFloat {
        textLayoutManager?.textContainer?.size.width ?? 0
    }

    /// A table row's rule and border reach the table's width, and a scrolled table's text is drawn over the container's
    /// width, which may be wider than the table's box: the surface covers both. A callout's box spans the container, and
    /// a comment's dot sits in the right inset: the surface covers those too.
    override var renderingSurfaceBounds: CGRect {
        var bounds = super.renderingSurfaceBounds
        guard let text = (textElement as? NSTextParagraph)?.attributedString, text.length > 0 else { return bounds }
        if let mark = text.attribute(.inkTableRow, at: 0, effectiveRange: nil) as? TableRowMark {
            let width = max(mark.width, containerWidth) + 2
            bounds = bounds.union(CGRect(x: -layoutFragmentFrame.minX, y: 0, width: width, height: layoutFragmentFrame.height))
        }
        if text.attribute(.inkCallout, at: 0, effectiveRange: nil) != nil {
            bounds = bounds.union(CGRect(x: -layoutFragmentFrame.minX, y: 0, width: containerWidth, height: layoutFragmentFrame.height))
        }
        if text.attribute(.inkComment, at: 0, effectiveRange: nil) != nil {
            bounds = bounds.union(CGRect(x: -layoutFragmentFrame.minX + containerWidth, y: 0,
                                         width: Self.commentDotMargin, height: layoutFragmentFrame.height))
        }
        return bounds
    }

    override func draw(at renderingOrigin: CGPoint, in context: CGContext) {
        guard let paragraph = textElement as? NSTextParagraph,
              let line = textLineFragments.first
        else {
            super.draw(at: renderingOrigin, in: context)
            return
        }
        let text = paragraph.attributedString
        context.saveGState()
        if let callout = text.attribute(.inkCallout, at: 0, effectiveRange: nil) as? CalloutMark {
            drawCallout(callout, origin: renderingOrigin)
        }
        if let mark = text.attribute(.inkTableRow, at: 0, effectiveRange: nil) as? TableRowMark {
            // The box is the table's width, or the container's (less a point, so the border stays inside it) when the
            // table is wider. The text is shifted by the mark's offset and clipped to the box, so the cells scroll in it.
            let fit = containerWidth - 1
            let visible = fit > 0 ? min(mark.width, fit) : mark.width
            drawTableRow(mark, width: visible, origin: renderingOrigin, in: context)
            let offset = min(max(0, mark.offset), max(0, mark.width - visible))
            context.clip(to: CGRect(x: -layoutFragmentFrame.minX, y: renderingOrigin.y, width: visible,
                                    height: layoutFragmentFrame.height))
            context.translateBy(x: -offset, y: 0)
        }
        super.draw(at: renderingOrigin, in: context)
        context.restoreGState()
        if text.attribute(.inkComment, at: 0, effectiveRange: nil) != nil {
            drawCommentDot(line: line, origin: renderingOrigin)
        }

        // The `[!TYPE]` label of a callout's opening line, at the padding; the title follows it (`MarkdownStyler`).
        var label: NSAttributedString?
        text.enumerateAttribute(.inkCalloutLabel, in: NSRange(location: 0, length: text.length)) { value, _, stop in
            guard let found = value as? NSAttributedString else { return }
            label = found
            stop.pointee = true
        }
        if let label {
            let ascent = (label.attribute(.font, at: 0, effectiveRange: nil) as? UIFont)?.ascender ?? 0
            let baseline = renderingOrigin.y + line.typographicBounds.minY + line.glyphOrigin.y
            UIGraphicsPushContext(context)
            defer { UIGraphicsPopContext() }
            label.draw(at: CGPoint(x: renderingOrigin.x + line.typographicBounds.minX + CalloutMark.padding, y: baseline - ascent))
        }

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

    /// The gold dot of a line that holds a comment, vertically centred on the line and centred in the editor's right
    /// inset (`commentDotMargin`), just past the container's right edge.
    private func drawCommentDot(line: NSTextLineFragment, origin: CGPoint) {
        let colour = UIColor(InkColor.commentDot).resolvedColor(with: UITraitCollection.current)
        let size = Self.commentDotSize
        let left = -layoutFragmentFrame.minX + containerWidth + (Self.commentDotMargin - size) / 2
        let centreY = origin.y + line.typographicBounds.midY
        colour.setFill()
        UIBezierPath(ovalIn: CGRect(x: left, y: centreY - size / 2, width: size, height: size)).fill()
    }

    /// A callout line's background, tinted by its kind, and the bar at its left edge, over the width of the text container.
    private func drawCallout(_ mark: CalloutMark, origin: CGPoint) {
        let colour = UIColor(CalloutMark.color(for: mark.kind)).resolvedColor(with: UITraitCollection.current)
        let box = CGRect(x: -layoutFragmentFrame.minX, y: origin.y, width: containerWidth, height: layoutFragmentFrame.height)
        colour.withAlphaComponent(0.1).setFill()
        UIBezierPath(rect: box).fill()
        colour.setFill()
        UIBezierPath(rect: CGRect(x: box.minX, y: box.minY, width: 3, height: box.height)).fill()
    }

    /// A table row's header fill, its rule under it and its part of the border, over the table's visible box of `width`
    /// points. The row's rectangle is clipped, so every row strokes the border's sides, and the header's top and the
    /// last row's bottom close it. The text is drawn by `draw`, shifted and clipped to the same box.
    private func drawTableRow(_ mark: TableRowMark, width: CGFloat, origin: CGPoint, in context: CGContext) {
        let traits = UITraitCollection.current
        func resolved(_ token: Color) -> UIColor { UIColor(token).resolvedColor(with: traits) }

        // The drawing is in the fragment's space: the margin is `-minX`, where the table's box starts.
        let left = -layoutFragmentFrame.minX
        let height = layoutFragmentFrame.height

        // The border is a rounded rectangle that runs past the row where the table goes on, so each row strokes
        // its own sides; the header's top corners and the last row's bottom corners are in its rectangle.
        let top = mark.isHeader ? origin.y + 0.5 : origin.y - 1_000
        let bottom = mark.isLast ? origin.y + height - 0.5 : origin.y + height + 1_000
        let border = UIBezierPath(roundedRect: CGRect(x: left + 0.5, y: top, width: width - 1, height: bottom - top),
                                  cornerRadius: 8)
        let rule = UIBezierPath()
        rule.move(to: CGPoint(x: left, y: origin.y + height - 0.5))
        rule.addLine(to: CGPoint(x: left + width, y: origin.y + height - 0.5))

        context.saveGState()
        context.clip(to: CGRect(x: left, y: origin.y, width: width, height: height))
        if mark.isHeader {
            resolved(InkColor.tableHeader).setFill()
            border.fill()
        }
        resolved(InkColor.tableLine).setStroke()
        border.lineWidth = 1
        border.stroke()
        if !mark.isLast {
            rule.lineWidth = 1
            rule.stroke()
        }
        context.restoreGState()
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
