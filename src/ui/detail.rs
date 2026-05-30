use crate::app::{App, DetailTab};
use crate::ticket_lock::read_tickets;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

fn tab_style(active: bool, dl: ratatui::style::Color, dv: ratatui::style::Color) -> Style {
    if active {
        Style::default()
            .fg(dl)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default().fg(dv)
    }
}

pub fn draw_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    super::table::draw_table(f, app, chunks[0]);

    let tickets = read_tickets(&app.tickets);
    let Some(ticket_idx) = app.selected_ticket_index() else {
        return;
    };
    let ticket = &tickets[ticket_idx];

    let dl = app.theme.detail_label;
    let dv = app.theme.detail_value;

    let tab_line = Line::from(vec![
        Span::styled(
            " Details ",
            tab_style(app.detail_tab == DetailTab::Details, dl, dv),
        ),
        Span::styled(" | ", Style::default().fg(app.theme.border)),
        Span::styled(
            " Description ",
            tab_style(app.detail_tab == DetailTab::Description, dl, dv),
        ),
        Span::styled(" | ", Style::default().fg(app.theme.border)),
        Span::styled(
            " Comments ",
            tab_style(app.detail_tab == DetailTab::Comments, dl, dv),
        ),
        Span::styled(" | ", Style::default().fg(app.theme.border)),
        Span::styled(
            " Links ",
            tab_style(app.detail_tab == DetailTab::Links, dl, dv),
        ),
    ]);

    let mut lines = vec![tab_line, Line::from("")];

    match app.detail_tab {
        DetailTab::Details => {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<12}", "Key:"),
                    Style::default().fg(dl).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&ticket.key, Style::default().fg(dv)),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<12}", "Site:"),
                    Style::default().fg(dl).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&ticket.site, Style::default().fg(dv)),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<12}", "Type:"),
                    Style::default().fg(dl).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&ticket.issue_type, Style::default().fg(dv)),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<12}", "Status:"),
                    Style::default().fg(dl).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&ticket.status, app.theme.status_style(&ticket.status_color)),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<12}", "Priority:"),
                    Style::default().fg(dl).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&ticket.priority, app.theme.priority_style(&ticket.priority)),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<12}", "Assignee:"),
                    Style::default().fg(dl).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&ticket.assignee, Style::default().fg(dv)),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<12}", "Reporter:"),
                    Style::default().fg(dl).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&ticket.reporter, Style::default().fg(dv)),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<12}", "Age:"),
                    Style::default().fg(dl).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} days", ticket.ageing_days),
                    Style::default().fg(dv),
                ),
            ]));

            if let Some(due) = ticket.due_date {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{:<12}", "Due:"),
                        Style::default().fg(dl).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(due.to_string(), Style::default().fg(dv)),
                ]));
            }

            if let Some(ref pk) = ticket.parent_key {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{:<12}", "Parent:"),
                        Style::default().fg(dl).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(pk.clone(), Style::default().fg(dv)),
                ]));
                if let Some(ref ps) = ticket.parent_summary {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", ps),
                        Style::default().fg(dv),
                    )));
                }
            }

            if !ticket.labels.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{:<12}", "Labels:"),
                        Style::default().fg(dl).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(ticket.labels.join(", "), Style::default().fg(dv)),
                ]));
            }

            if let Some(ref sprint) = ticket.sprint_name {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{:<12}", "Sprint:"),
                        Style::default().fg(dl).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(sprint.clone(), Style::default().fg(dv)),
                ]));
            }

            let mut mentioned = Vec::new();
            if let Some(ref adf) = ticket.description_adf {
                mentioned.extend(crate::api::types::collect_mention_labels(adf));
            }
            for comment in &ticket.all_comments {
                if let Some(ref body) = comment.body {
                    mentioned.extend(crate::api::types::collect_mention_labels(body));
                }
            }
            mentioned.sort();
            mentioned.dedup();
            if !mentioned.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{:<12}", "Mentioned:"),
                        Style::default().fg(dl).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(mentioned.join(", "), Style::default().fg(dv)),
                ]));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Summary",
                Style::default().fg(dl).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                format!("  {}", ticket.summary),
                Style::default().fg(dv),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Link",
                Style::default().fg(dl).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                format!("  {}", ticket.link),
                Style::default().fg(dv),
            )));
            lines.push(Line::from(""));

            if let Some(ref pk) = ticket.parent_key {
                lines.push(Line::from(Span::styled(
                    "Parent Link",
                    Style::default().fg(dl).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(Span::styled(
                    format!(
                        "  {}/browse/{}",
                        ticket
                            .link
                            .trim_end_matches(&ticket.key)
                            .trim_end_matches('/'),
                        pk
                    ),
                    Style::default().fg(dv),
                )));
                lines.push(Line::from(""));
            }
        }
        DetailTab::Description => {
            if let Some(ref adf) = ticket.description_adf {
                let rendered = crate::ui::adf::render_doc(adf);
                if rendered.is_empty() || (rendered.len() == 1 && rendered[0].spans.is_empty()) {
                    lines.push(Line::from(Span::styled(
                        "  No description",
                        Style::default().fg(app.theme.border),
                    )));
                } else {
                    lines.extend(rendered);
                }
            } else if let Some(ref desc) = ticket.description {
                for line in desc.lines() {
                    lines.push(Line::from(Span::styled(line, Style::default().fg(dv))));
                }
            } else {
                lines.push(Line::from(Span::styled(
                    "  No description",
                    Style::default().fg(app.theme.border),
                )));
            }
        }
        DetailTab::Comments => {
            if ticket.all_comments.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  No comments",
                    Style::default().fg(app.theme.border),
                )));
            } else {
                for (i, comment) in ticket.all_comments.iter().enumerate() {
                    if i > 0 {
                        lines.push(Line::from(""));
                        lines.push(Line::from(Span::styled(
                            "────",
                            Style::default().fg(app.theme.border),
                        )));
                        lines.push(Line::from(""));
                    }
                    lines.push(Line::from(vec![
                        Span::styled(
                            comment.author.clone(),
                            Style::default().fg(dl).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            comment.created.clone(),
                            Style::default().fg(app.theme.border),
                        ),
                    ]));
                    if let Some(ref adf_body) = comment.body {
                        let rendered = crate::ui::adf::render_doc(adf_body);
                        if rendered.is_empty()
                            || (rendered.len() == 1 && rendered[0].spans.is_empty())
                        {
                            lines.push(Line::from(Span::styled(
                                "    (empty)",
                                Style::default().fg(app.theme.border),
                            )));
                        } else {
                            for rline in rendered {
                                let mut spans = vec![Span::raw("  ")];
                                spans.extend(rline.spans);
                                lines.push(Line::from(spans));
                            }
                        }
                    }
                }
            }
        }
        DetailTab::Links => {
            lines.push(Line::from(Span::styled(
                "  j/k · Enter jump · o open · I link · Shift+I remove · Shift+N subtask",
                Style::default().fg(app.theme.border),
            )));
            lines.push(Line::from(""));
            if app.loading && app.issue_relations.is_none() {
                lines.push(Line::from(Span::styled(
                    "  Loading…",
                    Style::default().fg(app.theme.border),
                )));
            } else if let Some(ref rel) = app.issue_relations {
                let mut row = 0usize;
                lines.push(Line::from(Span::styled(
                    "Issue links",
                    Style::default().fg(dl).add_modifier(Modifier::BOLD),
                )));
                if rel.links.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "  (none)",
                        Style::default().fg(app.theme.border),
                    )));
                } else {
                    for link in &rel.links {
                        let is_sel = row == app.links_selected;
                        row += 1;
                        let marker = if is_sel { "›" } else { " " };
                        let row_style = if is_sel {
                            Style::default()
                                .fg(app.theme.accent)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(dv)
                        };
                        lines.push(Line::from(vec![
                            Span::styled(format!(" {marker} "), row_style),
                            Span::styled(format!("{} ", link.other_key), row_style),
                            Span::styled(
                                format!("{} · ", link.direction),
                                Style::default().fg(app.theme.border),
                            ),
                            Span::styled(link.other_status.clone(), Style::default().fg(dv)),
                        ]));
                        lines.push(Line::from(Span::styled(
                            format!("    {}", link.other_summary),
                            Style::default().fg(dv),
                        )));
                    }
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Subtasks",
                    Style::default().fg(dl).add_modifier(Modifier::BOLD),
                )));
                if rel.subtasks.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "  (none)",
                        Style::default().fg(app.theme.border),
                    )));
                } else {
                    for st in &rel.subtasks {
                        let is_sel = row == app.links_selected;
                        row += 1;
                        let marker = if is_sel { "›" } else { " " };
                        let row_style = if is_sel {
                            Style::default()
                                .fg(app.theme.accent)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(dv)
                        };
                        lines.push(Line::from(vec![
                            Span::styled(format!(" {marker} "), row_style),
                            Span::styled(format!("{} ", st.key), row_style),
                            Span::styled(
                                format!("{} · ", st.status),
                                Style::default().fg(app.theme.border),
                            ),
                            Span::styled(st.summary.clone(), Style::default().fg(dv)),
                        ]));
                    }
                }
            } else {
                lines.push(Line::from(Span::styled(
                    "  Open this tab or press Enter on a ticket to load",
                    Style::default().fg(app.theme.border),
                )));
            }
        }
    }

    let detail = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.detail_border))
                .title(" Ticket Detail "),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(detail, chunks[1]);
}
