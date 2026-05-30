use super::load_more_users_key;
use super::mentions::{active_mention_query, mentions_enabled};
use super::transitions::{transition_user_field_key_action, TransitionUserFieldKeyAction};
use crate::app::InputMode;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

#[test]
fn load_more_users_accepts_modifier_r() {
    assert!(load_more_users_key(&key(
        KeyCode::Char('r'),
        KeyModifiers::CONTROL
    )));
    assert!(load_more_users_key(&key(
        KeyCode::Char('r'),
        KeyModifiers::SUPER
    )));
    assert!(load_more_users_key(&key(
        KeyCode::Char('r'),
        KeyModifiers::META
    )));
    assert!(!load_more_users_key(&key(
        KeyCode::Char('r'),
        KeyModifiers::empty()
    )));
    assert!(!load_more_users_key(&key(
        KeyCode::Char('R'),
        KeyModifiers::SHIFT
    )));
}

#[test]
fn mentions_enabled_only_for_comment_and_description() {
    assert!(mentions_enabled(InputMode::Comment));
    assert!(mentions_enabled(InputMode::EditDescription));
    assert!(!mentions_enabled(InputMode::None));
    assert!(!mentions_enabled(InputMode::TransitionField));
    assert!(!mentions_enabled(InputMode::EditSummary));
}

#[test]
fn transition_user_field_plain_r_passes_to_input() {
    assert_eq!(
        transition_user_field_key_action(&key(KeyCode::Char('r'), KeyModifiers::empty()), true),
        TransitionUserFieldKeyAction::PassToInput
    );
    assert_eq!(
        transition_user_field_key_action(&key(KeyCode::Char('R'), KeyModifiers::SHIFT), true),
        TransitionUserFieldKeyAction::PassToInput
    );
}

#[test]
fn transition_user_field_modifier_r_loads_more() {
    assert_eq!(
        transition_user_field_key_action(&key(KeyCode::Char('r'), KeyModifiers::CONTROL), false),
        TransitionUserFieldKeyAction::LoadMoreUsers
    );
}

#[test]
fn transition_user_field_j_k_only_when_options() {
    assert_eq!(
        transition_user_field_key_action(&key(KeyCode::Char('j'), KeyModifiers::empty()), true),
        TransitionUserFieldKeyAction::MoveDown
    );
    assert_eq!(
        transition_user_field_key_action(&key(KeyCode::Char('k'), KeyModifiers::empty()), true),
        TransitionUserFieldKeyAction::MoveUp
    );
    assert_eq!(
        transition_user_field_key_action(&key(KeyCode::Char('j'), KeyModifiers::empty()), false),
        TransitionUserFieldKeyAction::PassToInput
    );
}

#[test]
fn transition_user_field_numeric_pick() {
    assert_eq!(
        transition_user_field_key_action(&key(KeyCode::Char('3'), KeyModifiers::empty()), true),
        TransitionUserFieldKeyAction::PickIndex(2)
    );
    assert_eq!(
        transition_user_field_key_action(&key(KeyCode::Char('3'), KeyModifiers::empty()), false),
        TransitionUserFieldKeyAction::PassToInput
    );
}

#[test]
fn detects_query_after_at() {
    let (pos, q) = active_mention_query("hello @ali").unwrap();
    assert_eq!(pos, 6);
    assert_eq!(q, "ali");
}

#[test]
fn rejects_completed_mention_with_space() {
    assert!(active_mention_query("hey @Alice done").is_none());
}

#[test]
fn uses_last_at_sign() {
    let (pos, q) = active_mention_query("@a @bob").unwrap();
    assert_eq!(pos, 3);
    assert_eq!(q, "bob");
}

#[test]
fn empty_query_after_at_is_valid() {
    let (pos, q) = active_mention_query("cc @").unwrap();
    assert_eq!(pos, 3);
    assert_eq!(q, "");
}
