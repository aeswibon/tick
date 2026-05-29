use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw_help(f: &mut Frame, _app: &App, area: Rect) {
    let popup = centered_rect(60, 75, area);
    f.render_widget(Clear, popup);

    let lines = vec![
        Line::from(Span::styled(
            " Help",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Navigation",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )),
        Line::from("  j/k  or  Up/Down      Move selection"),
        Line::from("  [ / ]                  Scroll up / down by page_size rows"),
        Line::from("  g / G                  Go to first / last row"),
        Line::from("  1–5                    Jump to view tab"),
        Line::from(""),
        Line::from(Span::styled(
            " Actions",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )),
        Line::from("  Enter                  Toggle detail pane"),
        Line::from("  Esc                    Close pane / help / overlay"),
        Line::from("  r                      Refresh tickets"),
        Line::from("  o                      Open selected ticket in browser"),
        Line::from(
            "  O                      Open ticket from clipboard/key (probes sites if needed)",
        ),
        Line::from("  n                      Create new issue (blank)"),
        Line::from("  N                      Create from config template"),
        Line::from("  C                      Duplicate selected issue (maximal field copy)"),
        Line::from("  y                      Copy ticket key to clipboard"),
        Line::from("  e                      Open config file in editor"),
        Line::from(""),
        Line::from(Span::styled(
            " Detail Pane",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )),
        Line::from("  h / l                  Prev / next detail tab"),
        Line::from(""),
        Line::from(Span::styled(
            " View",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )),
        Line::from("  ?                      Toggle this help"),
        Line::from("  /                      Filter tickets"),
        Line::from("  s                      Cycle sort field"),
        Line::from("  S                      Toggle sort asc ↑ / desc ↓ (table)"),
        Line::from("  ← / →                  Cycle view (pane closed)"),
        Line::from("  Tab / Shift+Tab        Cycle view (pane closed)"),
        Line::from(""),
        Line::from(Span::styled(
            " Jira Actions",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )),
        Line::from("  t / T                  Change status (workflow transition picker)"),
        Line::from("  c                      Add comment (@ tags users)"),
        Line::from("  w                      Log work time"),
        Line::from("  a / u                  Assign to me / unassign (detail open)"),
        Line::from("  S / P / L / M / D      Edit fields (D: markdown + @mentions)"),
        Line::from("  !                      Toggle site error overlay"),
        Line::from(""),
        Line::from(Span::styled(
            " General",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )),
        Line::from("  q                      Quit"),
    ];

    let help = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Keybindings "),
        )
        .wrap(Wrap { trim: false })
        .style(Style::default());

    f.render_widget(help, popup);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length((r.height * (100 - percent_y)) / 200),
            Constraint::Min(1),
            Constraint::Length((r.height * (100 - percent_y)) / 200),
        ])
        .split(r);

    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Length((r.width * (100 - percent_x)) / 200),
            Constraint::Min(1),
            Constraint::Length((r.width * (100 - percent_x)) / 200),
        ])
        .split(popup_layout[1])[1]
}
