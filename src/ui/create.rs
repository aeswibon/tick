use ratatui::Frame;

use crate::app::App;
use crate::create_flow::CreateStep;

pub fn draw_create_picker(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let Some(session) = app.create_session.as_ref() else {
        return;
    };
    let (title, subtitle) = match session.step {
        CreateStep::Site => ("Create — site", "Choose Jira site"),
        CreateStep::Project => ("Create — project", "Choose project (p to reopen)"),
        CreateStep::IssueType => ("Create — type", "Choose issue type (t to reopen)"),
        CreateStep::Template => ("Create — template", "Pre-filled issue; edit summary next"),
        _ => ("Create", ""),
    };
    super::field_picker::draw_field_picker(
        f,
        title,
        subtitle,
        "",
        &session.picker_options,
        session.picker_selected,
        false,
        false,
        &app.theme,
        area,
    );
}
