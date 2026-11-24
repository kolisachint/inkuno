import SwiftUI
import UIKit

/// The live editor's text view: TextKit 2. It asks its controller about Enter and Tab from the keyboard,
/// so a list continues as one undo step.
final class EditorTextView: UITextView {
    /// Called with "\n" or "\t" as the user types. Returns true when the controller handled it.
    var onTypedText: ((String) -> Bool)?
    var onTraitChange: (() -> Void)?

    override func insertText(_ text: String) {
        // During composition (marked text) the input method owns the text: never intercept.
        if markedTextRange == nil, onTypedText?(text) == true {
            return
        }
        super.insertText(text)
    }

    override func traitCollectionDidChange(_ previousTraitCollection: UITraitCollection?) {
        super.traitCollectionDidChange(previousTraitCollection)
        if previousTraitCollection?.userInterfaceStyle != traitCollection.userInterfaceStyle {
            onTraitChange?()
        }
    }
}

/// SwiftUI wrapper. The controller owns the text view and writes edits back to `text`.
struct LiveMarkdownEditor: UIViewRepresentable {
    @Binding var text: String
    let controller: MarkdownEditorController
    let textSize: CGFloat
    let showMarkers: Bool
    let isEditable: Bool
    let onSlash: () -> Void

    func makeUIView(context: Context) -> EditorTextView {
        controller.textView
    }

    func updateUIView(_ view: EditorTextView, context: Context) {
        controller.onTextChange = { text = $0 }
        controller.onSlash = onSlash
        controller.isEditable = isEditable
        controller.textSize = textSize
        controller.showMarkers = showMarkers
        controller.update(text: text)
    }
}
