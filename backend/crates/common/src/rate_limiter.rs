/// Token-bucket rate limiter for Gemini API (15 RPM free tier).
/// Shared via Arc across all agents.
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration, Instant};

const MAX_REQUESTS_PER_MINUTE: u32 = 12; // conservative: leave 3 RPM headroom
const WINDOW: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct RateLimiter(Arc<Mutex<Inner>>);

struct Inner {
    tokens: u32,
    window_start: Instant,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Inner {
            tokens: MAX_REQUESTS_PER_MINUTE,
            window_start: Instant::now(),
        })))
    }

    /// Waits until a token is available, then consumes one.
    pub async fn acquire(&self) {
        loop {
            {
                let mut inner = self.0.lock().await;
                let elapsed = inner.window_start.elapsed();
                if elapsed >= WINDOW {
                    inner.tokens = MAX_REQUESTS_PER_MINUTE;
                    inner.window_start = Instant::now();
                }
                if inner.tokens > 0 {
                    inner.tokens -= 1;
                    return;
                }
                // How long until window resets
                let wait = WINDOW.saturating_sub(elapsed);
                drop(inner);
                tracing::info!("Rate limit: waiting {}s for window reset", wait.as_secs());
                sleep(wait).await;
            }
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Retry a Gemini API call with exponential backoff on 429.
/// Parses the `retryDelay` from the error body when available.
pub async fn with_retry<F, Fut, T>(limiter: &RateLimiter, mut f: F) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let mut delay = Duration::from_secs(20);
    for attempt in 0..5 {
        limiter.acquire().await;
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("429") || msg.contains("RESOURCE_EXHAUSTED") {
                    // Try to parse retryDelay from error message
                    if let Some(secs) = parse_retry_delay(&msg) {
                        delay = Duration::from_secs(secs + 2);
                    }
                    tracing::warn!(
                        "Gemini 429 (attempt {}), retrying in {}s",
                        attempt + 1,
                        delay.as_secs()
                    );
                    sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(120));
                } else {
                    return Err(e);
                }
            }
        }
    }
    anyhow::bail!("Gemini API: exceeded max retries (5) due to rate limiting")
}

pub(crate) fn parse_retry_delay(msg: &str) -> Option<u64> {
    // Looks for patterns like: "retryDelay": "20s" or "retry in 20.4s"
    let patterns = [r#""retryDelay": ""#, "retry in "];
    for pat in &patterns {
        if let Some(idx) = msg.find(pat) {
            let rest = &msg[idx + pat.len()..];
            let num: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            if let Ok(f) = num.parse::<f64>() {
                return Some(f.ceil() as u64);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_retry_delay_from_json() {
        assert_eq!(parse_retry_delay(r#""retryDelay": "20s""#), Some(20));
        assert_eq!(parse_retry_delay(r#""retryDelay": "5s""#), Some(5));
    }

    #[test]
    fn parse_retry_delay_from_prose() {
        assert_eq!(parse_retry_delay("retry in 30.4s please"), Some(31));
        assert_eq!(parse_retry_delay("retry in 10s"), Some(10));
    }

    #[test]
    fn parse_retry_delay_missing() {
        assert_eq!(parse_retry_delay("some unrelated error"), None);
        assert_eq!(parse_retry_delay(""), None);
    }

    #[tokio::test]
    async fn rate_limiter_grants_tokens() {
        let limiter = RateLimiter::new();
        // Should succeed immediately (12 tokens available)
        tokio::time::timeout(std::time::Duration::from_millis(100), limiter.acquire())
            .await
            .expect("acquire should not block when tokens available");
    }
}
