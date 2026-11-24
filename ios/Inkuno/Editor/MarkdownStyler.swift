import SwiftUI
import UIKit

/// Styles the lines of a Markdown text storage for the live editor. A pure pass over attributes, so the
/// tests read them without a view. The editor passes only the lines an edit touched and the caret lines.
///
/// Markers are the syntax characters of `StyleBit.marker` and the block markers of the layout. Off the
/// caret line, or with the setting off, they are hidden: 0.01 pt font, clear colour, no background. On the
/// caret line they show in `faint`. A bullet or task marker keeps its first character, clear, with a kern for
/// its slot; `MarkdownLayoutFragment` draws the disc or the box there, so the stored text does not change.
/// A table block off the caret is drawn as a grid (`styleTable`).
struct MarkdownStyler {
    /// Font size of a hidden marker. Zero would collapse the glyph and confuse the caret.
    static let hiddenFontSize: CGFloat = 0.01
    /// Body line height: 1.6 times the font, as in the prototype.
    static let lineHeightMultiple: CGFloat = 1.6
    /// The space between a table's border and its text, on each side: the prototype's cell padding.
    static let tablePadding: CGFloat = 10

    let provider: MarkdownLayoutProvider
    /// Note text size in points (14, 15, 16 or 18).
    let textSize: CGFloat
    /// The Settings toggle "Show Markdown on the live line".
    let showMarkersOnCaretLine: Bool
    /// Resolves the dynamic colours for the current appearance.
    let traits: UITraitCollection

