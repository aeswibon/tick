//! Footer text input: multiline modes, paste, and submit vs newline.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::InputMode;

/// Modes where Enter inserts a newline (Shift+Enter or Alt+Enter) and only plain Enter submits.
pub fn multiline_input_mode(mode: InputMode) -> bool {
    matches!(
        mode,
        InputMode::Comment
            | InputMode::EditDescription
            | InputMode::CreateDescription
            | InputMode::TemplateEditDescription
    )
}

pub fn should_insert_newline_on_enter(key: &KeyEvent) -> bool {
    key.code == KeyCode::Enter
        && (key.modifiers.contains(KeyModifiers::SHIFT)
            || key.modifiers.contains(KeyModifiers::ALT))
}

#[allow(dead_code)]
pub fn should_submit_on_enter(key: &KeyEvent, mode: InputMode) -> bool {
    key.code == KeyCode::Enter
        && !should_insert_newline_on_enter(key)
        && (!multiline_input_mode(mode) || !key.modifiers.intersects(KeyModifiers::CONTROL))
}

/// Insert pasted text without treating embedded newlines as separate Enter events.
pub fn insert_paste(buffer: &mut String, text: &str) {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    buffer.push_str(&normalized);
}

/// Buffer for footer submit: preserve internal newlines in multiline modes.
pub fn buffer_for_submit(buffer: &str, mode: InputMode) -> String {
    if multiline_input_mode(mode) {
        buffer.trim_end().to_string()
    } else {
        buffer.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn multiline_modes_include_comment() {
        assert!(multiline_input_mode(InputMode::Comment));
        assert!(!multiline_input_mode(InputMode::Worklog));
    }

    #[test]
    fn shift_enter_is_newline_not_submit() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT);
        assert!(should_insert_newline_on_enter(&key));
        assert!(!should_submit_on_enter(&key, InputMode::Comment));
    }

    #[test]
    fn plain_enter_submits_comment() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        assert!(should_submit_on_enter(&key, InputMode::Comment));
    }

    #[test]
    fn paste_normalizes_crlf() {
        let mut b = String::new();
        insert_paste(&mut b, "a\r\nb");
        assert_eq!(b, "a\nb");
    }

    #[test]
    fn submit_preserves_internal_newlines() {
        let s = buffer_for_submit("line1\nline2\n", InputMode::Comment);
        assert_eq!(s, "line1\nline2");
    }
}
