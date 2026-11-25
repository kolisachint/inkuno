import Foundation
import InkunoCore

// The live editor's Markdown layer, backed by the Rust core (`crates/inkuno-ffi`, UniFFI). The core's
// records use its own names and offsets; this adapter turns them into the editor's types. The core
// module is `InkunoCore` (ios/Generated, built by scripts/build-ios-core.sh). Its types are spelled
// `InkunoCore.X` where the editor has a type of the same name.

struct CoreMarkdownProvider: MarkdownLayoutProvider {

    /// The core's bit of each style, read from the core once. Nothing here assumes the core's numbers.
    private static let coreBits = InkunoCore.styleBits()

    /// (core bit, editor bit) for each inline style the editor draws. Core styles with no editor bit
    /// (highlight, underline, checkboxes) are dropped.
    private static let bitMap: [(core: UInt32, editor: UInt32)] = [
        (coreBits.bold, StyleBit.bold),
        (coreBits.italic, StyleBit.italic),
        (coreBits.code, StyleBit.code),
        (coreBits.strike, StyleBit.strike),
        (coreBits.link, StyleBit.link),
        (coreBits.tag, StyleBit.tag),
        // The core dims the syntax characters (`**`, backticks, the brackets of a link).
        (coreBits.dim, StyleBit.marker),
        // The core's quiet text: the address of a link. Hidden off the caret line, except in front matter.
        (coreBits.quiet, StyleBit.quiet),
    ]

    /// (core bit, editor bit) for a code line's colouring. Only the colours and the italics of a comment:
    /// the core's code runs also carry `DIM` and `QUIET` on comments, which must not hide them.
    private static let codeBitMap: [(core: UInt32, editor: UInt32)] = [
        (coreBits.keyword, StyleBit.keyword),
        (coreBits.string, StyleBit.string),
        (coreBits.number, StyleBit.number),
        (coreBits.comment, StyleBit.comment),
        (coreBits.typeName, StyleBit.type),
        (coreBits.function, StyleBit.function),
        (coreBits.italic, StyleBit.italic),
    ]

    /// `@2026-11-24`, or a day word after `@`. Not after a word character or a dot, so an address is left alone.
    /// ponytail: the core has no date bit yet. This scan finds the `@` words and the core's date
    /// suggestions accept or refuse them, so the desktop's day rules apply.
    private static let dateWord = try? NSRegularExpression(pattern: "(?<![\\w.\\\\])@[A-Za-z0-9-]+")

    func documentLayout(_ text: String) -> [LineLayout] {
        let lines = InkunoCore.documentLayout(text: text).lines
        let sourceLines = text.components(separatedBy: "\n")
        // The language of the open code block, from its opening fence ("```rust" is "rust").
        var openLang: String?
        return lines.enumerated().map { index, line in
            let kind = LineKind(line.kind)
            // A fence inside a block closes it; the core marks both fences as in the block.
            if kind == .fence, line.inCodeBlock, index < sourceLines.count {
                if openLang == nil {
                    let info = sourceLines[index].drop { $0 == " " || $0 == "`" || $0 == "~" }
                    openLang = info.split(separator: " ").first.map(String.init) ?? ""
                } else {
                    openLang = nil
                }
            }
            return LineLayout(
                startUTF16: Int(line.startUtf16),
                // The core's length leaves out the line break; the editor's counts it, except on the last line.
                lenUTF16: Int(line.lenUtf16) + (index < lines.count - 1 ? 1 : 0),
                kind: kind,
                headingLevel: Int(line.headingLevel),
                markerLenUTF16: Int(line.markerLenUtf16),
                inCodeBlock: line.inCodeBlock,
                codeLang: kind == .code ? (openLang ?? "") : ""
            )
        }
    }

    func styleRuns(line: String) -> [StyleRun] {
        let runs = InkunoCore.lineStyleRuns(line: line).compactMap { run in
            Self.run(run.startUtf16, run.lenUtf16, run.bits, Self.bitMap)
        }
        return runs + Self.dateRuns(in: line)
    }

