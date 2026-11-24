import Foundation
import InkunoCore

// The live editor's Markdown layer, backed by the Rust core (`crates/inkuno-ffi`, UniFFI). The core's
// records use its own names and offsets; this adapter turns them into the editor's types. The core
// module is `InkunoCore` (ios/Generated, built by scripts/build-ios-core.sh). Its types are spelled
// `InkunoCore.X` where the editor has a type of the same name.

struct CoreMarkdownProvider: MarkdownLayoutProvider {

    /// The core's bit of each style, read from the core once. Nothing here assumes the core's numbers.
    private static let coreBits = InkunoCore.styleBits()

    /// (core bit, editor bit) for each style the editor draws. Core styles with no editor bit (highlight,
    /// underline, code colouring, quiet text, checkboxes) are dropped. `date` has no core bit: no run
    /// carries it yet.
    private static let bitMap: [(core: UInt32, editor: UInt32)] = [
        (coreBits.bold, StyleBit.bold),
        (coreBits.italic, StyleBit.italic),
        (coreBits.code, StyleBit.code),
        (coreBits.strike, StyleBit.strike),
        (coreBits.link, StyleBit.link),
        (coreBits.tag, StyleBit.tag),
        // The core dims the syntax characters (`**`, backticks, the brackets and URL of a link).
        (coreBits.dim, StyleBit.marker),
    ]

    func documentLayout(_ text: String) -> [LineLayout] {
        let lines = InkunoCore.documentLayout(text: text).lines
        return lines.enumerated().map { index, line in
            LineLayout(
                startUTF16: Int(line.startUtf16),
                // The core's length leaves out the line break; the editor's counts it, except on the last line.
                lenUTF16: Int(line.lenUtf16) + (index < lines.count - 1 ? 1 : 0),
                kind: LineKind(line.kind),
                headingLevel: Int(line.headingLevel),
                markerLenUTF16: Int(line.markerLenUtf16),
                inCodeBlock: line.inCodeBlock
            )
        }
    }

    func styleRuns(line: String) -> [StyleRun] {
        InkunoCore.lineStyleRuns(line: line).compactMap { run in
            let bits = Self.editorBits(run.bits)
            guard bits != 0 else { return nil }
            return StyleRun(startUTF16: Int(run.startUtf16), lenUTF16: Int(run.lenUtf16), bits: bits)
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

    /// The editor's bits for the core bits of `bits`.
    private static func editorBits(_ bits: UInt32) -> UInt32 {
        bitMap.reduce(into: UInt32(0)) { result, pair in
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
