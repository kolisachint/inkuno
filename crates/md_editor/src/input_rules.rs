//! Input rules wired to editing keys (plan task 4.5, DESIGN §4.3). Owner: task 4.5.
//!
//! Typed text (the IME commit path, see `ime.rs`), Enter and Backspace ask
//! `md_core::input_rules::apply_input_rule` first. A matching rule applies its edit through
//! `Editor::replace_range`, so it is one undo step, and then sets the caret the rule returns.
//! When no rule matches, the selection is not empty, or an IME composition is active, the
//! caller does its default behavior.

use gpui::Context;
use md_core::input_rules::{apply_input_rule, Input, RuleResult};

use crate::editor::Editor;
use crate::selection::Selection;

/// Rules kept off until the repo owner confirms the math work (AGENTS.md, "ON HOLD"). The
/// engine still has the rule; the editor simply does not apply it, so Enter after `$$` keeps
/// its default behavior.
const HELD_RULES: &[&str] = &["math_block"];

/// True when a rule result may be applied now.
fn allowed(result: &RuleResult) -> bool {
    !HELD_RULES.contains(&result.rule)
}

/// Try the input rules for `input` at the caret. Returns true when a rule applied its edit.
///
/// Does nothing while an IME composition is active or when the selection is not empty.
pub fn try_input_rule(editor: &mut Editor, input: Input, cx: &mut Context<Editor>) -> bool {
    if editor.ime.marked.is_some() {
        return false;
    }
    let selection = editor.selection.selection;
    if !selection.is_empty() {
        return false;
    }
    let text = editor.text();
    let Some(result) = apply_input_rule(&text, selection.head, &input) else {
        return false;
    };
    if !allowed(&result) {
        return false;
    }
    // Every rule in md_core returns exactly one edit, so the change is one undo step.
    let [edit] = result.edits.as_slice() else {
        return false;
    };
    editor.replace_range(edit.range.clone(), &edit.text, cx);
    editor.set_selection(Selection::caret(result.caret), cx);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_block_is_held() {
        let held = RuleResult {
            edits: Vec::new(),
            caret: 0,
            rule: "math_block",
        };
        let code = RuleResult {
            rule: "code_fence",
            ..held.clone()
        };
        assert!(!allowed(&held));
        assert!(allowed(&code));
    }
}
