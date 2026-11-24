import Foundation

// MARK: - Desktop formats, mirrored in Swift
//
// The on-disk formats below are the desktop's, not new ones. Each section names the Rust source
// it mirrors; the paths are relative to the repo root.
// ponytail: the Rust core (InkunoCore via UniFFI) will replace this parsing. Until then the Swift
// mirrors stay deliberately small: header-only reads, first-occurrence placement, no code-span or
// table-cell rules.

// MARK: Revisions — crates/inkuno-logic/src/revision.rs, crates/inkuno-platform/src/revisions.rs
//
// A version is `<vault>/.inkuno/revisions/<note-rel-path>/NNNN.md`, e.g.
// `.inkuno/revisions/ideas/plan.md/0003.md` (revision.rs `folder`, revisions.rs `save`).
// The file opens with a front matter block (`revision.rs` `render`):
//
//     ---
//     revision: 3
//     parent: 2
//     author: sachin
//     date: 2026-11-14T10:32:00Z
//     kind: checkpoint
//     note: dropped the plugin idea
//     ---
//     <the note text, byte for byte>

enum RevisionKind: String {
    case checkpoint
    case auto

    var rowWord: String { rawValue }
}

/// The front matter of one revision file (`revision.rs` `Header`).
struct RevisionHeader: Equatable {
    var number: UInt32
    var parent: UInt32?
    var author: String
    var date: String
    var kind: RevisionKind
    var note: String?
}

enum RevisionFormatError: Error {
    case noHeader
    case unclosed
    case field(String)
}

enum RevisionFormat {
    static let fence = "---"

    /// `revision.rs` `file_name`: `0001.md`, wider past 9999.
    static func fileName(_ number: UInt32) -> String {
        String(format: "%04d.md", number)
    }

    /// `revision.rs` `number_of`: `0003.md` is 3, anything else nil.
    static func number(of name: String) -> UInt32? {
        guard name.hasSuffix(".md") else { return nil }
        let digits = name.dropLast(3)
        guard !digits.isEmpty, digits.allSatisfy(\.isNumber),
              let n = UInt32(digits), n >= 1 else { return nil }
        return n
    }

    /// `revision.rs` `next_number`.
    static func nextNumber(_ saved: some Sequence<UInt32>) -> UInt32 {
        (saved.max() ?? 0) + 1
    }

    /// `revision.rs` `folder`: the same path under `root/dir`, so `ideas/plan.md` maps to
    /// `<root>/.inkuno/revisions/ideas/plan.md`. Nil when the note is not inside `root`, or when
    /// what follows climbs out of `root/dir` (a `..`, absolute or prefix component).
    static func folder(root: URL, dir: String, note: URL) -> URL? {
        let rootPath = root.standardized.path
        let notePath = note.standardized.path
        guard notePath.hasPrefix(rootPath + "/") else { return nil }
        let rel = String(notePath.dropFirst(rootPath.count + 1))
        guard !rel.isEmpty else { return nil }
        let parts = rel.split(separator: "/", omittingEmptySubsequences: false).map(String.init)
        guard parts.allSatisfy({ $0 != ".." && $0 != "." && !$0.hasPrefix("/") && !$0.isEmpty }) else { return nil }
        var out = root.appendingPathComponent(dir, isDirectory: true)
        for part in parts { out.appendPathComponent(part, isDirectory: false) }
        return out
    }

    /// A front matter value, written plain when it reads back as it is, else quoted with escapes
    /// (`revision.rs` `encode`).
    static func encode(_ value: String) -> String {
        let plain = !value.isEmpty && value.trimmingCharacters(in: .whitespacesAndNewlines) == value
            && !value.hasPrefix("\"") && !value.unicodeScalars.contains(where: { $0.properties.generalCategory == .control })
        if plain { return value }
        var out = "\""
        for scalar in value.unicodeScalars {
            switch scalar {
            case "\\": out += "\\\\"
            case "\"": out += "\\\""
            case "\n": out += "\\n"
            case "\r": out += "\\r"
            case "\t": out += "\\t"
            default:
                if scalar.properties.generalCategory == .control {
                    out += String(format: "\\u%04x", scalar.value)
                } else {
                    out.unicodeScalars.append(scalar)
                }
            }
        }
        out += "\""
        return out
    }

