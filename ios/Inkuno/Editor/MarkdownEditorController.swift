import InkunoCore
import SwiftUI
import UIKit

/// The formatting the palette's Formatting group applies to the open note.
enum EditorFormat: Equatable {
    /// `**` around the selection.
    case bold
    /// `*` around the selection.
    case italic
    /// `# ` at the start of the caret's line.
    case heading
}

/// A replacement of `range` in a text with `text`, and the selection that follows it.
struct EditorEdit: Equatable {
    /// The UTF-16 range replaced.
    let range: NSRange
    /// What replaces it.
    let text: String
    /// The selection after the replacement.
    let selection: NSRange
}

/// Drives the live editor's text view. After every change and caret move it restyles only the lines the
/// edit touched and the caret lines, and it performs the editing actions: list continuation, indent,
/// the accessory bar, the slash blocks, the formatting (Bold, Italic, Heading) and the block menu (Comment,
/// Show Markdown, Duplicate, Delete). Plain UIKit, so the tests drive it without SwiftUI.
@MainActor
final class MarkdownEditorController: NSObject {
    private let provider: MarkdownLayoutProvider

    var onTextChange: ((String) -> Void)?
    var onSlash: (() -> Void)?
    /// Comment from the block menu: the block's line index, its text, and whether it is a table line.
    var onComment: ((_ line: Int, _ text: String) -> Void)?
    /// A tap on a wiki link, a Markdown link to a note, or a tag, on a line that is not live. The host opens it.
    /// The markup is the link's source text (`[[Plan#Intro]]`), which keeps a wiki link's heading. A `.url` opens
    /// in the browser and a `.heading` scrolls this note, so the host is not asked for those.
    var onOpenLink: ((_ link: InkunoCore.LinkInfo, _ markup: String) -> Void)?
    /// A tap on an `@` day on a line that is not live: the word after the `@`, as written (`2026-11-24`, `monday`).
    var onOpenDate: ((String) -> Void)?
    /// A tap on the passage of a comment on a line that is not live: the comment, to show its popover.
    var onOpenComment: ((NoteComment) -> Void)?

    var textSize: CGFloat = 16 {
        didSet {
            if oldValue != textSize { restyleAll() }
        }
    }

    var showMarkers = true {
        didSet {
            if oldValue != showMarkers { restyleAll() }
        }
    }

    var isEditable = true {
        didSet { textView.isEditable = isEditable }
    }

    /// Created on first use, so the controller is cheap to build inside SwiftUI state.
    private(set) lazy var textView: EditorTextView = makeTextView()
    private lazy var accessory = EditorAccessoryBar()
    /// Draws the bullet and task glyphs in each paragraph's layout fragment (`MarkdownLayoutFragment`).
    private let glyphLayout = MarkdownLayoutDelegate()

    /// The lines of the text as last styled. Valid whenever the text storage matches `styledText`.
    private var layouts: [LineLayout] = [LineLayout(startUTF16: 0, lenUTF16: 0, kind: .para)]
    /// The text and caret as of the last styling pass; the next pass diffs against them.
    private var styledText = NSString()
    private var styledCaret = 0
    /// The lines whose Markdown the block menu's Show Markdown made visible. Kept while the caret stays in them.
    private var revealedLines: Set<Int> = []
    /// The caret line when the current touch began. A tap on a line that was not the caret's keeps its markup.
    private var touchCaretLine = 0
    /// The passage of each comment of the text as last styled, with its range: what is washed and tapped.
    private var commentPassages: [(range: NSRange, comment: NoteComment)] = []
    /// The horizontal offset of each table block the pan has scrolled, keyed by the block's first line.
    private var tableOffsets: [Int: CGFloat] = [:]
    /// The first line of the table the horizontal pan is scrolling, from the pan's start.
    private var scrollingTable: Int?
    /// Scrolls a table sideways. It begins only on a table row, and only when it moves mostly sideways.
    private var tablePan: UIPanGestureRecognizer?

    // MARK: Setup

    /// `provider` is the Markdown layer: the Rust core unless a test passes another one.
    init(provider: MarkdownLayoutProvider = CoreMarkdownProvider()) {
        self.provider = provider
        super.init()
    }

