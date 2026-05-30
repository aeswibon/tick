//! Shared per-issue batch loops for TUI bulk and future CLI.

use std::borrow::Borrow;

#[derive(Debug, Default)]
pub struct BatchOutcome {
    pub ok: usize,
    pub failures: Vec<String>,
}

pub fn format_batch_notice(label: &str, outcome: &BatchOutcome) -> String {
    if outcome.failures.is_empty() {
        return format!("{label}: {} ok", outcome.ok);
    }
    let fail_summary = if outcome.failures.len() <= 2 {
        outcome.failures.join("; ")
    } else {
        format!("{}; …", outcome.failures[..2].join("; "))
    };
    format!(
        "{label}: {} ok, {} failed ({fail_summary})",
        outcome.ok,
        outcome.failures.len()
    )
}

pub async fn run_batch<I, F, Fut>(items: I, mut op: F) -> BatchOutcome
where
    I: IntoIterator,
    I::Item: Borrow<str>,
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let mut outcome = BatchOutcome::default();
    for item in items {
        let key = item.borrow().to_string();
        match op(key.clone()).await {
            Ok(()) => outcome.ok += 1,
            Err(e) => outcome.failures.push(format!("{key}: {e}")),
        }
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_batch_counts_ok_and_failures() {
        let keys = ["A-1", "A-2", "A-3"];
        let outcome = run_batch(keys, |k| async move {
            if k == "A-2" {
                Err("nope".into())
            } else {
                Ok(())
            }
        })
        .await;
        assert_eq!(outcome.ok, 2);
        assert_eq!(outcome.failures.len(), 1);
        assert!(outcome.failures[0].contains("A-2"));
    }

    #[test]
    fn format_notice_truncates_many_failures() {
        let outcome = BatchOutcome {
            ok: 1,
            failures: vec!["a".into(), "b".into(), "c".into()],
        };
        let s = format_batch_notice("Bulk test", &outcome);
        assert!(s.contains("3 failed"));
        assert!(s.contains('…'));
    }
}
