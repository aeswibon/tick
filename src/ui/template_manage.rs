use crate::template_manage_flow::{TemplateManageSession, TemplateManageStep};
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn draw_template_manage(
    f: &mut Frame,
    session: &TemplateManageSession,
    theme: &Theme,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(" Templates (Shift+E) ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    match session.step {
        TemplateManageStep::List => draw_list(f, session, theme, inner),
        TemplateManageStep::Actions => draw_actions(f, session, theme, inner),
        TemplateManageStep::ConfirmDelete => draw_confirm_delete(f, session, theme, inner),
        TemplateManageStep::EditSummary
        | TemplateManageStep::EditProject
        | TemplateManageStep::EditIssueType
        | TemplateManageStep::EditDescription
        | TemplateManageStep::EditLabels => {}
    }
}

fn draw_list(f: &mut Frame, session: &TemplateManageSession, theme: &Theme, area: Rect) {
    let items: Vec<ListItem> = session
        .names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let style = if i == session.selected {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            let prefix = if i == session.selected { "› " } else { "  " };
            ListItem::new(Line::from(vec![Span::styled(
                format!("{prefix}{name}"),
                style,
            )]))
        })
        .collect();
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::NONE)
            .title(" j/k move · Enter actions · Esc close "),
    );
    f.render_widget(list, area);
}

fn draw_actions(f: &mut Frame, session: &TemplateManageSession, theme: &Theme, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(4)])
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::styled("Template: ", Style::default().fg(theme.border)),
        Span::styled(
            &session.editing_name,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    f.render_widget(title, chunks[0]);

    let lines = vec![
        " e  edit summary",
        " p  edit project",
        " i  edit issue type",
        " b  edit description (markdown)",
        " l  edit labels (comma-separated)",
        " d  delete template",
        "",
        " Esc  back to list",
    ];
    let body: Vec<Line> = lines
        .into_iter()
        .map(|l| Line::from(Span::raw(l)))
        .collect();
    f.render_widget(Paragraph::new(body), chunks[1]);
}

fn draw_confirm_delete(f: &mut Frame, session: &TemplateManageSession, theme: &Theme, area: Rect) {
    let text = format!(
        "Delete template '{}'?\n\nEnter confirm · Esc cancel",
        session.editing_name
    );
    let p = Paragraph::new(text).style(Style::default().fg(theme.accent));
    f.render_widget(p, area);
}