    /// `revision.rs` `decode`: a quoted value is unescaped, anything else taken as it is.
    static func decode(_ raw: String) -> String {
        guard raw.hasPrefix("\""), raw.hasSuffix("\""), raw.count >= 2 else { return raw }
        let inner = raw.dropFirst().dropLast()
        var out = ""
        var chars = inner.makeIterator()
        while let c = chars.next() {
            if c == "\\" {
                guard let e = chars.next() else { return raw }
                switch e {
                case "\\": out += "\\"
                case "\"": out += "\""
                case "n": out += "\n"
                case "r": out += "\r"
                case "t": out += "\t"
                case "u":
                    let hex = String([chars.next(), chars.next(), chars.next(), chars.next()].compactMap { $0 })
                    guard hex.count == 4, let code = UInt32(hex, radix: 16),
                          let scalar = Unicode.Scalar(code) else { return raw }
                    out.unicodeScalars.append(scalar)
                default: return raw
                }
            } else if c == "\"" {
                return raw
            } else {
                out.append(c)
            }
        }
        return out
    }

    /// `revision.rs` `render`.
    static func render(number: UInt32, parent: UInt32?, author: String, date: String, kind: RevisionKind, note: String?, text: String) -> String {
        var out = "---\nrevision: \(number)\n"
        if let parent { out += "parent: \(parent)\n" }
        out += "author: \(encode(author))\n"
        out += "date: \(encode(date))\n"
        out += "kind: \(kind.rawValue)\n"
        if let note, !note.isEmpty { out += "note: \(encode(note))\n" }
        out += "---\n"
        out += text
        return out
    }

    private static func bare(_ line: Substring) -> String {
        var s = line
        if s.hasSuffix("\n") { s = s.dropLast() }
        if s.hasSuffix("\r") { s = s.dropLast() }
        return String(s)
    }

    /// `revision.rs` `parse_header`: the front matter and the byte offset where the text starts.
    /// Only the head is read; the text is never looked at.
    static func parseHeader(_ file: String) -> Result<(header: RevisionHeader, textStart: String.Index), RevisionFormatError> {
        var rest = file[...]
        guard let firstLine = rest.firstLine() else { return .failure(.noHeader) }
        guard bare(firstLine) == fence else { return .failure(.noHeader) }
        rest = rest.dropFirst(firstLine.count)
        var fields: [(String, String)] = []
        var closed = false
        while let line = rest.firstLine() {
            rest = rest.dropFirst(line.count)
            if bare(line) == fence { closed = true; break }
            if let colon = line.firstIndex(of: ":") {
                fields.append((String(line[..<colon]).trimmingCharacters(in: .whitespacesAndNewlines),
                               String(line[line.index(after: colon)...]).trimmingCharacters(in: .whitespacesAndNewlines)))
            }
        }
        guard closed else { return .failure(.unclosed) }
        func get(_ key: String) -> String? {
            fields.first(where: { $0.0 == key })?.1
        }
        guard let rawNumber = get("revision"), let number = UInt32(rawNumber), number >= 1 else {
            return .failure(.field("revision"))
        }
        var parent: UInt32?
        if let raw = get("parent") {
            guard let p = UInt32(raw), p >= 1 else { return .failure(.field("parent")) }
            parent = p
        }
        let kind: RevisionKind
        if let raw = get("kind") {
            guard let k = RevisionKind(rawValue: raw) else { return .failure(.field("kind")) }
            kind = k
        } else {
            kind = .checkpoint
        }
        let author = get("author").map(decode).flatMap { $0.isEmpty ? nil : $0 } ?? "unknown"
        let date = get("date").map(decode) ?? ""
        let note = get("note").map(decode).flatMap { $0.isEmpty ? nil : $0 }
        let header = RevisionHeader(number: number, parent: parent, author: author, date: date, kind: kind, note: note)
        return .success((header, rest.startIndex))
    }

    /// `revision.rs` `parse`.
    static func parse(_ file: String) -> Result<(header: RevisionHeader, text: String), RevisionFormatError> {
        parseHeader(file).map { (header, at) in (header, String(file[at...])) }
    }

