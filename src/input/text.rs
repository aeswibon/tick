//! Footer text input: multiline modes, paste, cursor, and submit vs newline.

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

pub fn clamp_cursor(buffer: &str, cursor: usize) -> usize {
    let cursor = cursor.min(buffer.len());
    if buffer.is_char_boundary(cursor) {
        cursor
    } else {
        (0..=cursor)
            .rev()
            .find(|&i| buffer.is_char_boundary(i))
            .unwrap_or(0)
    }
}

pub fn cursor_left(buffer: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(buffer, cursor);
    if cursor == 0 {
        return 0;
    }
    buffer[..cursor]
        .char_indices()
        .last()
        .map(|(i, _)| i)
        .unwrap_or(0)
}

pub fn cursor_right(buffer: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(buffer, cursor);
    if cursor >= buffer.len() {
        return buffer.len();
    }
    buffer[cursor..]
        .char_indices()
        .nth(1)
        .map(|(i, _)| cursor + i)
        .unwrap_or(buffer.len())
}

pub fn cursor_home(buffer: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(buffer, cursor);
    buffer[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

pub fn cursor_end(buffer: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(buffer, cursor);
    buffer[cursor..]
        .find('\n')
        .map(|i| cursor + i)
        .unwrap_or(buffer.len())
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn cursor_word_left(buffer: &str, cursor: usize) -> usize {
    let mut i = clamp_cursor(buffer, cursor);
    if i == 0 {
        return 0;
    }
    while i > 0 {
        let prev = cursor_left(buffer, i);
        if is_word_char(buffer[prev..i].chars().next().unwrap_or(' ')) {
            break;
        }
        i = prev;
    }
    while i > 0 {
        let prev = cursor_left(buffer, i);
        if !is_word_char(buffer[prev..i].chars().next().unwrap_or(' ')) {
            break;
        }
        i = prev;
    }
    i
}

pub fn cursor_word_right(buffer: &str, cursor: usize) -> usize {
    let len = buffer.len();
    let mut i = clamp_cursor(buffer, cursor);
    if i >= len {
        return len;
    }
    if buffer
        .get(i..)
        .and_then(|s| s.chars().next())
        .map(is_word_char)
        .unwrap_or(false)
    {
        while i < len {
            let next = cursor_right(buffer, i);
            let c = buffer[i..next].chars().next().unwrap_or(' ');
            if !is_word_char(c) {
                break;
            }
            i = next;
        }
    }
    while i < len {
        let c = buffer[i..].chars().next().unwrap_or(' ');
        if is_word_char(c) {
            return i;
        }
        i = cursor_right(buffer, i);
    }
    len
}

pub fn delete_word_backward(buffer: &mut String, cursor: &mut usize) {
    let target = cursor_word_left(buffer, *cursor);
    *cursor = clamp_cursor(buffer, *cursor);
    buffer.drain(target..*cursor);
    *cursor = target;
}

pub fn delete_forward(buffer: &mut String, cursor: &mut usize) {
    *cursor = clamp_cursor(buffer, *cursor);
    if *cursor >= buffer.len() {
        return;
    }
    let next = cursor_right(buffer, *cursor);
    buffer.drain(*cursor..next);
}

pub fn insert_char(buffer: &mut String, cursor: &mut usize, c: char) {
    *cursor = clamp_cursor(buffer, *cursor);
    let s = c.to_string();
    buffer.insert_str(*cursor, &s);
    *cursor += s.len();
}

pub fn backspace_at(buffer: &mut String, cursor: &mut usize) {
    *cursor = clamp_cursor(buffer, *cursor);
    if *cursor == 0 {
        return;
    }
    let prev = cursor_left(buffer, *cursor);
    buffer.drain(prev..*cursor);
    *cursor = prev;
}

/// Insert pasted text without treating embedded newlines as separate Enter events.
pub fn insert_paste(buffer: &mut String, text: &str) {
    let mut cursor = buffer.len();
    insert_paste_at(buffer, &mut cursor, text);
}

pub fn insert_paste_at(buffer: &mut String, cursor: &mut usize, text: &str) {
    *cursor = clamp_cursor(buffer, *cursor);
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    buffer.insert_str(*cursor, &normalized);
    *cursor += normalized.len();
}

/// Render footer buffer with a visible cursor (`█`).
pub fn format_with_cursor(buffer: &str, cursor: usize) -> String {
    let cursor = clamp_cursor(buffer, cursor);
    let (before, after) = buffer.split_at(cursor);
    format!("{before}█{after}")
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

    #[test]
    fn cursor_moves_by_char() {
        let s = "hello";
        assert_eq!(cursor_left(s, 3), 2);
        assert_eq!(cursor_right(s, 2), 3);
    }

    #[test]
    fn home_end_respect_newlines() {
        let s = "one\ntwo";
        assert_eq!(cursor_home(s, 6), 4);
        assert_eq!(cursor_end(s, 4), 7);
    }

    #[test]
    fn insert_and_backspace_at_cursor() {
        let mut b = "helo".to_string();
        let mut c = 2;
        insert_char(&mut b, &mut c, 'l');
        assert_eq!(b, "hello");
        assert_eq!(c, 3);
        backspace_at(&mut b, &mut c);
        assert_eq!(b, "helo");
        assert_eq!(c, 2);
    }

    #[test]
    fn word_motion_skips_tokens() {
        let s = "hello world-test";
        assert_eq!(cursor_word_left(s, 16), 12);
        assert_eq!(cursor_word_left(s, 12), 6);
        assert_eq!(cursor_word_right(s, 0), 6);
        assert_eq!(cursor_word_right(s, 6), 12);
    }

    #[test]
    fn delete_word_backward_removes_prior_token() {
        let mut b = "one two".to_string();
        let mut c = b.len();
        delete_word_backward(&mut b, &mut c);
        assert_eq!(b, "one ");
        assert_eq!(c, 4);
    }

    #[test]
    fn delete_forward_removes_next_char() {
        let mut b = "abcd".to_string();
        let mut c = 1;
        delete_forward(&mut b, &mut c);
        assert_eq!(b, "acd");
        assert_eq!(c, 1);
    }

    #[test]
    fn format_shows_cursor_marker() {
        assert_eq!(format_with_cursor("abc", 1), "a█bc");
    }
}
