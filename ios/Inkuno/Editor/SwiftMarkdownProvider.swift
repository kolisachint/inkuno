import Foundation

// ponytail: throwaway fallback until InkunoCore (inkuno-logic over UniFFI) lands. It classifies lines
// and inline spans with regular expressions and knows none of CommonMark's edge cases: no setext
// headings, no indented code, no nested emphasis, no escapes, no HTML. Delete it when the core
// provider is wired in.
struct SwiftMarkdownProvider: MarkdownLayoutProvider {

    private static func regex(_ pattern: String) -> NSRegularExpression {
        // The patterns are constants, so a failure is a programming error caught by the tests.
        try! NSRegularExpression(pattern: pattern)
    }

    // Block patterns. Group 1 is the indentation where a list needs it.
    private static let fenceRE = regex("^ *(`{3,}|~{3,})")
    private static let headingRE = regex("^ *(#{1,6})(?= |$)")
    private static let ruleRE = regex("^ {0,3}([-*_])(?:[ \\t]*\\1){2,}[ \\t]*$")
    private static let taskRE = regex("^( *)([-*+]) \\[[ xX]\\] ")
    private static let bulletRE = regex("^( *)([-*+])(?: |$)")
    private static let orderedRE = regex("^( *)([0-9]{1,9})([.)])(?: |$)")
    private static let quoteRE = regex("^( *)>(?: ?)")
    private static let tableRE = regex("^ *\\|")

    // Inline patterns.
    private static let codeRE = regex("`[^`\\n]+`")
    private static let boldRE = regex("\\*\\*(?=\\S)(.+?)(?<=\\S)\\*\\*")
    private static let boldUnderRE = regex("__(?=\\S)(.+?)(?<=\\S)__")
    private static let italicRE = regex("(?<![*\\w])\\*(?=[^\\s*])([^*\\n]+?)(?<=[^\\s*])\\*(?![*\\w])")
    private static let italicUnderRE = regex("(?<!\\w)_(?=[^\\s_])([^_\\n]+?)(?<=[^\\s_])_(?!\\w)")
    private static let strikeRE = regex("~~(?=\\S)(.+?)(?<=\\S)~~")
    private static let wikiRE = regex("\\[\\[([^\\]\\n]+)\\]\\]")
    private static let linkRE = regex("\\[([^\\]\\n]+)\\]\\(([^)\\n]*)\\)")
    private static let tagRE = regex("(?<!\\S)#[\\p{L}\\p{N}_/-]+")
    private static let dateRE = regex("(?<!\\S)@[0-9]{4}-[0-9]{2}-[0-9]{2}(?!\\S)")

    private static func firstMatch(_ re: NSRegularExpression, _ text: String) -> NSTextCheckingResult? {
        re.firstMatch(in: text, range: NSRange(location: 0, length: text.utf16.count))
    }

    private static func substring(_ text: String, _ range: NSRange) -> String {
        (text as NSString).substring(with: range)
    }

    // MARK: Lines

    private struct Fence {
        var char: String
        var count: Int
    }

    func documentLayout(_ text: String) -> [LineLayout] {
        let parts = text.components(separatedBy: "\n")
        var layouts: [LineLayout] = []
        var fence: Fence?
        var start = 0
        for (index, part) in parts.enumerated() {
            let length = part.utf16.count + (index < parts.count - 1 ? 1 : 0)
            var line = LineLayout(startUTF16: start, lenUTF16: length, kind: .para)
            classify(part, into: &line, fence: &fence)
            layouts.append(line)
            start += length
        }
        return layouts
    }