    /// `revision.rs` `row_name`: `#12 · sachin`, then `· auto` and `· <note>`.
    static func rowName(_ header: RevisionHeader, auto autoWord: String) -> String {
        var name = "#\(header.number) · \(header.author)"
        if header.kind == .auto { name += " · \(autoWord)" }
        if let note = header.note, !note.isEmpty { name += " · \(note)" }
        return name
    }
}

private extension Substring {
    /// The first line including its break, or nil when empty.
    func firstLine() -> Substring? {
        guard !isEmpty else { return nil }
        if let nl = firstIndex(of: "\n") { return self[...nl] }
        return self
    }
}

// MARK: Pending changes — crates/inkuno-platform/src/sidecar.rs
//
// A note's file holds its own text only; its pending changes wait in
// `<vault>/.inkuno/pending/<note-rel-path>.json` (`sidecar.rs`), e.g.
// `ideas/plan.md` maps to `.inkuno/pending/ideas/plan.md.json`:
//
//     {"version": 2, "changes": [{"id": 1, "author": "assistant", "at": 7,
//       "old": "old\n", "new": "new\n", "before": "a\n", "after": "b\n",
//       "orphan_since": 1700000000}]}

/// One pending change kept beside its note (`inkuno-logic` `pending.rs` `Entry`).
struct PendingChange: Equatable {
    var id: UInt32
    var author: String
    /// Byte offset hint of `old` in the note; the context lines place it, not this number.
    var at: Int
    var old: String
    var new: String
    var before: String
    var after: String
    var orphanSince: UInt64?
}

enum PendingList {
    /// `sidecar.rs` `read`: a missing or unreadable list is empty. Lists without `before`/`after`
    /// (version 1) read with empty context; `orphan_since` defaults to nil.
    static func parse(_ data: Data) -> [PendingChange] {
        guard let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let list = json["changes"] as? [[String: Any]] else { return [] }
        return list.compactMap { change in
            guard let id = (change["id"] as? NSNumber).map({ $0.uint32Value }),
                  let author = change["author"] as? String,
                  let at = (change["at"] as? NSNumber)?.intValue,
                  let old = change["old"] as? String,
                  let new = change["new"] as? String else { return nil }
            return PendingChange(
                id: id, author: author, at: at, old: old, new: new,
                before: change["before"] as? String ?? "",
                after: change["after"] as? String ?? "",
                orphanSince: (change["orphan_since"] as? NSNumber)?.uint64Value
            )
        }
    }

    /// `sidecar.rs` `store`: `{"version": 2, "changes": […]}` with a trailing newline. No file is
    /// written for an empty list; the caller removes it.
    static func encode(_ entries: [PendingChange]) -> Data {
        let changes = entries.map { entry -> [String: Any] in
            var change: [String: Any] = [
                "id": entry.id, "author": entry.author, "at": entry.at,
                "old": entry.old, "new": entry.new,
                "before": entry.before, "after": entry.after,
            ]
            if let stamp = entry.orphanSince { change["orphan_since"] = stamp }
            return change
        }
        let json: [String: Any] = ["version": 2, "changes": changes]
        let data = (try? JSONSerialization.data(withJSONObject: json, options: [.prettyPrinted, .sortedKeys])) ?? Data()
        return data + Data("\n".utf8)
    }

    /// Accept one change into `text`: `old` becomes `new`. The entry is placed by its context
    /// lines (`before`/`after`), nearest to `at`; without context the first occurrence wins.
    /// Nil when `old` is not found, so the caller keeps the entry.
    /// ponytail: the full placement rule (`pending.rs` `place`: line starts outside code fences,
    /// blank-context insertions pinned to `at`) lives in the Rust core; this is first-occurrence
    /// with a context check, enough for review on the phone.
    static func accept(_ entry: PendingChange, in text: String) -> String? {
        if entry.old.isEmpty {
            guard let at = insertionPoint(entry, in: text) else { return nil }
            var out = text
            out.insert(contentsOf: entry.new, at: at)
            return out
        }
        guard let range = removalRange(entry, in: text) else { return nil }
        var out = text
        out.replaceSubrange(range, with: entry.new)
        return out
    }

    /// Rejecting never touches the text; the entry is just dropped from the list.
    static func reject(_ entry: PendingChange, in text: String) -> String { text }

