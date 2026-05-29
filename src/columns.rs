use ratatui::{text::Span, widgets::Cell};

use crate::api::types::Ticket;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Site,
    Key,
    Type,
    Status,
    Priority,
    Age,
    Due,
    Assignee,
    Reporter,
    Summary,
}

impl Column {
    pub fn id(self) -> &'static str {
        match self {
            Self::Site => "site",
            Self::Key => "key",
            Self::Type => "type",
            Self::Status => "status",
            Self::Priority => "priority",
            Self::Age => "age",
            Self::Due => "due",
            Self::Assignee => "assignee",
            Self::Reporter => "reporter",
            Self::Summary => "summary",
        }
    }

    pub fn header(self) -> &'static str {
        match self {
            Self::Site => " Site",
            Self::Key => "Key",
            Self::Type => "Type",
            Self::Status => "Status",
            Self::Priority => "Priority",
            Self::Age => "Age",
            Self::Due => "Due",
            Self::Assignee => "Assignee",
            Self::Reporter => "Reporter",
            Self::Summary => "Summary",
        }
    }

    pub fn parse_id(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "site" => Some(Self::Site),
            "key" => Some(Self::Key),
            "type" | "issuetype" => Some(Self::Type),
            "status" => Some(Self::Status),
            "priority" => Some(Self::Priority),
            "age" | "ageing" => Some(Self::Age),
            "due" | "duedate" => Some(Self::Due),
            "assignee" => Some(Self::Assignee),
            "reporter" => Some(Self::Reporter),
            "summary" => Some(Self::Summary),
            _ => None,
        }
    }

    pub fn default_set() -> Vec<Self> {
        vec![
            Self::Site,
            Self::Key,
            Self::Type,
            Self::Status,
            Self::Priority,
            Self::Age,
            Self::Due,
            Self::Assignee,
            Self::Reporter,
        ]
    }

    pub fn resolve(config: Option<&[String]>) -> Vec<Self> {
        let Some(names) = config else {
            return Self::default_set();
        };
        let parsed: Vec<Self> = names.iter().filter_map(|n| Self::parse_id(n)).collect();
        if parsed.is_empty() {
            Self::default_set()
        } else {
            parsed
        }
    }

    pub fn cell(self, ticket: &Ticket, theme: &Theme) -> Cell<'static> {
        match self {
            Self::Site => Cell::from(Span::raw(format!(" {}", ticket.site))),
            Self::Key => Cell::from(Span::raw(ticket.key.clone())),
            Self::Type => Cell::from(Span::raw(ticket.issue_type.clone())),
            Self::Status => Cell::from(Span::styled(
                ticket.status.clone(),
                theme.status_style(&ticket.status_color),
            )),
            Self::Priority => Cell::from(Span::styled(
                ticket.priority.clone(),
                theme.priority_style(&ticket.priority),
            )),
            Self::Age => Cell::from(Span::raw(format!("{}d", ticket.ageing_days))),
            Self::Due => {
                let due = ticket
                    .due_date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "-".to_string());
                Cell::from(Span::raw(due))
            }
            Self::Assignee => Cell::from(Span::raw(ticket.assignee.clone())),
            Self::Reporter => Cell::from(Span::raw(ticket.reporter.clone())),
            Self::Summary => Cell::from(Span::raw(ticket.summary.clone())),
        }
    }

    pub fn width_hint(self) -> u16 {
        match self {
            Self::Site => 10,
            Self::Key => 12,
            Self::Type => 12,
            Self::Status => 16,
            Self::Priority => 8,
            Self::Age => 6,
            Self::Due => 12,
            Self::Assignee => 12,
            Self::Reporter => 12,
            Self::Summary => 24,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_columns_match_legacy_table() {
        let cols = Column::default_set();
        assert_eq!(cols.len(), 9);
        assert_eq!(cols[0], Column::Site);
        assert_eq!(cols[1], Column::Key);
    }

    #[test]
    fn parse_column_ids() {
        assert_eq!(Column::parse_id("ISSUETYPE"), Some(Column::Type));
        assert_eq!(Column::parse_id("summary"), Some(Column::Summary));
        assert_eq!(Column::parse_id("nope"), None);
    }

    #[test]
    fn invalid_config_falls_back_to_default() {
        let cols = Column::resolve(Some(&["bad".into()]));
        assert_eq!(cols, Column::default_set());
    }

    #[test]
    fn custom_column_order_preserved() {
        let cols = Column::resolve(Some(&["key".into(), "summary".into()]));
        assert_eq!(cols, vec![Column::Key, Column::Summary]);
    }
}
