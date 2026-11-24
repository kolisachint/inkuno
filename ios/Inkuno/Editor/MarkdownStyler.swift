import SwiftUI
import UIKit

/// Styles the lines of a Markdown text storage for the live editor. A pure pass over attributes, so the
/// tests read them without a view. The editor passes only the lines an edit touched and the caret lines.
///
/// Markers are the syntax characters of `StyleBit.marker` and the block markers of the layout. Off the
/// caret line, or with the setting off, they are hidden: 0.01 pt font, clear colour, no background. On the
/// caret line they show in `faint`. A bullet or task marker keeps its first character, drawn as a disc or a
/// box by a text attachment on that character, so the stored text does not change.
struct MarkdownStyler {
    /// Font size of a hidden marker. Zero would collapse the glyph and confuse the caret.
    static let hiddenFontSize: CGFloat = 0.01
    /// Body line height: 1.6 times the font, as in the prototype.
    static let lineHeightMultiple: CGFloat = 1.6

    let provider: MarkdownLayoutProvider
    /// Note text size in points (14, 15, 16 or 18).
    let textSize: CGFloat
    /// The Settings toggle "Show Markdown on the live line".
    let showMarkersOnCaretLine: Bool
    /// Resolves the dynamic colours for the current appearance.
    let traits: UITraitCollection

    /// Restyles `lines` (indices into `layouts`) of `storage`, whose string must be the text `layouts` describe.
    /// `caretLine` is the line that holds the caret, if any.
    func style(_ storage: NSMutableAttributedString, layouts: [LineLayout], lines: [Int], caretLine: Int?) {
        let text = NSString(string: storage.string)
        storage.beginEditing()
        for index in lines where layouts.indices.contains(index) {
            let live = index == caretLine && showMarkersOnCaretLine
            styleLine(layouts[index], text: text, storage: storage, live: live)
        }
        storage.endEditing()
    }

    // MARK: One line

    private func styleLine(_ line: LineLayout, text: NSString, storage: NSMutableAttributedString, live: Bool) {
        let lineRange = NSRange(location: line.startUTF16, length: line.lenUTF16)
        guard NSMaxRange(lineRange) <= text.length else { return }
        storage.setAttributes(baseAttributes(for: line), range: lineRange)

        var length = line.lenUTF16
        if length > 0, text.character(at: NSMaxRange(lineRange) - 1) == 10 {
            length -= 1
        }
        guard length > 0 else { return }
        let lineText = text.substring(with: NSRange(location: line.startUTF16, length: length))

        var bits = [UInt32](repeating: 0, count: length)
        for run in provider.styleRuns(line: lineText) {
            let low = max(0, run.startUTF16)
            let high = min(length, run.startUTF16 + run.lenUTF16)
            guard low < high else { continue }
            for index in low..<high {
                bits[index] |= run.bits
            }
        }

        let block = blockMarkerRange(line, lineText: lineText, length: length)
        let isListMarker = block.lowerBound < block.upperBound && isListKind(line.kind)
        let glyph = isListMarker ? block.lowerBound : -1
        if case .task(checked: true) = line.kind {
            for index in block.upperBound..<length {
                bits[index] |= StyleBit.strike
            }
        }

        var index = 0
        while index < length {
            let isBlock = block.contains(index)
            if index == glyph && !live {
                let range = NSRange(location: line.startUTF16 + index, length: 1)
                storage.addAttributes(glyphAttributes(for: line), range: range)
                index += 1
                continue
            }
            var end = index + 1
            while end < length, end != glyph, block.contains(end) == isBlock, bits[end] == bits[index] {
                end += 1
            }
            let attributes = self.attributes(bits: bits[index], isBlockMarker: isBlock, line: line, live: live)
            storage.addAttributes(attributes, range: NSRange(location: line.startUTF16 + index, length: end - index))
            index = end
        }
    }

    /// The block marker characters of the line, relative to the line start. A list item keeps its
    /// indentation visible, so the nesting shows.
    private func blockMarkerRange(_ line: LineLayout, lineText: String, length: Int) -> Range<Int> {
        switch line.kind {
        case .heading, .quote, .fence:
            return 0..<min(line.markerLenUTF16, length)
        case .bullet, .task:
            let indent = lineText.prefix { $0 == " " }.utf16.count
            return indent..<max(indent, min(line.markerLenUTF16, length))
        default:
            return 0..<0
        }
    }

    private func isListKind(_ kind: LineKind) -> Bool {
        switch kind {
        case .bullet, .task: return true
        default: return false
        }
    }

    // MARK: Attributes

    private struct FontSpec {
        var size: CGFloat
        var weight: UIFont.Weight = .regular
        var mono = false
        var italic = false

        func font() -> UIFont {
            var font = mono
                ? UIFont.monospacedSystemFont(ofSize: size, weight: weight)
                : UIFont.systemFont(ofSize: size, weight: weight)
            if italic {
                let traits = font.fontDescriptor.symbolicTraits.union(.traitItalic)
                if let descriptor = font.fontDescriptor.withSymbolicTraits(traits) {
                    font = UIFont(descriptor: descriptor, size: size)
                }
            }
            return font
        }
    }

    private func lineFont(_ line: LineLayout) -> FontSpec {
        switch line.kind {
        case .heading:
            let size = InkType.headingSize(line.headingLevel, base: textSize)
            return FontSpec(size: size, weight: line.headingLevel == 1 ? .bold : .semibold)
        case .fence, .code:
            return FontSpec(size: textSize * 0.82, mono: true)
        case .table:
            return FontSpec(size: textSize * 0.92, mono: true)
        default:
            return FontSpec(size: textSize)
        }
    }

