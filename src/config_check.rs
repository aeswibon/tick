use crate::columns;
use crate::config::Config;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckFinding {
    pub level: &'static str,
    pub message: String,
}

fn err(message: impl Into<String>) -> CheckFinding {
    CheckFinding {
        level: "error",
        message: message.into(),
    }
}

fn warn(message: impl Into<String>) -> CheckFinding {
    CheckFinding {
        level: "warn",
        message: message.into(),
    }
}

pub fn validate_config(config: &Config) -> Vec<CheckFinding> {
    let mut out = Vec::new();

    if config.sites.is_empty() {
        out.push(err("No [[sites]] configured"));
    }

    let mut names = std::collections::HashSet::new();
    for site in &config.sites {
        if !names.insert(&site.name) {
            out.push(err(format!("Duplicate site name '{}'", site.name)));
        }
        if site.base_url.trim().is_empty() {
            out.push(err(format!("Site '{}' has empty base_url", site.name)));
        } else if !site.base_url.contains("atlassian.net") {
            out.push(warn(format!(
                "Site '{}' base_url does not look like Jira Cloud (*.atlassian.net)",
                site.name
            )));
        }
    }

    for view in &config.views.custom {
        if view.jql.trim().is_empty() {
            out.push(err(format!("Custom view '{}' has empty jql", view.name)));
        }
        if view.name.trim().is_empty() {
            out.push(err("Custom view with empty name"));
        }
    }

    for t in &config.create.templates {
        if t.name.trim().is_empty() {
            out.push(err("Template with empty name"));
        }
        if t.project.trim().is_empty() {
            out.push(warn(format!("Template '{}' has empty project", t.name)));
        }
    }

    if let Some(cols) = &config.columns {
        let resolved = columns::Column::resolve(Some(cols.as_slice()));
        if resolved.is_empty() && !cols.is_empty() {
            out.push(warn(
                "columns config did not parse any valid column ids (check spelling)",
            ));
        }
        for id in cols {
            let id = id.trim();
            if id.is_empty() {
                out.push(err("columns contains an empty entry"));
                continue;
            }
            if let Some(suffix) = id.strip_prefix("customfield_") {
                if suffix.is_empty() || !suffix.chars().all(|c| c.is_ascii_digit()) {
                    out.push(warn(format!(
                        "Column '{id}' is not a valid customfield_<digits> id"
                    )));
                }
            }
        }
    }

    out
}

pub fn run_check(config: &Config) -> i32 {
    let findings = validate_config(config);
    if findings.is_empty() {
        println!("Config OK");
        return 0;
    }
    for f in &findings {
        eprintln!("[{}] {}", f.level, f.message);
    }
    if findings.iter().any(|f| f.level == "error") {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuthMethod, Config, CreateSettings, OAuthSettings, ViewQueries};

    fn minimal_config(sites: Vec<crate::config::Site>) -> Config {
        Config {
            email: "a@b.com".into(),
            token: "t".into(),
            sites,
            columns: None,
            max_results: 50,
            page_size: 20,
            theme: "default".into(),
            views: ViewQueries::default(),
            notify_on_refresh: false,
            auth: AuthMethod::Token,
            oauth: OAuthSettings::default(),
            create: CreateSettings::default(),
            view_jql: Config::build_view_jql(&ViewQueries::default()),
        }
    }

    #[test]
    fn empty_sites_is_error() {
        let config = minimal_config(vec![]);
        assert!(
            validate_config(&config)
                .iter()
                .any(|f| f.level == "error" && f.message.contains("sites"))
        );
    }

    #[test]
    fn duplicate_site_name_is_error() {
        let site = crate::config::Site {
            name: "a".into(),
            base_url: "https://x.atlassian.net".into(),
            ..Default::default()
        };
        let config = minimal_config(vec![site.clone(), site]);
        assert!(
            validate_config(&config)
                .iter()
                .any(|f| f.message.contains("Duplicate"))
        );
    }

    #[test]
    fn clean_minimal_config_ok() {
        let config = minimal_config(vec![crate::config::Site {
            name: "zeta".into(),
            base_url: "https://zeta.atlassian.net".into(),
            ..Default::default()
        }]);
        assert!(validate_config(&config).is_empty());
    }
}
