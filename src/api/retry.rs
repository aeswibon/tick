use reqwest::{header::RETRY_AFTER, Response, StatusCode};
use std::future::Future;
use std::time::Duration;

const MAX_ATTEMPTS: u32 = 4;
const BASE_BACKOFF_MS: u64 = 400;

pub fn should_retry_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS
        || status == StatusCode::BAD_GATEWAY
        || status == StatusCode::SERVICE_UNAVAILABLE
        || status == StatusCode::GATEWAY_TIMEOUT
}

fn should_retry_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.is_request()
}

async fn sleep_backoff(attempt: u32, retry_after: Option<u64>) {
    let ms = retry_after.unwrap_or_else(|| BASE_BACKOFF_MS.saturating_mul(1 << attempt.min(4)));
    let capped = ms.min(15_000);
    tokio::time::sleep(Duration::from_millis(capped)).await;
}

fn parse_retry_after_secs(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    let value = headers.get(RETRY_AFTER)?.to_str().ok()?;
    value.parse::<u64>().ok()
}

/// Run an HTTP request with exponential backoff on rate limits and transient failures.
pub async fn with_retry<F, Fut>(mut operation: F) -> Result<Response, String>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<Response, reqwest::Error>>,
{
    let mut last_err = String::from("request failed");

    for attempt in 0..MAX_ATTEMPTS {
        match operation().await {
            Ok(resp) if should_retry_status(resp.status()) && attempt + 1 < MAX_ATTEMPTS => {
                let retry_after = parse_retry_after_secs(resp.headers());
                let _ = resp.bytes().await;
                sleep_backoff(attempt, retry_after).await;
            }
            Ok(resp) => return Ok(resp),
            Err(err) if should_retry_error(&err) && attempt + 1 < MAX_ATTEMPTS => {
                last_err = format!("HTTP error: {err}");
                sleep_backoff(attempt, None).await;
            }
            Err(err) => return Err(format!("HTTP error: {err}")),
        }
    }
    Err(last_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_rate_limit_and_server_errors() {
        assert!(should_retry_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(should_retry_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(!should_retry_status(StatusCode::BAD_REQUEST));
        assert!(!should_retry_status(StatusCode::NOT_FOUND));
    }
}