    func codeStyleRuns(line: String, lang: String) -> [StyleRun] {
        InkunoCore.codeStyleRuns(line: line, lang: lang).compactMap { run in
            Self.run(run.startUtf16, run.lenUtf16, run.bits, Self.codeBitMap)
        }
    }


    func onEnter(line: String, colUTF16: Int, inFence: Bool) -> EnterOutcome {
        switch InkunoCore.onEnter(line: line, colUtf16: UInt64(max(0, colUTF16)), inFence: inFence) {
        case .clear:
            return .clear
        case .insert(let text):
            // The core's text starts with the line break. A plain Enter is left to UIKit.
            guard text.hasPrefix("\n"), text != "\n" else { return .none }
            return .insert(String(text.dropFirst()))
        }
    }

    func indent(_ text: String, rangeUTF16 range: NSRange, outdent: Bool) -> (newText: String, newRange: NSRange) {
        let edit = InkunoCore.indentLines(
            text: text,
            startUtf16: UInt64(range.location),
            endUtf16: UInt64(NSMaxRange(range)),
            outdent: outdent
        )
        let replaced = NSRange(
            location: Int(edit.startUtf16),
            length: Int(edit.endUtf16) - Int(edit.startUtf16)
        )
        let newText = (text as NSString).replacingCharacters(in: replaced, with: edit.replacement)
        let selection = NSRange(
            location: Int(edit.selectionStartUtf16),
            length: Int(edit.selectionEndUtf16) - Int(edit.selectionStartUtf16)
        )
        return (newText, selection)
    }

    private static func run(_ start: UInt64, _ length: UInt64, _ bits: UInt32, _ map: [(core: UInt32, editor: UInt32)]) -> StyleRun? {
        let editor = editorBits(bits, map)
        guard editor != 0 else { return nil }
        return StyleRun(startUTF16: Int(start), lenUTF16: Int(length), bits: editor)
    }

    /// The runs of the `@` words of `line` that the core reads as a day (`dateSuggestions`).
    private static func dateRuns(in line: String) -> [StyleRun] {
        guard line.contains("@"), let dateWord else { return [] }
        let text = line as NSString
        return dateWord.matches(in: line, range: NSRange(location: 0, length: text.length)).compactMap { match in
            let word = text.substring(with: NSRange(location: match.range.location + 1, length: match.range.length - 1))
            guard Self.day(named: word) != nil else { return nil }
            return StyleRun(startUTF16: match.range.location, lenUTF16: match.range.length, bits: StyleBit.date)
        }
    }

    /// The day a `@` word names: a whole day name or an ISO date, the words the styler reads as a date. Nil otherwise.
    /// The core completes prefixes ("mo" for Monday); only a whole word is a day.
    static func day(named word: String) -> InkunoCore.DateSuggestion? {
        InkunoCore.dateSuggestions(query: word, today: CoreDates.ymd(.now)).first { day in
            day.iso == word || day.label.split(separator: ",").first?.lowercased() == word.lowercased()
        }
    }

    /// The editor's bits for the core bits of `bits`, through `map`.
    private static func editorBits(_ bits: UInt32, _ map: [(core: UInt32, editor: UInt32)]) -> UInt32 {
        map.reduce(into: UInt32(0)) { result, pair in
            if bits & pair.core != 0 {
                result |= pair.editor
            }
        }
    }
}

extension LineKind {
    /// The editor's kind of a core line kind. The heading level is carried by `LineLayout.headingLevel`.
    init(_ core: InkunoCore.LineKind) {
        switch core {
        case .para: self = .para
        case .heading: self = .heading
        case .bullet: self = .bullet
        case .ordered: self = .ordered
        case .task(let checked): self = .task(checked: checked)
        case .quote: self = .quote
        case .fence: self = .fence
        case .code: self = .code
        case .rule: self = .rule
        case .table: self = .table
        case .meta: self = .meta
        }
    }
}