    private static func removalRange(_ entry: PendingChange, in text: String) -> Range<String.Index>? {
        var search = text.startIndex..<text.endIndex
        while let found = text.range(of: entry.old, range: search) {
            let beforeOK = entry.before.isEmpty || text[..<found.lowerBound].hasSuffix(entry.before)
            let afterOK = entry.after.isEmpty || text[found.upperBound...].hasPrefix(entry.after)
            if beforeOK && afterOK { return found }
            search = found.upperBound..<text.endIndex
        }
        return text.range(of: entry.old)
    }

    private static func insertionPoint(_ entry: PendingChange, in text: String) -> String.Index? {
        if !entry.before.isEmpty || !entry.after.isEmpty {
            let joint = entry.before + entry.after
            if let found = text.range(of: joint) {
                return text.index(found.lowerBound, offsetBy: entry.before.count, limitedBy: text.endIndex)
            }
        }
        let clamped = min(max(entry.at, 0), text.count)
        return text.index(text.startIndex, offsetBy: clamped, limitedBy: text.endIndex)
    }
}

// MARK: Comments — crates/inkuno-logic/src/markdown/mod.rs
//
// `passage<!-- comment: text -->` (`comment_on`). GitHub hides the HTML comment, so the note
// reads cleanly there. Resolve removes the markup and keeps the passage (`links.rs`
// `comment_here`); a comment alone on its line takes the line with it.

struct NoteComment: Identifiable {
    var id: Int
    /// The commented text; empty for a comment on its own.
    var passage: String
    var who: String
    var time: String
    var text: String
    /// 0-based line number, for `onSelectLine`.
    var line: Int
}

enum CommentFormat {
    static let open = "<!-- comment:"
    static let close = "-->"

    /// `markdown/mod.rs` `all_comments`, simplified to one line at a time: comments inside fenced
    /// code blocks are skipped; inline-code spans, table cells and multi-line comments are not
    /// special-cased (ponytail: the Rust core owns those rules).
    static func parse(_ text: String) -> [NoteComment] {
        var found: [NoteComment] = []
        var inFence = false
        for (index, rawLine) in text.components(separatedBy: "\n").enumerated() {
            let line = rawLine.hasSuffix("\r") ? String(rawLine.dropLast()) : rawLine
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if trimmed.hasPrefix("```") || trimmed.hasPrefix("~~~") {
                inFence.toggle()
                continue
            }
            guard !inFence else { continue }
            var from = line.startIndex
            var previousEnd = line.startIndex
            while let openRange = line.range(of: open, range: from..<line.endIndex),
                  let closeRange = line.range(of: close, range: openRange.upperBound..<line.endIndex) {
                // `said_range`: one space on each side is the markup's, the rest is the text.
                var said = String(line[openRange.upperBound..<closeRange.lowerBound])
                if said.hasPrefix(" ") { said.removeFirst() }
                if said.hasSuffix(" "), !said.isEmpty { said.removeLast() }
                let passage = String(line[previousEnd..<openRange.lowerBound]).trimmingCharacters(in: .whitespaces)
                let (who, time, comment) = splitSaid(said)
                found.append(NoteComment(id: found.count, passage: passage, who: who, time: time, text: comment, line: index))
                previousEnd = closeRange.upperBound
                from = closeRange.upperBound
            }
        }
        return found
    }

    /// `shell/comments.rs` `split_name`: the first ": " ends the head; the head's last ", "
    /// splits `Name, HH:MM`. Otherwise the whole said is the text.
    static func splitSaid(_ said: String) -> (who: String, time: String, text: String) {
        guard let headEnd = said.range(of: ": "),
              let split = String(said[..<headEnd.lowerBound]).range(of: ", ", options: .backwards) else {
            return ("", "", said)
        }
        let name = String(said[..<split.lowerBound])
        let time = String(said[split.upperBound..<headEnd.lowerBound])
        let text = String(said[headEnd.upperBound...])
        let clock = time.count == 5 && time[time.index(time.startIndex, offsetBy: 2)] == ":"
            && time.enumerated().allSatisfy { i, c in i == 2 || c.isNumber }
        guard clock, !name.isEmpty, name.count <= 40, !name.contains("\n") else {
            return ("", "", said)
        }
        return (name, time, text)
    }

