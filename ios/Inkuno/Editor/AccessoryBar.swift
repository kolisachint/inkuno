import UIKit

/// The actions of the keyboard accessory bar.
enum EditorAccessoryAction {
    case slash, hash, wikiLink, task, indent, outdent, undo, hideKeyboard
}

/// The bar above the keyboard: `/ # [[ task indent outdent undo hide`. 44 pt, on the InkColor tokens.
final class EditorAccessoryBar: UIView {
    var onAction: ((EditorAccessoryAction) -> Void)?

    private typealias Item = (action: EditorAccessoryAction, title: String?, symbol: String?, label: String)

    private static let items: [Item] = [
        (.slash, "/", nil, "Insert block"),
        (.hash, "#", nil, "Tag"),
        (.wikiLink, "[[", nil, "Link to a note"),
        (.task, nil, "checkmark.square", "Task"),
        (.indent, nil, "increase.indent", "Indent"),
        (.outdent, nil, "decrease.indent", "Outdent"),
        (.undo, nil, "arrow.uturn.backward", "Undo"),
        (.hideKeyboard, nil, "keyboard.chevron.compact.down", "Hide keyboard"),
    ]

    init() {
        super.init(frame: CGRect(x: 0, y: 0, width: 0, height: 44))
        autoresizingMask = .flexibleWidth
        backgroundColor = UIColor(InkColor.ground)

        let buttons = Self.items.enumerated().map { index, item in makeButton(index, item) }
        let stack = UIStackView(arrangedSubviews: buttons)
        stack.axis = .horizontal
        stack.distribution = .fillEqually
        stack.translatesAutoresizingMaskIntoConstraints = false

        let hairline = UIView()
        hairline.backgroundColor = UIColor(InkColor.line)
        hairline.translatesAutoresizingMaskIntoConstraints = false

        addSubview(stack)
        addSubview(hairline)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: trailingAnchor),
            stack.topAnchor.constraint(equalTo: topAnchor),
            stack.bottomAnchor.constraint(equalTo: bottomAnchor),
            hairline.leadingAnchor.constraint(equalTo: leadingAnchor),
            hairline.trailingAnchor.constraint(equalTo: trailingAnchor),
            hairline.topAnchor.constraint(equalTo: topAnchor),
            hairline.heightAnchor.constraint(equalToConstant: 1),
        ])
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
    }

    override var intrinsicContentSize: CGSize {
        CGSize(width: UIView.noIntrinsicMetric, height: 44)
    }

    private func makeButton(_ index: Int, _ item: Item) -> UIButton {
        let button = UIButton(type: .system)
        if let title = item.title {
            button.setTitle(title, for: .normal)
            button.titleLabel?.font = .monospacedSystemFont(ofSize: 17, weight: .medium)
        }
        if let symbol = item.symbol {
            button.setImage(UIImage(systemName: symbol), for: .normal)
        }
        button.tintColor = UIColor(InkColor.icon)
        button.accessibilityLabel = item.label
        button.tag = index
        button.addTarget(self, action: #selector(tapped(_:)), for: .touchUpInside)
        return button
    }

    @objc private func tapped(_ sender: UIButton) {
        onAction?(Self.items[sender.tag].action)
    }
}
