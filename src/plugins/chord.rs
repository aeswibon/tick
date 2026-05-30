//! Parse and match plugin key chords (`ctrl+shift+h`).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord {
    pub modifiers: KeyModifiers,
    pub code: KeyCode,
}

/// Canonical chord label for manifest matching and Lua `on_key(chord)`.
pub fn format_key(key: &KeyEvent) -> String {
    let mut parts = modifier_names(key.modifiers);
    parts.push(code_name(key.code));
    parts.join("+")
}

pub fn parse_chord(raw: &str) -> Result<Chord, String> {
    let raw = raw.trim().to_lowercase();
    if raw.is_empty() {
        return Err("empty chord".into());
    }
    let segments: Vec<&str> = raw
        .split('+')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if segments.is_empty() {
        return Err("empty chord".into());
    }
    let key_part = *segments
        .last()
        .ok_or_else(|| "chord missing key".to_string())?;
    let mut modifiers = KeyModifiers::empty();
    for seg in &segments[..segments.len().saturating_sub(1)] {
        match *seg {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            "alt" | "option" => modifiers |= KeyModifiers::ALT,
            "super" | "meta" | "cmd" | "command" => modifiers |= KeyModifiers::SUPER,
            other => return Err(format!("unknown modifier '{other}'")),
        }
    }
    let code = parse_code(key_part)?;
    Ok(Chord { modifiers, code })
}

fn parse_code(name: &str) -> Result<KeyCode, String> {
    match name {
        "space" => Ok(KeyCode::Char(' ')),
        "tab" => Ok(KeyCode::Tab),
        "enter" => Ok(KeyCode::Enter),
        "esc" | "escape" => Ok(KeyCode::Esc),
        "backspace" => Ok(KeyCode::Backspace),
        "up" => Ok(KeyCode::Up),
        "down" => Ok(KeyCode::Down),
        "left" => Ok(KeyCode::Left),
        "right" => Ok(KeyCode::Right),
        "pageup" => Ok(KeyCode::PageUp),
        "pagedown" => Ok(KeyCode::PageDown),
        "home" => Ok(KeyCode::Home),
        "end" => Ok(KeyCode::End),
        s if s.len() == 1 => {
            let c = s.chars().next().unwrap();
            Ok(KeyCode::Char(c))
        }
        s if s.starts_with('f') && s.len() <= 3 => s[1..]
            .parse::<u8>()
            .ok()
            .filter(|&n| (1..=12).contains(&n))
            .map(KeyCode::F)
            .ok_or_else(|| format!("unknown key '{name}'")),
        other => Err(format!("unknown key '{other}'")),
    }
}

fn modifier_names(mods: KeyModifiers) -> Vec<String> {
    let mut names = Vec::new();
    if mods.contains(KeyModifiers::CONTROL) {
        names.push("ctrl".into());
    }
    if mods.contains(KeyModifiers::ALT) {
        names.push("alt".into());
    }
    if mods.contains(KeyModifiers::SHIFT) {
        names.push("shift".into());
    }
    if mods.contains(KeyModifiers::SUPER) {
        names.push("super".into());
    }
    names.sort();
    names
}

fn code_name(code: KeyCode) -> String {
    match code {
        KeyCode::Char(' ') => "space".into(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Tab => "tab".into(),
        KeyCode::Enter => "enter".into(),
        KeyCode::Esc => "esc".into(),
        KeyCode::Backspace => "backspace".into(),
        KeyCode::Up => "up".into(),
        KeyCode::Down => "down".into(),
        KeyCode::Left => "left".into(),
        KeyCode::Right => "right".into(),
        KeyCode::PageUp => "pageup".into(),
        KeyCode::PageDown => "pagedown".into(),
        KeyCode::Home => "home".into(),
        KeyCode::End => "end".into(),
        KeyCode::F(n) => format!("f{n}"),
        _ => "unknown".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    #[test]
    fn parse_and_match_ctrl_shift_h() {
        let parsed = parse_chord("ctrl+shift+h").unwrap();
        let event = key(
            KeyCode::Char('h'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        assert_eq!(parsed.code, event.code);
        assert_eq!(parsed.modifiers, event.modifiers);
        assert_eq!(format_key(&event), "ctrl+shift+h");
    }

    #[test]
    fn rejects_unknown_modifier() {
        assert!(parse_chord("win+h").is_err());
    }
}
