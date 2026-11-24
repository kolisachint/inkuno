import Foundation

// MARK: - Desktop formats, mirrored in Swift
//
// The on-disk formats below are the desktop's, not new ones. Each section names the Rust source
// it mirrors; the paths are relative to the repo root.

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
