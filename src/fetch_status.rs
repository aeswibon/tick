/// Per-site fetch warnings (partial success) vs a single action error (transition, comment, etc.).
#[derive(Debug, Clone, Default)]
pub struct FetchStatus {
    pub site_warnings: Vec<String>,
    pub action_error: Option<String>,
}

impl FetchStatus {
    pub fn clear_action_error(&mut self) {
        self.action_error = None;
    }

    pub fn set_action_error(&mut self, message: impl Into<String>) {
        self.action_error = Some(message.into());
    }

    pub fn set_site_warnings(&mut self, warnings: Vec<String>) {
        self.site_warnings = warnings;
    }

    pub fn has_warnings(&self) -> bool {
        !self.site_warnings.is_empty()
    }

    /// Truncated site-warning text (used in tests; UI uses the error overlay).
    #[allow(dead_code)]
    pub fn format_warnings(&self, max_len: usize) -> String {
        if self.site_warnings.is_empty() {
            return String::new();
        }
        let body = if self.site_warnings.len() == 1 {
            self.site_warnings[0].clone()
        } else {
            self.site_warnings.join(" · ")
        };
        let prefix = format!(
            " {} site{} failed:",
            self.site_warnings.len(),
            if self.site_warnings.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        truncate(&format!("{prefix} {body}"), max_len)
    }
}

#[allow(dead_code)]
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_len.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_single_warning() {
        let mut s = FetchStatus::default();
        s.set_site_warnings(vec!["acme: HTTP 401".into()]);
        assert!(s.format_warnings(200).contains("acme"));
    }

    #[test]
    fn truncates_long_warning_line() {
        let mut s = FetchStatus::default();
        s.set_site_warnings(vec!["a: x".repeat(80)]);
        assert!(s.format_warnings(40).ends_with('…'));
    }
}