    private func classify(_ text: String, into line: inout LineLayout, fence: inout Fence?) {
        if let open = fence {
            line.inCodeBlock = true
            if let match = Self.firstMatch(Self.fenceRE, text) {
                let run = Self.substring(text, match.range(at: 1))
                // A closing fence: the same character, at least as long, and nothing else on the line.
                if run.hasPrefix(open.char), run.count >= open.count,
                   text.trimmingCharacters(in: .whitespaces) == run {
                    line.kind = .fence
                    line.markerLenUTF16 = text.utf16.count
                    fence = nil
                    return
                }
            }
            line.kind = .code
            return
        }
        if let match = Self.firstMatch(Self.fenceRE, text) {
            let run = Self.substring(text, match.range(at: 1))
            fence = Fence(char: String(run.prefix(1)), count: run.count)
            line.kind = .fence
            line.inCodeBlock = true
            line.markerLenUTF16 = match.range.length
            return
        }
        if let match = Self.firstMatch(Self.headingRE, text) {
            line.kind = .heading
            line.headingLevel = match.range(at: 1).length
            line.markerLenUTF16 = match.range.length + Self.spaceFollows(text, match.range)
            return
        }
        if Self.firstMatch(Self.ruleRE, text) != nil {
            line.kind = .rule
            return
        }
        if let match = Self.firstMatch(Self.taskRE, text) {
            let marker = Self.substring(text, match.range)
            line.kind = .task(checked: marker.contains("[x]") || marker.contains("[X]"))
            line.markerLenUTF16 = match.range.length
            return
        }
        if let match = Self.firstMatch(Self.bulletRE, text) {
            line.kind = .bullet
            line.markerLenUTF16 = match.range.length
            return
        }
        if let match = Self.firstMatch(Self.orderedRE, text) {
            line.kind = .ordered
            line.markerLenUTF16 = match.range.length
            return
        }
        if let match = Self.firstMatch(Self.quoteRE, text) {
            line.kind = .quote
            line.markerLenUTF16 = match.range.length
            return
        }
        if Self.firstMatch(Self.tableRE, text) != nil {
            line.kind = .table
        }
    }

    /// 1 when the character after `range` is a space, else 0.
    private static func spaceFollows(_ text: String, _ range: NSRange) -> Int {
        let next = NSMaxRange(range)
        guard next < text.utf16.count else { return 0 }
        return (text as NSString).character(at: next) == 32 ? 1 : 0
    }

    // MARK: Inline spans

    func styleRuns(line: String) -> [StyleRun] {
        let length = line.utf16.count
        guard length > 0 else { return [] }
        var flags = [UInt32](repeating: 0, count: length)
        let full = NSRange(location: 0, length: length)

        // Code spans first: nothing else styles inside them.
        for match in Self.codeRE.matches(in: line, range: full) {
            flags.add(match.range, StyleBit.code)
            flags.add(NSRange(location: match.range.location, length: 1), StyleBit.marker)
            flags.add(NSRange(location: match.range.upperBound - 1, length: 1), StyleBit.marker)
        }

        // A delimited span: its whole range gets `bits`, its delimiters get `marker`.
        func wrapped(_ re: NSRegularExpression, delimiter: Int, _ bits: UInt32) {
            for match in re.matches(in: line, range: full) where flags[match.range.location] & StyleBit.code == 0 {
                flags.add(match.range, bits)
                flags.add(NSRange(location: match.range.location, length: delimiter), StyleBit.marker)
                flags.add(NSRange(location: match.range.upperBound - delimiter, length: delimiter), StyleBit.marker)
            }
        }
        wrapped(Self.boldRE, delimiter: 2, StyleBit.bold)
        wrapped(Self.boldUnderRE, delimiter: 2, StyleBit.bold)
        wrapped(Self.italicRE, delimiter: 1, StyleBit.italic)
        wrapped(Self.italicUnderRE, delimiter: 1, StyleBit.italic)
        wrapped(Self.strikeRE, delimiter: 2, StyleBit.strike)
        wrapped(Self.wikiRE, delimiter: 2, StyleBit.link)

        for match in Self.linkRE.matches(in: line, range: full) where flags[match.range.location] & StyleBit.code == 0 {
            let text = match.range(at: 1)
            flags.add(text, StyleBit.link)
            flags.add(NSRange(location: match.range.location, length: 1), StyleBit.marker)
            // The "](url)" part.
            flags.add(NSRange(location: text.upperBound, length: match.range.upperBound - text.upperBound), StyleBit.marker)
        }
        for match in Self.tagRE.matches(in: line, range: full) where flags[match.range.location] & StyleBit.code == 0 {
            flags.add(match.range, StyleBit.tag)
        }
        for match in Self.dateRE.matches(in: line, range: full) where flags[match.range.location] & StyleBit.code == 0 {
            flags.add(match.range, StyleBit.date)
        }

        var runs: [StyleRun] = []
        var index = 0
        while index < length {
            let bits = flags[index]
            var end = index + 1
            while end < length && flags[end] == bits { end += 1 }
            if bits != 0 {
                runs.append(StyleRun(startUTF16: index, lenUTF16: end - index, bits: bits))
            }
            index = end
        }
        return runs
    }