    private func color(_ token: Color) -> UIColor {
        UIColor(token).resolvedColor(with: traits)
    }

    private func lineColor(_ line: LineLayout) -> UIColor {
        switch line.kind {
        case .quote, .meta, .task(checked: true): return color(InkColor.soft)
        case .rule: return color(InkColor.faint)
        default: return color(InkColor.ink)
        }
    }

    /// Attributes for a whole line before its spans are styled: font, colour, paragraph, code background.
    private func baseAttributes(for line: LineLayout) -> [NSAttributedString.Key: Any] {
        let paragraph = NSMutableParagraphStyle()
        paragraph.lineHeightMultiple = Self.lineHeightMultiple
        var attributes: [NSAttributedString.Key: Any] = [
            .font: lineFont(line).font(),
            .foregroundColor: lineColor(line),
            .paragraphStyle: paragraph,
        ]
        if line.kind == .fence || line.kind == .code {
            attributes[.backgroundColor] = color(InkColor.codeBackground)
        }
        return attributes
    }

    /// Attributes for one run of characters that share their style bits and block-marker state.
    private func attributes(bits: UInt32, isBlockMarker: Bool, line: LineLayout, live: Bool) -> [NSAttributedString.Key: Any] {
        let isMarker = isBlockMarker || bits & StyleBit.marker != 0
        if isMarker && !live {
            return [.font: UIFont.systemFont(ofSize: Self.hiddenFontSize), .foregroundColor: UIColor.clear]
        }

        var spec = lineFont(line)
        var foreground = lineColor(line)
        var extra: [NSAttributedString.Key: Any] = [:]
        if bits & StyleBit.code != 0 && line.kind != .fence && line.kind != .code && line.kind != .table {
            spec.size *= 0.86
            spec.mono = true
            extra[.backgroundColor] = color(InkColor.codeBackground)
        }
        if bits & StyleBit.bold != 0 {
            spec.weight = .bold
        }
        if bits & StyleBit.italic != 0 {
            spec.italic = true
        }
        if bits & StyleBit.link != 0 {
            foreground = color(InkColor.link)
            extra[.underlineStyle] = NSUnderlineStyle.single.rawValue
        }
        if bits & StyleBit.tag != 0 {
            extra[.backgroundColor] = color(InkColor.chip)
        }
        if bits & StyleBit.date != 0 {
            foreground = color(InkColor.accent)
        }
        if bits & StyleBit.strike != 0 {
            extra[.strikethroughStyle] = NSUnderlineStyle.single.rawValue
        }
        if isMarker {
            foreground = color(InkColor.faint)
        }
        extra[.font] = spec.font()
        extra[.foregroundColor] = foreground
        return extra
    }

    // MARK: Bullet and task glyphs

    /// The character of a bullet or task marker while the line is not live: clear text, with an attachment
    /// that draws the disc or the box.
    private func glyphAttributes(for line: LineLayout) -> [NSAttributedString.Key: Any] {
        let attachment = NSTextAttachment()
        let slot = textSize * (line.kind == .bullet ? 1.0 : 1.3)
        let image = glyphImage(for: line.kind, width: slot)
        attachment.image = image
        // The image sits from 0.2 em below the baseline to 0.8 em above it.
        attachment.bounds = CGRect(x: 0, y: -0.2 * textSize, width: slot, height: textSize)
        return [
            .font: lineFont(line).font(),
            .foregroundColor: UIColor.clear,
            .attachment: attachment,
        ]
    }

    private func glyphImage(for kind: LineKind, width: CGFloat) -> UIImage {
        let size = CGSize(width: width, height: textSize)
        let ink = color(InkColor.ink)
        let accent = color(InkColor.accent)
        let soft = color(InkColor.soft)
        let white = UIColor.white
        let renderer = UIGraphicsImageRenderer(size: size)
        return renderer.image { _ in
            if kind == .bullet {
                // The centre sits 0.26 em above the baseline, which is 0.54 em from the top of the image.
                let diameter = textSize * 0.36
                let rect = CGRect(x: (width - diameter) / 2, y: textSize * 0.54 - diameter / 2,
                                  width: diameter, height: diameter)
                ink.setFill()
                UIBezierPath(ovalIn: rect).fill()
                return
            }
            guard case .task(let checked) = kind else { return }
            // The box is centred 0.36 em above the baseline, which is 0.44 em from the top.
            let side = textSize * 0.85
            let box = CGRect(x: 0, y: textSize * 0.44 - side / 2, width: side, height: side)
            let corner = UIBezierPath(roundedRect: box, cornerRadius: textSize * 0.15)
            if checked {
                accent.setFill()
                corner.fill()
                let tick = UIBezierPath()
                tick.move(to: CGPoint(x: box.minX + side * 0.24, y: box.minY + side * 0.52))
                tick.addLine(to: CGPoint(x: box.minX + side * 0.42, y: box.minY + side * 0.70))
                tick.addLine(to: CGPoint(x: box.minX + side * 0.76, y: box.minY + side * 0.30))
                white.setStroke()
                tick.lineWidth = max(1, side * 0.12)
                tick.lineCapStyle = .round
                tick.lineJoinStyle = .round
                tick.stroke()
            } else {
                soft.setStroke()
                corner.lineWidth = 1.2
                corner.stroke()
            }
        }
    }
}
