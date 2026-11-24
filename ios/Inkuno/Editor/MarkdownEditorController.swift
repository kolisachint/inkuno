import SwiftUI
import UIKit

/// Drives the live editor's text view. After every change and caret move it restyles only the lines the
/// edit touched and the caret lines, and it performs the editing actions: list continuation, indent,
/// the accessory bar and the slash blocks. Plain UIKit, so the tests drive it without SwiftUI.
@MainActor
final class MarkdownEditorController: NSObject {
    private let provider: MarkdownLayoutProvider

    var onTextChange: ((String) -> Void)?
    var onSlash: (() -> Void)?

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

    /// The lines of the text as last styled. Valid whenever the text storage matches `styledText`.
    private var layouts: [LineLayout] = [LineLayout(startUTF16: 0, lenUTF16: 0, kind: .para)]
    /// The text and caret as of the last styling pass; the next pass diffs against them.
    private var styledText = NSString()
    private var styledCaret = 0

    // MARK: Setup

    /// `provider` is the Markdown layer: the Rust core unless a test passes another one.
    init(provider: MarkdownLayoutProvider = CoreMarkdownProvider()) {
        self.provider = provider
        super.init()
    }

    private func makeTextView() -> EditorTextView {
        let view = EditorTextView(usingTextLayoutManager: true)
        view.delegate = self
        view.backgroundColor = UIColor(InkColor.sheet)
        view.textContainerInset = UIEdgeInsets(top: 12, left: 0, bottom: 24, right: 0)
        view.textContainer.lineFragmentPadding = 0
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
        view.addGestureRecognizer(tap)
        accessory.onAction = { [weak self] action in self?.perform(action) }
        return view
    }

    // MARK: Text in and out

    /// Shows `text` when it differs from the view. Never during composition.
    func update(text: String) {
        guard textView.markedTextRange == nil, textView.textStorage.string != text else { return }
        textView.text = text
        restyleAll()
    }

    /// Restyles the whole text: after a load, a settings change or an appearance change.
    func restyleAll() {
        guard textView.markedTextRange == nil else { return }
        let text = textView.textStorage.string
        layouts = provider.documentLayout(text)
        styledText = NSString(string: text)
        styledCaret = textView.selectedRange.location
        style(Array(layouts.indices))
    }

    /// Restyles after the text changed, whatever changed it (typing, paste, undo, a replaced range).
    /// It diffs against the last pass, so only the touched lines are restyled. During composition it does
    /// nothing: the marked text belongs to the input method, and the change is restyled once committed.
    private func restyleAfterEdit() {
        guard textView.markedTextRange == nil else { return }
        let old = styledText
        let new = NSString(string: textView.textStorage.string)
        let change = TextChange(from: old, to: new)
        let previousCaret = change.map(styledCaret)

        layouts = provider.documentLayout(new as String)
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
        style(lines.sorted())
    }

    /// Restyles after the caret moved. Only the line it left and the line it entered change.
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
        guard before != after else { return }
        style([before, after].sorted())
    }

    /// Styles `lines` with the caret as last recorded. Undo registration is off while styling, so a
    /// style change never becomes an undo step.
    private func style(_ lines: [Int]) {
        let styler = MarkdownStyler(
            provider: provider,
            textSize: textSize,
            showMarkersOnCaretLine: showMarkers,
            traits: textView.traitCollection
        )
        let undo = textView.undoManager
        undo?.disableUndoRegistration()
        styler.style(
            textView.textStorage,
            layouts: layouts,
            lines: lines,
            caretLine: layouts.lineIndex(containing: styledCaret)
        )
        undo?.enableUndoRegistration()
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

    /// A tap on the box of a task line that is not live toggles it: `[ ]` to `[x]` and back.
    @objc private func handleTap(_ recognizer: UITapGestureRecognizer) {
        guard textView.markedTextRange == nil else { return }
        let point = recognizer.location(in: textView)
        guard let position = textView.closestPosition(to: point) else { return }
        let offset = textView.offset(from: textView.beginningOfDocument, to: position)
        let index = layouts.lineIndex(containing: offset)
        let line = layouts[index]
        guard case .task = line.kind else { return }
        let isLive = showMarkers && index == layouts.lineIndex(containing: styledCaret)
        guard !isLive else { return }

        let indent = lineText(index).prefix { $0 == " " }.utf16.count
        let glyph = line.startUTF16 + indent
        guard offset == glyph || offset == glyph + 1 else { return }
        let box = glyph + 3
        let mark = textView.textStorage.string.substring(startUTF16: box, lengthUTF16: 1)
        guard mark == " " || mark == "x" || mark == "X" else { return }
        replace(NSRange(location: box, length: 1), with: mark == " " ? "x" : " ", selection: textView.selectedRange)
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
}

private extension String {
    /// The UTF-16 slice `[startUTF16, startUTF16 + lengthUTF16)`.
    func substring(startUTF16: Int, lengthUTF16: Int) -> String {
        (self as NSString).substring(with: NSRange(location: startUTF16, length: lengthUTF16))
    }
}