    private func makeTextView() -> EditorTextView {
        let view = EditorTextView(usingTextLayoutManager: true)
        view.delegate = self
        view.accessibilityIdentifier = "editor.text"
        view.backgroundColor = UIColor(InkColor.sheet)
        // The right inset holds each comment's dot, so a full-width line wraps before the dot.
        view.textContainerInset = UIEdgeInsets(top: 12, left: 0, bottom: 24, right: MarkdownLayoutFragment.commentDotMargin)
        view.textContainer.lineFragmentPadding = 0
        view.textLayoutManager?.delegate = glyphLayout
        // Smart punctuation would corrupt Markdown: "--" to a dash, straight quotes to curly ones.
        view.smartQuotesType = .no
        view.smartDashesType = .no
        view.smartInsertDeleteType = .no
        view.autocapitalizationType = .sentences
        view.alwaysBounceVertical = true
        view.keyboardDismissMode = .interactive
        view.inputAccessoryView = accessory
        view.onTypedText = { [weak self] text in self?.handleTyped(text) ?? false }
        view.onTraitChange = { [weak self] in self?.restyleAll() }
        let tap = UITapGestureRecognizer(target: self, action: #selector(handleTap(_:)))
        tap.cancelsTouchesInView = false
        tap.delegate = self
        view.addGestureRecognizer(tap)
        // The text view's double-tap (word selection) must fail before a single tap acts. Otherwise the first tap
        // of a double-tap opens a link, tag, day or comment, or toggles a task box, as well as selecting the word.
        for case let doubleTap as UITapGestureRecognizer in view.gestureRecognizers ?? [] where doubleTap.numberOfTapsRequired == 2 {
            tap.require(toFail: doubleTap)
        }
        let pan = UIPanGestureRecognizer(target: self, action: #selector(handleTablePan(_:)))
        pan.delegate = self
        view.addGestureRecognizer(pan)
        // The text view's scroll pan waits for the table pan to fail, so a vertical drag fails the table pan at once
        // (`gestureRecognizerShouldBegin`) and the note scrolls; a horizontal drag on a table is not scrolled by both.
        view.panGestureRecognizer.require(toFail: pan)
        tablePan = pan
        accessory.onAction = { [weak self] action in self?.perform(action) }
        return view
    }

    // MARK: Text in and out

    /// Shows `text` when it differs from the view. Never during composition.
    func update(text: String) {
        guard textView.markedTextRange == nil, textView.textStorage.string != text else { return }
        textView.text = text
        tableOffsets = [:]
        restyleAll()
    }

    /// Restyles the whole text: after a load, a settings change or an appearance change.
    func restyleAll() {
        guard textView.markedTextRange == nil else { return }
        let text = textView.textStorage.string
        layouts = provider.documentLayout(text)
        commentPassages = Self.passages(of: text)
        styledText = NSString(string: text)
        styledCaret = textView.selectedRange.location
        style(Array(layouts.indices))
    }

    /// Restyles after the text changed, whatever changed it (typing, paste, undo, a replaced range).
    /// It diffs against the last pass, so only the touched lines are restyled. During composition it does
    /// nothing: the marked text belongs to the input method, and the change is restyled once committed.
    private func restyleAfterEdit() {
        guard textView.markedTextRange == nil else { return }
        // Keys are first lines, which an edit can move: a table re-entered after an edit starts unscrolled.
        let scrolledTables = !tableOffsets.isEmpty
        tableOffsets = [:]
        let old = styledText
        let new = NSString(string: textView.textStorage.string)
        let change = TextChange(from: old, to: new)
        let previousCaret = change.map(styledCaret)

        layouts = provider.documentLayout(new as String)
        commentPassages = Self.passages(of: new as String)
        styledText = new
        styledCaret = textView.selectedRange.location

        var lines: Set<Int> = [
            layouts.lineIndex(containing: previousCaret),
            layouts.lineIndex(containing: styledCaret),
        ]
        if change.isChanged {
            if Self.touchesFence(change, old: old, new: new) {
                // A fence changes how every later line reads, so restyle the lot.
                lines = Set(layouts.indices)
            } else {
                let low = layouts.lineIndex(containing: change.prefix)
                let high = layouts.lineIndex(containing: max(change.prefix, change.newEnd))
                lines.formUnion(low...high)
            }
        }
        if scrolledTables {
            // The offsets were cleared, so every table is drawn at offset 0 from now on. Restyle them all, or a
            // table the edit did not touch keeps drawing at the offset the pan left it at.
            lines.formUnion(layouts.indices.filter { layouts[$0].kind == .table })
        }
        style(lines.sorted())
    }

    /// Restyles after the caret moved. Only the line it left and the line it entered change, and the
    /// revealed lines when the caret leaves them.
    func selectionChanged() {
        guard textView.markedTextRange == nil else { return }
        guard textView.textStorage.string == (styledText as String) else {
            restyleAfterEdit()
            return
        }
        let caret = textView.selectedRange.location
        let before = layouts.lineIndex(containing: styledCaret)
        let after = layouts.lineIndex(containing: caret)
        styledCaret = caret
        var changed: Set<Int> = before != after ? [before, after] : []
        if !revealedLines.isEmpty, !revealedLines.contains(after) {
            changed.formUnion(revealedLines)
            revealedLines = []
        }
        guard !changed.isEmpty else { return }
        style(changed.sorted())
    }

    /// Scrolls to `line` (0-based, as the outline and the comments give it) and puts the caret at its start.
    func reveal(line: Int) {
        guard !layouts.isEmpty else { return }
        let index = min(max(0, line), layouts.count - 1)
        let location = layouts[index].startUTF16
        textView.selectedRange = NSRange(location: location, length: 0)
        selectionChanged()
        textView.scrollRangeToVisible(NSRange(location: location, length: 0))
    }

    /// Styles `lines` with the caret as last recorded. Undo registration is off while styling, so a
    /// style change never becomes an undo step.
    private func style(_ lines: [Int]) {
        let styler = MarkdownStyler(
            provider: provider,
            textSize: textSize,
            showMarkersOnCaretLine: showMarkers,
            traits: textView.traitCollection,
            washes: commentPassages.map { $0.range },
            tableOffsets: tableOffsets
        )
        let undo = textView.undoManager
        undo?.disableUndoRegistration()
        styler.style(
            textView.textStorage,
            layouts: layouts,
            lines: lines,
            caretLine: layouts.lineIndex(containing: styledCaret),
            revealedLines: revealedLines
        )
        undo?.enableUndoRegistration()
    }

    /// The passage of each comment of `text` with its UTF-16 range, and the comment. A comment on its own has no
    /// passage and is left out. Pure apart from the core's parse, so the tests read it without a view.
    static func passages(of text: String) -> [(range: NSRange, comment: NoteComment)] {
        CommentFormat.parse(text).compactMap { comment -> (range: NSRange, comment: NoteComment)? in
            let length = comment.passage.utf16.count
            guard length > 0 else { return nil }
            return (range: NSRange(location: Int(comment.startUtf16), length: length), comment: comment)
        }
    }

    private static func touchesFence(_ change: TextChange, old: NSString, new: NSString) -> Bool {
        func hasFence(_ text: NSString, _ from: Int, _ to: Int) -> Bool {
            let low = max(0, from - 2)
            let high = min(text.length, to + 2)
            guard low < high else { return false }
            let part = text.substring(with: NSRange(location: low, length: high - low))
            return part.contains("```") || part.contains("~~~")
        }
        return hasFence(new, change.prefix, change.newEnd) || hasFence(old, change.prefix, change.oldEnd)
    }

    // MARK: Edits

    /// Replaces `range` through UIKit, so the change is one undo step. Then restyles and reports the text.
    private func replace(_ range: NSRange, with string: String, selection: NSRange? = nil) {
        guard let start = textView.position(from: textView.beginningOfDocument, offset: range.location),
              let end = textView.position(from: start, offset: range.length),
              let target = textView.textRange(from: start, to: end)
        else { return }
        textView.replace(target, withText: string)
        if let selection {
            textView.selectedRange = selection
        }
        restyleAfterEdit()
        onTextChange?(textView.text)
    }

    /// The text of line `index`, without its newline.
    private func lineText(_ index: Int) -> String {
        let line = layouts[index]
        let hasNewline = index < layouts.count - 1
        let length = line.lenUTF16 - (hasNewline ? 1 : 0)
        return textView.textStorage.string.substring(startUTF16: line.startUTF16, lengthUTF16: length)
    }

    /// Enter and Tab from the keyboard. Returns true when the controller did the work.
    private func handleTyped(_ typed: String) -> Bool {
        guard textView.markedTextRange == nil else { return false }
        switch typed {
        case "\n":
            return continueList()
        case "\t":
            indent(outdent: false)
            return true
        default:
            return false
        }
    }

    private func continueList() -> Bool {
        let caret = textView.selectedRange
        let index = layouts.lineIndex(containing: caret.location)
        let line = layouts[index]
        let outcome = provider.onEnter(
            line: lineText(index),
            colUTF16: caret.location - line.startUTF16,
            inFence: line.inCodeBlock
        )
        switch outcome {
        case .none:
            return false
        case .insert(let next):
            replace(caret, with: "\n" + next)
        case .clear:
            replace(
                NSRange(location: line.startUTF16, length: line.markerLenUTF16),
                with: "",
                selection: NSRange(location: line.startUTF16, length: 0)
            )
        }
        return true
    }

    private func indent(outdent: Bool) {
        let old = textView.textStorage.string
        let (newText, newRange) = provider.indent(old, rangeUTF16: textView.selectedRange, outdent: outdent)
        let change = TextChange(from: NSString(string: old), to: NSString(string: newText))
        guard change.isChanged else {
            textView.selectedRange = newRange
            return
        }
        let inserted = (newText as NSString).substring(
            with: NSRange(location: change.prefix, length: change.newEnd - change.prefix)
        )
        replace(
            NSRange(location: change.prefix, length: change.oldEnd - change.prefix),
            with: inserted,
            selection: newRange
        )
    }

    // MARK: Formatting

    /// Bold: wraps the selection in `**`, or unwraps it when it is already wrapped.
    func toggleBold() {
        applyWrap("**")
    }

    /// Italic: wraps the selection in `*`, or unwraps it when it is already wrapped.
    func toggleItalic() {
        applyWrap("*")
    }

    /// Heading: adds `# ` to the start of the caret's line, or removes it when the line already has it.
    func toggleHeading() {
        let edit = Self.headingEdit(textView.textStorage.string, caret: textView.selectedRange.location)
        replace(edit.range, with: edit.text, selection: edit.selection)
    }

    /// Applies `format` to the note, as the palette's Formatting group asks.
    func apply(_ format: EditorFormat) {
        switch format {
        case .bold: toggleBold()
        case .italic: toggleItalic()
        case .heading: toggleHeading()
        }
    }

    /// Wraps or unwraps the selection in `marker` through the edit path, so it is one undo step.
    private func applyWrap(_ marker: String) {
        let edit = Self.wrapEdit(textView.textStorage.string, selection: textView.selectedRange, marker: marker)
        replace(edit.range, with: edit.text, selection: edit.selection)
    }

    /// The edit that wraps `selection` of `text` in `marker`. A selection already wrapped, inside it (`**x**` selected
    /// whole) or just outside it (`x` between the `**`), is unwrapped instead. An empty selection gets the pair with the
    /// caret between them. Pure, so the tests read it without a view.
    static func wrapEdit(_ text: String, selection: NSRange, marker: String) -> EditorEdit {
        let ns = text as NSString
        let m = marker.utf16.count
        let at = selection.location
        let end = NSMaxRange(selection)
        guard selection.length > 0 else {
            return EditorEdit(range: selection, text: marker + marker, selection: NSRange(location: at + m, length: 0))
        }
        let inner = ns.substring(with: selection)
        if let core = unwrapped(inner, marker: marker) {
            return EditorEdit(range: selection, text: core, selection: NSRange(location: at, length: core.utf16.count))
        }
        // The markers outside the selection: they must not be part of a longer run of the same character
        // (`*x*` inside `**x**` is not italic, so Italic wraps the bold text again).
        if let last = marker.utf16.last, at >= m, end + m <= ns.length,
           ns.substring(with: NSRange(location: at - m, length: m)) == marker,
           ns.substring(with: NSRange(location: end, length: m)) == marker,
           !(at - m > 0 && ns.character(at: at - m - 1) == last),
           !(end + m < ns.length && ns.character(at: end + m) == last) {
            return EditorEdit(
                range: NSRange(location: at - m, length: selection.length + 2 * m),
                text: inner,
                selection: NSRange(location: at - m, length: selection.length)
            )
        }
        return EditorEdit(
            range: selection,
            text: marker + inner + marker,
            selection: NSRange(location: at + m, length: selection.length)
        )
    }

    /// The text between `marker`s when `part` is wrapped in them, else nil. The core must not start or end with the
    /// marker's character: `**x**` is bold, not italic `*x*`.
    private static func unwrapped(_ part: String, marker: String) -> String? {
        let ns = part as NSString
        let m = marker.utf16.count
        guard ns.length > 2 * m, part.hasPrefix(marker), part.hasSuffix(marker), let last = marker.utf16.last else {
            return nil
        }
        let core = ns.substring(with: NSRange(location: m, length: ns.length - 2 * m))
        guard core.utf16.first != last, core.utf16.last != last else { return nil }
        return core
    }

    /// The edit that adds `# ` to the start of the line of `caret`, or removes it when the line already starts with it.
    /// The caret moves with the text. Pure, so the tests read it without a view.
    static func headingEdit(_ text: String, caret: Int) -> EditorEdit {
        let ns = text as NSString
        let marker = "# "
        let m = marker.utf16.count
        let line = ns.lineRange(for: NSRange(location: caret, length: 0)).location
        if line + m <= ns.length, ns.substring(with: NSRange(location: line, length: m)) == marker {
            return EditorEdit(
                range: NSRange(location: line, length: m),
                text: "",
                selection: NSRange(location: max(line, caret - m), length: 0)
            )
        }
        return EditorEdit(
            range: NSRange(location: line, length: 0),
            text: marker,
            selection: NSRange(location: caret + m, length: 0)
        )
    }

    // MARK: Block menu

    /// The lines the selection touches: the block the menu acts on. A caret alone is its own line.
    private func selectedLines() -> ClosedRange<Int> {
        let selection = textView.selectedRange
        let first = layouts.lineIndex(containing: selection.location)
        let end = selection.length > 0 ? NSMaxRange(selection) - 1 : selection.location
        let last = layouts.lineIndex(containing: max(selection.location, end))
        return first...max(first, last)
    }

    /// The UTF-16 range of a block: its lines, with the line break after the last one when it has one.
    /// A layout's `lenUTF16` already counts that break.
    private func blockRange(_ lines: ClosedRange<Int>) -> NSRange {
        let start = layouts[lines.lowerBound].startUTF16
        let last = layouts[lines.upperBound]
        return NSRange(location: start, length: last.startUTF16 + last.lenUTF16 - start)
    }

    /// Comment: asks the host for the comment text of the block's line.
    func beginComment() {
        let index = selectedLines().lowerBound
        onComment?(index, lineText(index))
    }

    /// Writes the comment on line `index` as `passage<!-- comment @author time: said -->`, the passage being the
    /// line, the author the one in Settings and the time now (`CommentFormat.commented`).
    /// False (and nothing written) when the line cannot take a comment or `said` is empty.
    @discardableResult
    func commentLine(_ index: Int, said: String) -> Bool {
        guard layouts.indices.contains(index) else { return false }
        let line = layouts[index]
        let text = lineText(index)
        guard let commented = CommentFormat.commented(line: text, said: said) else { return false }
        replace(
            NSRange(location: line.startUTF16, length: text.utf16.count),
            with: commented,
            selection: NSRange(location: line.startUTF16 + commented.utf16.count, length: 0)
        )
        return true
    }

    /// Show Markdown: the block's markers show while the caret stays in it.
    func revealBlock() {
        let lines = selectedLines()
        revealedLines = Set(lines)
        style(Array(lines))
    }

    /// Copy: the block's text on the pasteboard, without its line break.
    func copyBlock() {
        let range = blockRange(selectedLines())
        var text = textView.textStorage.string.substring(startUTF16: range.location, lengthUTF16: range.length)
        if text.hasSuffix("\n") {
            text.removeLast()
        }
        UIPasteboard.general.string = text
    }

    /// Duplicate: a copy of the block goes right after it, and the caret moves to the copy.
    func duplicateBlock() {
        let range = blockRange(selectedLines())
        let block = textView.textStorage.string.substring(startUTF16: range.location, lengthUTF16: range.length)
        let end = NSMaxRange(range)
        let hasBreak = block.hasSuffix("\n")
        // A block at the very end of the note has no break after it: its copy comes after a new one.
        let copy = hasBreak ? block : "\n" + block
        replace(NSRange(location: end, length: 0), with: copy, selection: NSRange(location: end + (hasBreak ? 0 : 1), length: 0))
    }

    /// Delete: the block goes, with its line break (or the one before it, for the last line of the note).
    func deleteBlock() {
        let range = blockRange(selectedLines())
        let text = textView.textStorage.string as NSString
        let hasBreak = range.length > 0 && text.character(at: NSMaxRange(range) - 1) == 10
        var cut = range
        if !hasBreak, range.location > 0 {
            cut = NSRange(location: range.location - 1, length: range.length + 1)
        }
        replace(cut, with: "", selection: NSRange(location: cut.location, length: 0))
    }

    /// Inserts a slash block at the caret line: into the line when it is empty, else on a new line after it.
    func insertBlock(_ item: SlashItem) {
        let index = layouts.lineIndex(containing: textView.selectedRange.location)
        let line = layouts[index]
        let text = lineText(index)
        let end = line.startUTF16 + text.utf16.count
        if text.isEmpty {
            replace(
                NSRange(location: line.startUTF16, length: 0),
                with: item.markdown,
                selection: NSRange(location: line.startUTF16 + item.caretOffset, length: 0)
            )
        } else {
            replace(
                NSRange(location: end, length: 0),
                with: "\n" + item.markdown,
                selection: NSRange(location: end + 1 + item.caretOffset, length: 0)
            )
        }
        _ = textView.becomeFirstResponder()
    }

    private func perform(_ action: EditorAccessoryAction) {
        let selection = textView.selectedRange
        switch action {
        case .slash:
            onSlash?()
        case .hash:
            replace(selection, with: "#")
        case .wikiLink:
            replace(selection, with: "[[]]", selection: NSRange(location: selection.location + 2, length: 0))
        case .task:
            let start = layouts[layouts.lineIndex(containing: selection.location)].startUTF16
            replace(
                NSRange(location: start, length: 0),
                with: "- [ ] ",
                selection: NSRange(location: selection.location + 6, length: selection.length)
            )
        case .indent:
            indent(outdent: false)
        case .outdent:
            indent(outdent: true)
        case .undo:
            textView.undoManager?.undo()
            restyleAfterEdit()
            onTextChange?(textView.text)
        case .hideKeyboard:
            _ = textView.resignFirstResponder()
        }
    }

    /// The link of `line` that holds the UTF-16 `column`, if any. Pure, so the tests read it without a view.
    static func link(in line: String, at column: Int) -> InkunoCore.LinkInfo? {
        InkunoCore.lineLinks(line: line).first { link in
            column >= Int(link.startUtf16) && column < Int(link.startUtf16 + link.lenUtf16)
        }
    }

    /// The word after the `@` of the day run of `text` that holds `column`, when the core reads it as a day.
    private func dateWord(in text: String, column: Int) -> String? {
        let run = provider.styleRuns(line: text).first { run in
            run.bits & StyleBit.date != 0 && column >= run.startUTF16 && column < run.startUTF16 + run.lenUTF16
        }
        guard let run else { return nil }
        return text.substring(startUTF16: run.startUTF16 + 1, lengthUTF16: run.lenUTF16 - 1)
    }

    /// Opens the link, the day or the comment under `offset` on the non-live line `index`. False when none is there.
    private func openMarkup(at offset: Int, line index: Int) -> Bool {
        guard !layouts[index].inCodeBlock else { return false }
        let text = lineText(index)
        let column = offset - layouts[index].startUTF16
        if let link = Self.link(in: text, at: column) {
            switch link.kind {
            case .url:
                if let url = URL(string: link.target) {
                    UIApplication.shared.open(url)
                }
            case .heading:
                if let at = InkunoCore.linkPosition(text: textView.textStorage.string, anchor: .heading(title: link.target)) {
                    reveal(line: layouts.lineIndex(containing: Int(at)))
                }
            case .footnote:
                return false
            case .wiki, .note, .tag:
                let markup = text.substring(startUTF16: Int(link.startUtf16), lengthUTF16: Int(link.lenUtf16))
                onOpenLink?(link, markup)
            }
            return true
        }
        if let word = dateWord(in: text, column: column) {
            onOpenDate?(word)
            return true
        }
        guard let passage = commentPassages.first(where: { comment in
            offset >= comment.range.location && offset < NSMaxRange(comment.range)
        }) else { return false }
        onOpenComment?(passage.comment)
        return true
    }

    /// A tap on a link, a tag or a day opens it (`openMarkup`) when the line is not live and no text is
    /// selected. A tap on the box of a task line that is not live toggles it: `[ ]` to `[x]` and back.
    @objc private func handleTap(_ recognizer: UITapGestureRecognizer) {
        guard textView.markedTextRange == nil else { return }
        let point = recognizer.location(in: textView)
        guard let position = textView.closestPosition(to: point) else { return }
        let offset = textView.offset(from: textView.beginningOfDocument, to: position)
        let index = layouts.lineIndex(containing: offset)
        let line = layouts[index]
        let isLive = showMarkers && index == touchCaretLine
        if !isLive, textView.selectedRange.length == 0, openMarkup(at: offset, line: index) {
            return
        }
        guard case .task = line.kind else { return }
        guard !isLive else { return }

        let indent = lineText(index).prefix { $0 == " " }.utf16.count
        let glyph = line.startUTF16 + indent
        guard offset == glyph || offset == glyph + 1 else { return }
        let box = glyph + 3
        let mark = textView.textStorage.string.substring(startUTF16: box, lengthUTF16: 1)
        guard mark == " " || mark == "x" || mark == "X" else { return }
        replace(NSRange(location: box, length: 1), with: mark == " " ? "x" : " ", selection: textView.selectedRange)
    }

    /// The first line of the table block under `point` in the text view, when the point is on a table row.
    private func tableStart(at point: CGPoint) -> Int? {
        guard let position = textView.closestPosition(to: point) else { return nil }
        let index = layouts.lineIndex(containing: textView.offset(from: textView.beginningOfDocument, to: position))
        guard layouts[index].kind == .table else { return nil }
        return layouts.tableBlock(containing: index).lowerBound
    }

    /// The width of the table block at `start`, and the width its box shows in the text container (as
    /// `MarkdownLayoutFragment` draws it).
    private func tableWidths(_ start: Int) -> (table: CGFloat, visible: CGFloat)? {
        let storage = textView.textStorage
        for index in layouts.tableBlock(containing: start) {
            let at = layouts[index].startUTF16
            guard at < storage.length,
                  let mark = storage.attribute(.inkTableRow, at: at, effectiveRange: nil) as? TableRowMark
            else { continue }
            let fit = textView.textContainer.size.width - 1
            return (mark.width, fit > 0 ? min(mark.width, fit) : mark.width)
        }
        return nil
    }

    /// Moves the table the pan began on by the drag, kept within its box, and restyles its lines with the new offset.
    @objc private func handleTablePan(_ recognizer: UIPanGestureRecognizer) {
        if recognizer.state == .began {
            scrollingTable = tableStart(at: recognizer.location(in: textView))
        }
        guard recognizer.state == .began || recognizer.state == .changed else {
            scrollingTable = nil
            return
        }
        guard let start = scrollingTable, let widths = tableWidths(start) else { return }
        let dragged = recognizer.translation(in: textView).x
        recognizer.setTranslation(.zero, in: textView)
        let scrollable = max(0, widths.table - widths.visible)
        tableOffsets[start] = min(max(0, (tableOffsets[start] ?? 0) - dragged), scrollable)
        // Restyling the table's lines sets their new offset, and the text view redraws the fragments that show it.
        style(Array(layouts.tableBlock(containing: start)))
        textView.setNeedsDisplay()
    }
}

extension MarkdownEditorController: UIGestureRecognizerDelegate {
    /// Records the caret line as a touch begins, before the text view moves the caret for it.
    func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldReceive touch: UITouch) -> Bool {
        touchCaretLine = layouts.lineIndex(containing: styledCaret)
        return true
    }

    /// The table pan begins only when it moves mostly sideways and starts on a table row. Its direction is the
    /// translation once it has moved; the velocity is used only while the translation is still zero.
    func gestureRecognizerShouldBegin(_ gestureRecognizer: UIGestureRecognizer) -> Bool {
        guard let pan = tablePan, gestureRecognizer === pan else { return true }
        var motion = pan.translation(in: textView)
        if motion == .zero {
            motion = pan.velocity(in: textView)
        }
        guard abs(motion.x) > abs(motion.y) else { return false }
        return tableStart(at: pan.location(in: textView)) != nil
    }

    /// The table pan does not recognise together with any other gesture: the text view's scroll pan waits for it to fail
    /// (`require(toFail:)`), so a vertical drag scrolls the note and a sideways drag on a table scrolls the table alone.
    /// The tap needs no overlap with a pan, so nothing is allowed to recognise with the table pan.
    func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer,
                           shouldRecognizeSimultaneouslyWith other: UIGestureRecognizer) -> Bool {
        false
    }
}

