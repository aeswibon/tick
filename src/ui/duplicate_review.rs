use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::create_flow::{CreateSession, DuplicateReviewPhase};
use crate::template_export::TemplateFieldRow;
use crate::theme::Theme;

pub fn draw_duplicate_review(f: &mut Frame, session: &CreateSession, theme: &Theme, area: Rect) {
    let (title, hint) = match session.duplicate_review_phase {
        DuplicateReviewPhase::IncludeFields => (
            "Duplicate — fields to copy",
            "Space toggle · Enter next · Esc cancel",
        ),
        DuplicateReviewPhase::ClearValues => (
            "Duplicate — leave blank for manual entry",
            "Space toggle clear · Enter continue · Esc cancel",
        ),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .style(Style::default().fg(theme.border));

    let items: Vec<ListItem> = session
        .duplicate_field_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            ListItem::new(format_duplicate_row(row, session.duplicate_review_phase)).style(
                if i == session.duplicate_field_selected {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    Style::default().fg(theme.detail_value)
                },
            )
        })
        .collect();

    f.render_widget(
        List::new(items)
            .block(block)
            .highlight_style(Style::default()),
        area,
    );

    let footer = Line::from(Span::styled(
        format!("  {hint}"),
        Style::default().fg(theme.footer_fg),
    ));
    let footer_area = Rect {
        y: area.y + area.height.saturating_sub(1),
        height: 1,
        ..area
    };
    f.render_widget(
        ratatui::widgets::Paragraph::new(footer).style(Style::default().bg(theme.footer_bg)),
        footer_area,
    );
}

fn format_duplicate_row(row: &TemplateFieldRow, phase: DuplicateReviewPhase) -> Line<'static> {
    let mark = match phase {
        DuplicateReviewPhase::IncludeFields => {
            if row.include {
                "[x]"
            } else {
                "[ ]"
            }
        }
        DuplicateReviewPhase::ClearValues => {
            if !row.include {
                return Line::from(format!("  — {} (excluded)", row.label));
            }
            if row.clear_value {
                "[clear]"
            } else {
                "[keep]"
            }
        }
    };
    Line::from(format!("  {mark} {:<20} {}", row.label, row.preview))
}
