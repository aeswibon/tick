use ratatui::{text::Span, widgets::Cell};

use crate::api::types::Ticket;
use crate::theme::Theme;

#[derive(Debug, Clone, PartialEq, Eq)]
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
    Parent,
    Labels,
    Sprint,
    Summary,
    /// Read-only Jira custom field (`customfield_*` id from config).
    Custom(String),
}

impl Column {
    #[allow(dead_code)]
    pub fn id(self) -> String {
        match self {
            Self::Site => "site".into(),
            Self::Key => "key".into(),
            Self::Type => "type".into(),
            Self::Status => "status".into(),
            Self::Priority => "priority".into(),
            Self::Age => "age".into(),
            Self::Due => "due".into(),
            Self::Assignee => "assignee".into(),
            Self::Reporter => "reporter".into(),
            Self::Parent => "parent".into(),
            Self::Labels => "labels".into(),
            Self::Sprint => "sprint".into(),
            Self::Summary => "summary".into(),
            Self::Custom(id) => id,
        }
    }

    pub fn header(&self) -> String {
        match self {
            Self::Site => " Site".into(),
            Self::Key => "Key".into(),
            Self::Type => "Type".into(),
            Self::Status => "Status".into(),
            Self::Priority => "Priority".into(),
            Self::Age => "Age".into(),
            Self::Due => "Due".into(),
            Self::Assignee => "Assignee".into(),
            Self::Reporter => "Reporter".into(),
            Self::Parent => "Parent".into(),
            Self::Labels => "Labels".into(),
            Self::Sprint => "Sprint".into(),
            Self::Summary => "Summary".into(),
            Self::Custom(id) => {
                if let Some(n) = id.strip_prefix("customfield_") {
                    format!("CF{n}")
                } else {
                    id.clone()
                }
            }
        }
    }

    pub fn parse_id(s: &str) -> Option<Self> {
        let t = s.trim();
        if t.to_ascii_lowercase().starts_with("customfield_") {
            return Some(Self::Custom(t.to_string()));
        }
        match t.to_lowercase().as_str() {
            "site" => Some(Self::Site),
            "key" => Some(Self::Key),
            "type" | "issuetype" => Some(Self::Type),
            "status" => Some(Self::Status),
            "priority" => Some(Self::Priority),
            "age" | "ageing" => Some(Self::Age),
            "due" | "duedate" => Some(Self::Due),
            "assignee" => Some(Self::Assignee),
            "reporter" => Some(Self::Reporter),
            "parent" | "epic" => Some(Self::Parent),
            "labels" | "label" => Some(Self::Labels),
            "sprint" => Some(Self::Sprint),
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

    pub fn cell(&self, ticket: &Ticket, theme: &Theme) -> Cell<'static> {
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
            Self::Parent => {
                let text = ticket.parent_key.as_deref().unwrap_or("-");
                Cell::from(Span::raw(text.to_string()))
            }
            Self::Labels => {
                let text = if ticket.labels.is_empty() {
                    "-".to_string()
                } else {
                    ticket.labels.join(", ")
                };
                Cell::from(Span::raw(text))
            }
            Self::Sprint => {
                let text = ticket.sprint_name.as_deref().unwrap_or("-");
                Cell::from(Span::raw(text.to_string()))
            }
            Self::Summary => Cell::from(Span::raw(ticket.summary.clone())),
            Self::Custom(id) => {
                let text = ticket
                    .custom_fields
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| "-".into());
                Cell::from(Span::raw(text))
            }
        }
    }

    /// Custom field ids referenced by the active column set (for bulk fetch).
    pub fn custom_field_ids(columns: &[Self]) -> Vec<String> {
        columns
            .iter()
            .filter_map(|c| match c {
                Self::Custom(id) => Some(id.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn width_hint(&self) -> u16 {
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
            Self::Parent => 12,
            Self::Labels => 16,
            Self::Sprint => 14,
            Self::Summary => 24,
            Self::Custom(_) => 14,
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
        assert_eq!(
            Column::parse_id("customfield_10042"),
            Some(Column::Custom("customfield_10042".into()))
        );
        assert_eq!(Column::parse_id("nope"), None);
    }

    #[test]
    fn custom_field_ids_from_columns() {
        let cols = Column::resolve(Some(&["key".into(), "customfield_10001".into()]));
        assert_eq!(
            Column::custom_field_ids(&cols),
            vec!["customfield_10001".to_string()]
        );
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