    // MARK: Enter and indent

    /// A list item or quote line: its prefix length and the text Enter continues it with.
    private struct ItemPrefix {
        var length: Int
        var next: String
    }

    private static func itemPrefix(_ line: String) -> ItemPrefix? {
        if let match = firstMatch(taskRE, line) {
            let indent = substring(line, match.range(at: 1))
            let bullet = substring(line, match.range(at: 2))
            return ItemPrefix(length: match.range.length, next: indent + bullet + " [ ] ")
        }
        if let match = firstMatch(bulletRE, line) {
            let indent = substring(line, match.range(at: 1))
            let bullet = substring(line, match.range(at: 2))
            return ItemPrefix(length: match.range.length, next: indent + bullet + " ")
        }
        if let match = firstMatch(orderedRE, line) {
            let indent = substring(line, match.range(at: 1))
            let number = Int(substring(line, match.range(at: 2))) ?? 0
            let delimiter = substring(line, match.range(at: 3))
            return ItemPrefix(length: match.range.length, next: indent + "\(number + 1)" + delimiter + " ")
        }
        if let match = firstMatch(quoteRE, line) {
            let indent = substring(line, match.range(at: 1))
            return ItemPrefix(length: match.range.length, next: indent + "> ")
        }
        return nil
    }

    func onEnter(line: String, colUTF16: Int, inFence: Bool) -> EnterOutcome {
        guard !inFence, let item = Self.itemPrefix(line), colUTF16 >= item.length else { return .none }
        let rest = (line as NSString).substring(from: item.length)
        if rest.trimmingCharacters(in: .whitespaces).isEmpty {
            return .clear
        }
        return .insert(item.next)
    }

    func indent(_ text: String, rangeUTF16 range: NSRange, outdent: Bool) -> (newText: String, newRange: NSRange) {
        let ns = NSString(string: text)
        let block = ns.lineRange(for: range)
        var pieces: [String] = []
        var deltas: [(start: Int, delta: Int)] = []
        var position = block.location
        repeat {
            let lineRange = ns.lineRange(for: NSRange(location: position, length: 0))
            let (line, delta) = Self.indentLine(ns.substring(with: lineRange), outdent: outdent)
            pieces.append(line)
            deltas.append((lineRange.location, delta))
            position = NSMaxRange(lineRange)
        } while position < NSMaxRange(block)

        let newText = ns.substring(to: block.location) + pieces.joined() + ns.substring(from: NSMaxRange(block))

        // Maps an old offset to the new text. A position inside removed indentation stays at the line start.
        func map(_ offset: Int) -> Int {
            var cumulative = 0
            var floor = block.location
            for change in deltas where change.start <= offset {
                floor = change.start + cumulative
                cumulative += change.delta
            }
            return max(floor, offset + cumulative)
        }
        let start = map(range.location)
        let end = map(NSMaxRange(range))
        return (newText, NSRange(location: start, length: end - start))
    }

    /// Four spaces (the desktop Tab) or one tab in, up to four spaces or one tab out. Returns the new line and the length change.
    private static func indentLine(_ line: String, outdent: Bool) -> (String, Int) {
        if !outdent {
            return ("    " + line, 4)
        }
        if line.hasPrefix("\t") {
            return (String(line.dropFirst()), -1)
        }
        let spaces = min(4, line.prefix(while: { $0 == " " }).count)
        return (String(line.dropFirst(spaces)), -spaces)
    }
}

private extension Array where Element == UInt32 {
    mutating func add(_ range: NSRange, _ bits: UInt32) {
        for index in range.location..<range.upperBound {
            self[index] |= bits
        }
    }
}