    /// Restyles `lines` (indices into `layouts`) of `storage`, whose string must be the text `layouts` describe.
    /// `caretLine` is the line that holds the caret, if any. `revealedLines` show their markers whatever the
    /// setting (the block menu's Show Markdown).
    func style(_ storage: NSMutableAttributedString, layouts: [LineLayout], lines: [Int], caretLine: Int?,
               revealedLines: Set<Int> = []) {
        let text = NSString(string: storage.string)
        storage.beginEditing()
        var done: Set<Int> = []
        for index in lines where layouts.indices.contains(index) && !done.contains(index) {
            if layouts[index].kind == .table {
                // A table is styled as a whole: its columns are padded to the widest cell of each column.
                let block = layouts.tableBlock(containing: index)
                done.formUnion(block)
                let live = block.contains { revealedLines.contains($0) }
                    || (showMarkersOnCaretLine && caretLine.map { block.contains($0) } == true)
                for line in block {
                    styleLine(layouts[line], text: text, storage: storage, live: live)
                }
                if !live {
                    styleTable(block, layouts: layouts, text: text, storage: storage)
                }
            } else {
                let live = revealedLines.contains(index) || (index == caretLine && showMarkersOnCaretLine)
                styleLine(layouts[index], text: text, storage: storage, live: live)
                done.insert(index)
            }
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

        // A code line takes the core's colouring only; a fence line and the rest of a block take nothing.
        let runs: [StyleRun]
        if line.kind == .code {
            runs = provider.codeStyleRuns(line: lineText, lang: line.codeLang)
        } else if line.inCodeBlock {
            runs = []
        } else {
            runs = provider.styleRuns(line: lineText)
        }

        var bits = [UInt32](repeating: 0, count: length)
        for run in runs {
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
                let character = (lineText as NSString).substring(with: NSRange(location: index, length: 1))
                storage.addAttributes(glyphAttributes(for: line, character: character), range: range)
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
        // ponytail: a table line is clipped at the edge, not wrapped, so its columns keep their alignment.
        paragraph.lineBreakMode = line.kind == .table ? .byClipping : .byWordWrapping
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
        // Quiet text (a link's address) is hidden like a marker. Front matter is read as it is: its
        // syntax and quiet text stay visible, since the core dims a link there too.
        let readAsIs = line.kind == .meta
        let quiet = bits & StyleBit.quiet != 0 && !readAsIs
        let syntax = bits & StyleBit.marker != 0 && !readAsIs
        let isMarker = isBlockMarker || syntax || quiet
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
        if let syntax = syntaxColor(bits) {
            foreground = color(syntax)
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

    /// The colour of a code token, the first of its bits that is set. Comment, keyword, string, number,
    /// type, then function (a token holds one of them).
    private func syntaxColor(_ bits: UInt32) -> Color? {
        if bits & StyleBit.comment != 0 { return InkColor.syntaxComment }
        if bits & StyleBit.keyword != 0 { return InkColor.syntaxKeyword }
        if bits & StyleBit.string != 0 { return InkColor.syntaxString }
        if bits & StyleBit.number != 0 { return InkColor.syntaxNumber }
        if bits & StyleBit.type != 0 { return InkColor.syntaxType }
        if bits & StyleBit.function != 0 { return InkColor.syntaxFunction }
        return nil
    }

    // MARK: Bullet and task glyphs

    /// The attributes of the character of a bullet or task marker while the line is not live: clear, with the
    /// glyph's slot as a kern (the character's own width is taken off), and the `.inkunoGlyph` mark that the
    /// layout fragment draws over it.
    private func glyphAttributes(for line: LineLayout, character: String) -> [NSAttributedString.Key: Any] {
        guard let glyph = MarkdownGlyph(line.kind) else { return [:] }
        let font = lineFont(line).font()
        // The slot of a bullet is one em, of a task box 1.3 em, as in the prototype.
        let slot = textSize * (glyph == .bullet ? 1.0 : 1.3)
        let width = (character as NSString).size(withAttributes: [.font: font]).width
        return [
            .font: font,
            .foregroundColor: UIColor.clear,
            .inkunoGlyph: NSNumber(value: glyph.rawValue),
            .kern: slot - width,
        ]
    }

    // MARK: Tables

    /// A table block off the caret line, drawn as a grid. The delimiter row (`| --- |`) collapses, and the outer
    /// pipes are hidden, with the spaces that follow the leading pipe. Each inner pipe is clear and keeps one
    /// character of width; its kern pads the cell before it to the widest cell of its column, so the columns line
    /// up in the table's monospace font. The header row is semibold. Each row carries a `TableRowMark` that
    /// `MarkdownLayoutFragment` draws the background, the rules and the border from.
    // ponytail: the padding counts visible characters, so a wide character (CJK, emoji) or a smaller inline code
    // span misaligns its column. Lines are clipped at the edge of the text view: a wide table is cut off and
    // cannot be scrolled (a per-block scroll view is later), and cells are not editable in v1 (the caret on the
    // table shows its raw Markdown).
    private func styleTable(_ block: ClosedRange<Int>, layouts: [LineLayout], text: NSString, storage: NSMutableAttributedString) {
        struct Row {
            let index: Int
            /// UTF-16 start of the line in the storage, and the line's units without the line break.
            let start: Int
            let units: [UInt16]
            /// Characters that are hidden markers (`**`, a link's address…), which take no room.
            let hidden: [Bool]
            let pipes: [Int]
            let isDelimiter: Bool
            /// The leading units that take no room: an outer leading pipe and the spaces after it, so the first
            /// cell starts at the margin.
            let lead: Int
            /// Where the spaces before an outer trailing pipe start, so the last cell ends at the border.
            let tail: Int
        }

        var rows: [Row] = []
        for index in block {
            let line = layouts[index]
            let full = NSRange(location: line.startUTF16, length: line.lenUTF16)
            guard NSMaxRange(full) <= text.length else { continue }
            var length = line.lenUTF16
            if length > 0, text.character(at: NSMaxRange(full) - 1) == 10 {
                length -= 1
            }
            let lineText = text.substring(with: NSRange(location: line.startUTF16, length: length))
            let units = Array(lineText.utf16)
            var hidden = [Bool](repeating: false, count: units.count)
            for run in provider.styleRuns(line: lineText) where run.bits & (StyleBit.marker | StyleBit.quiet) != 0 {
                let low = max(0, run.startUTF16)
                let high = min(units.count, run.startUTF16 + run.lenUTF16)
                guard low < high else { continue }
                for unit in low..<high {
                    hidden[unit] = true
                }
            }
            let pipes = units.indices.filter { units[$0] == Self.pipe }
            var lead = 0
            if let first = pipes.first, units[..<first].allSatisfy(Self.isBlank) {
                lead = first + 1
                while lead < units.count, Self.isBlank(units[lead]) {
                    lead += 1
                }
            }
            for unit in 0..<lead {
                hidden[unit] = true
            }
            var tail = units.count
            if let last = pipes.last, units[(last + 1)...].allSatisfy(Self.isBlank) {
                tail = last
                while tail > lead, Self.isBlank(units[tail - 1]) {
                    tail -= 1
                }
                for unit in tail..<last {
                    hidden[unit] = true
                }
            }
            rows.append(Row(
                index: index,
                start: line.startUTF16,
                units: units,
                hidden: hidden,
                pipes: pipes,
                isDelimiter: Self.isDelimiter(lineText),
                lead: lead,
                tail: tail
            ))
        }
        guard !rows.isEmpty else { return }

        let tableFont = lineFont(layouts[block.lowerBound]).font()
        let advance = ("0" as NSString).size(withAttributes: [.font: tableFont]).width
        let clear = UIColor.clear
        let hiddenAttributes: [NSAttributedString.Key: Any] = [
            .font: UIFont.systemFont(ofSize: Self.hiddenFontSize),
            .foregroundColor: clear,
        ]

        /// The cells of a row: the spans between its pipes, the first before the first pipe, the last after the last.
        func spans(_ row: Row) -> [Range<Int>] {
            var result: [Range<Int>] = []
            var start = 0
            for pipe in row.pipes {
                result.append(start..<pipe)
                start = pipe + 1
            }
            result.append(start..<row.units.count)
            return result
        }
        func visibleWidth(_ row: Row, _ span: Range<Int>) -> Int {
            span.filter { !row.hidden[$0] }.count
        }
        /// An outer pipe: nothing but spaces lies between it and the line's edge.
        func isOuter(_ row: Row, pipe: Int, first: Bool) -> Bool {
            let rest = first ? row.units[..<pipe] : row.units[(pipe + 1)...]
            return rest.allSatisfy(Self.isBlank)
        }
        /// The pipe at `column` is an outer pipe, drawn as nothing.
        func isOuterPipe(_ row: Row, column: Int) -> Bool {
            let pipe = row.pipes[column]
            return (column == 0 && isOuter(row, pipe: pipe, first: true))
                || (column == row.pipes.count - 1 && isOuter(row, pipe: pipe, first: false))
        }

        var widths: [Int] = []
        for row in rows where !row.isDelimiter {
            for (column, span) in spans(row).enumerated() {
                if column == widths.count { widths.append(0) }
                widths[column] = max(widths[column], visibleWidth(row, span))
            }
        }
        // The rules span the table: its padded cells, and one character for each inner pipe. A trailing outer pipe
        // leaves a last cell of spaces, which is not counted.
        let innerPipes = rows.map { row in row.pipes.indices.filter { !isOuterPipe(row, column: $0) }.count }.max() ?? 0
        let trailing = rows.contains { row in !row.pipes.isEmpty && isOuterPipe(row, column: row.pipes.count - 1) }
        let ruleUnits = widths.dropLast(trailing ? 1 : 0).reduce(0, +) + innerPipes
        let ruleWidth = CGFloat(ruleUnits) * advance + 2 * Self.tablePadding

        // The header is the first row when the second is the delimiter row.
        let header = rows.count > 1 && rows[1].isDelimiter ? rows[0].index : nil
        let bodyRows = rows.filter { !$0.isDelimiter }
        for row in rows {
            let lineRange = NSRange(location: row.start, length: row.units.count)
            if row.isDelimiter {
                // The whole line, its break too: a collapsed line takes no height.
                let whole = NSRange(location: row.start, length: min(row.units.count + 1, storage.length - row.start))
                storage.addAttributes(hiddenAttributes, range: whole)
                continue
            }
            let mark = TableRowMark(isHeader: row.index == header,
                                    isLast: row.index == bodyRows.last?.index,
                                    width: ruleWidth)
            storage.addAttribute(.inkTableRow, value: mark, range: lineRange)
            // The text is inset from the border on both sides; the paragraph keeps its other attributes.
            let whole = NSRange(location: row.start, length: min(row.units.count + 1, storage.length - row.start))
            let paragraph = (storage.attribute(.paragraphStyle, at: row.start, effectiveRange: nil) as? NSParagraphStyle)?
                .mutableCopy() as? NSMutableParagraphStyle ?? NSMutableParagraphStyle()
            paragraph.firstLineHeadIndent = Self.tablePadding
            paragraph.headIndent = Self.tablePadding
            storage.addAttribute(.paragraphStyle, value: paragraph, range: whole)
            if row.index == header {
                // Semibold, keeping each run's size and the hidden markers hidden.
                var runs: [(NSRange, UIFont)] = []
                storage.enumerateAttribute(.font, in: lineRange) { value, range, _ in
                    if let font = value as? UIFont, font.pointSize >= 0.1 { runs.append((range, font)) }
                }
                for (range, font) in runs {
                    storage.addAttribute(.font, value: UIFont.monospacedSystemFont(ofSize: font.pointSize, weight: .semibold), range: range)
                }
            }
            if row.lead > 0 {
                storage.addAttributes(hiddenAttributes, range: NSRange(location: row.start, length: row.lead))
            }
            if let last = row.pipes.last, row.tail < last {
                storage.addAttributes(hiddenAttributes, range: NSRange(location: row.start + row.tail, length: last - row.tail))
            }

            let cells = spans(row)
            for (column, pipe) in row.pipes.enumerated() {
                let range = NSRange(location: row.start + pipe, length: 1)
                if isOuterPipe(row, column: column) {
                    storage.addAttributes(hiddenAttributes, range: range)
                    continue
                }
                // The pipe after cell `column` takes the cell's padding.
                let padding = column < cells.count ? widths[column] - visibleWidth(row, cells[column]) : 0
                var attributes: [NSAttributedString.Key: Any] = [.font: tableFont, .foregroundColor: clear]
                if padding > 0 {
                    attributes[.kern] = CGFloat(padding) * advance
                }
                storage.addAttributes(attributes, range: range)
            }
        }
    }

    private static let pipe: UInt16 = 124

    /// A space or a tab.
    private static func isBlank(_ unit: UInt16) -> Bool {
        unit == 32 || unit == 9
    }

    /// A delimiter row: dashes, colons, pipes and spaces, with at least one dash.
    private static func isDelimiter(_ line: String) -> Bool {
        let body = line.filter { !" \t|".contains($0) }
        return body.contains("-") && body.allSatisfy { $0 == "-" || $0 == ":" }
    }
}
