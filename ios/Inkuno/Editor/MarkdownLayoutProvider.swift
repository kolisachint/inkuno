import Foundation

// The contract between the live editor and the Markdown layer. `CoreMarkdownProvider` implements it on
// the Rust core (`InkunoCore`, UniFFI). Offsets are UTF-16 units.

/// What a source line is, for styling.
enum LineKind: Equatable {
    case para
    case heading
    case bullet
    case ordered
    case task(checked: Bool)
    case quote
    case fence
    case code
    case rule
    case table
    case meta
}

/// One source line. `lenUTF16` counts the trailing "\n", so the lines tile the text: a text that ends
/// in "\n" ends with an empty line, and an empty text has one empty line.
struct LineLayout: Equatable {
    var startUTF16: Int
    var lenUTF16: Int
    var kind: LineKind
    /// 1 to 6 for headings, 0 otherwise.
    var headingLevel = 0
    /// The block marker at the start of the line ("## ", "  - [ ] ", "> ", "```"), counted from the line
    /// start, indentation included. Zero when there is none. For an ordered item this is the number and
    /// its dot; the editor keeps it visible.
    var markerLenUTF16 = 0
    /// True for the fence lines and the code lines between them.
    var inCodeBlock = false
    /// The info string of the block a code line is in ("rust"), empty otherwise. Picks the colouring.
    var codeLang = ""
}

/// Bits of a `StyleRun`. The editor's own bits: `CoreMarkdownProvider` maps the core's bits onto them.
enum StyleBit {
    static let bold: UInt32 = 1 << 0
    static let italic: UInt32 = 1 << 1
    static let code: UInt32 = 1 << 2
    static let strike: UInt32 = 1 << 3
    /// `[text](url)` and `[[wiki]]`: the text part.
    static let link: UInt32 = 1 << 4
    /// `#tag`.
    static let tag: UInt32 = 1 << 5
    /// `@2026-11-24`.
    static let date: UInt32 = 1 << 6
    /// Syntax characters that are hidden off the live line: `**`, backticks, the brackets and URL of a link.
    static let marker: UInt32 = 1 << 7
    /// Text that steps back but is still read: the address of a link. Hidden off the live line like a
    /// marker, except in front matter (`LineKind.meta`).
    static let quiet: UInt32 = 1 << 8
    /// Code colouring, from `codeStyleRuns`: keyword, string, number, comment, type and function.
    static let keyword: UInt32 = 1 << 9
    static let string: UInt32 = 1 << 10
    static let number: UInt32 = 1 << 11
    static let comment: UInt32 = 1 << 12
    static let type: UInt32 = 1 << 13
    static let function: UInt32 = 1 << 14
}

/// A span of one line, relative to the line start. Runs may overlap; their bits add up.
struct StyleRun: Equatable {
    var startUTF16: Int
    var lenUTF16: Int
    var bits: UInt32
}

/// What Enter does on a line.
enum EnterOutcome: Equatable {
    /// Insert a newline, then this text (a list continuation such as "- " or "2. ").
    case insert(String)
    /// The line is an empty list item: remove its block marker and insert no newline.
    case clear
    /// Ordinary Enter.
    case none
}

protocol MarkdownLayoutProvider {
    /// The lines of `text`, in order.
    func documentLayout(_ text: String) -> [LineLayout]
    /// The inline spans of one line, without its newline. Offsets are relative to the line.
    func styleRuns(line: String) -> [StyleRun]
    /// The colouring of one line of a code block in language `lang`.
    func codeStyleRuns(line: String, lang: String) -> [StyleRun]
    /// What Enter does at `colUTF16` (relative to the line start) of `line`.
    func onEnter(line: String, colUTF16: Int, inFence: Bool) -> EnterOutcome
    /// Indents (or outdents) every line the range touches. Returns the new text and the range that
    /// still covers the same lines.
    func indent(_ text: String, rangeUTF16: NSRange, outdent: Bool) -> (newText: String, newRange: NSRange)
}

extension Array where Element == LineLayout {
    /// The index of the line that holds `position`: the last line starting at or before it.
    /// Expects at least one line.
    func lineIndex(containing position: Int) -> Int {
        var low = 0
        var high = count - 1
        while low < high {
            let mid = (low + high + 1) / 2
            if self[mid].startUTF16 <= position {
                low = mid
            } else {
                high = mid - 1
            }
        }
        return low
    }
}

extension Array where Element == LineLayout {
    /// The lines of the table block that holds line `index` (a table line): the run of table lines around it.
    func tableBlock(containing index: Int) -> ClosedRange<Int> {
        var low = index
        var high = index
        while low > 0, self[low - 1].kind == .table {
            low -= 1
        }
        while high < count - 1, self[high + 1].kind == .table {
            high += 1
        }
        return low...high
    }
}