    /// Resolve: drop the `<!-- comment: … -->` markup, keep the passage. A comment alone on its
    /// line takes the line with it (`links.rs` `comment_here`).
    static func resolve(in text: String, comment: NoteComment) -> String {
        var lines = text.components(separatedBy: "\n")
        guard lines.indices.contains(comment.line) else { return text }
        let line = lines[comment.line]
        guard let openRange = line.range(of: open),
              let closeRange = line.range(of: close, range: openRange.upperBound..<line.endIndex) else {
            return text
        }
        let without = String(line[..<openRange.lowerBound]) + String(line[closeRange.upperBound...])
        if without.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            lines.remove(at: comment.line)
        } else {
            lines[comment.line] = without
        }
        return lines.joined(separator: "\n")
    }
}

// MARK: Outline — markdown/mod.rs `headings`

struct NoteHeading: Identifiable {
    var id: Int { line }
    var level: Int
    var text: String
    /// 0-based line number, for `onSelectLine`.
    var line: Int
}

enum Outline {
    /// ATX headings (`# ` to `###### `) outside fenced code blocks.
    static func parse(_ text: String) -> [NoteHeading] {
        var found: [NoteHeading] = []
        var inFence = false
        for (index, rawLine) in text.components(separatedBy: "\n").enumerated() {
            let line = rawLine.hasSuffix("\r") ? String(rawLine.dropLast()) : rawLine
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if trimmed.hasPrefix("```") || trimmed.hasPrefix("~~~") {
                inFence.toggle()
                continue
            }
            guard !inFence else { continue }
            let hashes = trimmed.prefix(while: { $0 == "#" }).count
            guard (1...6).contains(hashes) else { continue }
            let rest = trimmed.dropFirst(hashes)
            guard rest.hasPrefix(" ") || rest.hasPrefix("\t") else { continue }
            found.append(NoteHeading(level: hashes, text: String(rest.dropFirst()).trimmingCharacters(in: .whitespaces), line: index))
        }
        return found
    }
}

// MARK: Backlinks — crates/inkuno-logic/src/wiki.rs `backlinks`
//
// The desktop matches `[[wikilink]]` targets by note name. The phone scans bodies for a
// `[[Name]]` (case-insensitive) or a relative link to `Name.md`.
// ponytail: alias resolution (`answers`), embeds and the link index live in the Rust core.

enum BacklinkScan {
    static func backlinks(to title: String, fileName: String, selfURL: URL, notes: [(url: URL, title: String, body: String)]) -> [(url: URL, title: String)] {
        let folded = title.lowercased()
        let encoded = fileName.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? fileName
        return notes.compactMap { note in
            guard note.url != selfURL else { return nil }
            let body = note.body
            let wiki = body.range(of: "[[\(folded)]]", options: .caseInsensitive) != nil
            let relative = body.contains("](\(fileName))") || body.contains("](\(encoded))")
                || body.range(of: "]([^)]*\(fileName))", options: .regularExpression) != nil
            return (wiki || relative) ? (note.url, note.title) : nil
        }
        .sorted { $0.title.localizedStandardCompare($1.title) == .orderedAscending }
    }
}

// MARK: Palette ranking — crates/inkuno-logic/src/rank.rs `fuzzy`

enum PaletteRank {
    /// `rank.rs` `fold`: lowercase with accents folded, so « thème » is found by « theme ».
    static func fold(_ text: String) -> String {
        text.lowercased().map { c -> Character in
            switch c {
            case "à", "â", "ä": return "a"
            case "é", "è", "ê", "ë": return "e"
            case "î", "ï": return "i"
            case "ô", "ö": return "o"
            case "ù", "û", "ü": return "u"
            case "ç": return "c"
            default: return c
            }
        }.reduce(into: "") { $0.append($1) }
    }

    /// `rank.rs` `fuzzy`: substring first (earlier is better), otherwise subsequence. Higher wins.
    static func fuzzy(query: String, name: String) -> Int? {
        let q = fold(query), n = fold(name)
        if q.isEmpty { return 1000 }
        if let i = n.range(of: q).map({ n.distance(from: n.startIndex, to: $0.lowerBound) }) {
            return 1000 - i
        }
        var chars = n.makeIterator()
        for c in q {
            var found = false
            while let x = chars.next() {
                if x == c { found = true; break }
            }
            if !found { return nil }
        }
        return -n.count
    }
}
