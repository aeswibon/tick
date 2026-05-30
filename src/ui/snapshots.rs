//! UI text snapshots (insta + ratatui TestBackend).

#[cfg(test)]
mod tests {
    use crate::api::JiraClient;
    use crate::app::App;
    use crate::columns::Column;
    use crate::config::Config;
    use crate::theme::Theme;
    use crate::ui::help::draw_help;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui::Terminal;
    use std::sync::Arc;

    fn test_config() -> Config {
        Config {
            email: "a@b.com".into(),
            token: "t".into(),
            sites: vec![],
            columns: None,
            max_results: 50,
            page_size: 20,
            theme: "default".into(),
            views: Default::default(),
            notify_on_refresh: false,
            auth: Default::default(),
            oauth: Default::default(),
            create: Default::default(),
            hooks: Default::default(),
            detail: Default::default(),
            view_jql: Config::build_view_jql(&Default::default()),
        }
    }

    fn test_app() -> App {
        let jira = Arc::new(JiraClient::new("a@b.com", "t", false));
        App::new(test_config(), Theme::default(), jira, false)
    }

    fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
        let buf = terminal.backend().buffer();
        let mut lines = Vec::new();
        for y in 0..buf.area.height {
            let mut row = String::new();
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            lines.push(row.trim_end().to_string());
        }
        lines.join("\n")
    }

    #[test]
    fn help_overlay_snapshot() {
        let backend = TestBackend::new(72, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let app = test_app();
        terminal
            .draw(|f| draw_help(f, &app, Rect::new(0, 0, 72, 24)))
            .expect("draw");
        insta::assert_snapshot!(buffer_to_string(&terminal));
    }

    #[test]
    fn table_column_headers_snapshot() {
        let columns = Column::resolve(None);
        let line: String = columns
            .iter()
            .map(|c| c.header())
            .collect::<Vec<_>>()
            .join(" | ");
        insta::assert_snapshot!(line);
    }
}