extension MarkdownEditorController: UITextViewDelegate {
    func textViewDidChange(_ textView: UITextView) {
        restyleAfterEdit()
        onTextChange?(textView.text)
    }

    func textViewDidChangeSelection(_ textView: UITextView) {
        selectionChanged()
    }

    /// The edit menu of a selection: the system's actions, then the block actions (Comment, Show
    /// Markdown, Copy, Duplicate, and Delete last, destructive). A table block says Edit as Markdown: the same
    /// reveal, since a table is drawn as a grid until its markup shows.
    /// The long-press that selects a block's text opens this menu, so the block menu needs no gesture of its own:
    /// a UIContextMenuInteraction on the text view would compete with the system's selection long-press.
    func textView(_ textView: UITextView, editMenuForTextIn range: NSRange, suggestedActions: [UIMenuElement]) -> UIMenu? {
        let isTable = layouts[layouts.lineIndex(containing: textView.selectedRange.location)].kind == .table
        let block = UIMenu(options: .displayInline, children: [
            UIAction(title: "Comment", image: UIImage(systemName: "text.bubble")) { [weak self] _ in
                self?.beginComment()
            },
            UIAction(
                title: isTable ? "Edit as Markdown" : "Show Markdown",
                image: UIImage(systemName: "chevron.left.forwardslash.chevron.right")
            ) { [weak self] _ in
                self?.revealBlock()
            },
            UIAction(title: "Copy", image: UIImage(systemName: "doc.on.doc")) { [weak self] _ in
                self?.copyBlock()
            },
            UIAction(title: "Duplicate", image: UIImage(systemName: "plus.square.on.square")) { [weak self] _ in
                self?.duplicateBlock()
            },
        ])
        let delete = UIMenu(options: .displayInline, children: [
            UIAction(title: "Delete", image: UIImage(systemName: "trash"), attributes: .destructive) { [weak self] _ in
                self?.deleteBlock()
            },
        ])
        return UIMenu(children: suggestedActions + [block, delete])
    }
}

private extension String {
    /// The UTF-16 slice `[startUTF16, startUTF16 + lengthUTF16)`.
    func substring(startUTF16: Int, lengthUTF16: Int) -> String {
        (self as NSString).substring(with: NSRange(location: startUTF16, length: lengthUTF16))
    }
}
