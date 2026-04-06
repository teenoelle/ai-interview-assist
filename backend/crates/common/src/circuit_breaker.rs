use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Simple circuit breaker for external service calls.
///
/// - **Closed** (normal): failure count < threshold → provider is tried.
/// - **Open**: failure count ≥ threshold AND reset period not elapsed → provider is skipped.
/// - **Half-open**: reset period elapsed → one attempt allowed; success closes it,
///   failure re-opens it for another full reset period.
pub struct CircuitBreaker {
    failures: AtomicU32,
    opened_at: Mutex<Option<Instant>>,
    threshold: u32,
    reset: Duration,
    name: &'static str,
}

impl CircuitBreaker {
    pub fn new(name: &'static str, threshold: u32, reset_secs: u64) -> Self {
        Self {
            failures: AtomicU32::new(0),
            opened_at: Mutex::new(None),
            threshold,
            reset: Duration::from_secs(reset_secs),
            name,
        }
    }

    /// Returns true if the circuit is open and the provider should be skipped.
    pub fn is_open(&self) -> bool {
        if self.failures.load(Ordering::Relaxed) < self.threshold {
            return false;
        }
        let guard = self.opened_at.lock().unwrap();
        match *guard {
            Some(t) => t.elapsed() < self.reset,
            None => false,
        }
    }

    /// Call after a successful provider response — resets the breaker to closed.
    pub fn record_success(&self) {
        let prev = self.failures.swap(0, Ordering::Relaxed);
        if prev >= self.threshold {
            tracing::info!("Circuit breaker '{}' closed after successful response", self.name);
        }
        *self.opened_at.lock().unwrap() = None;
    }

    /// Call after a failed provider response.
    /// Opens the breaker once the failure count hits the threshold.
    pub fn record_failure(&self) {
        let prev = self.failures.fetch_add(1, Ordering::Relaxed);
        let now_count = prev + 1;
        if now_count >= self.threshold {
            let mut guard = self.opened_at.lock().unwrap();
            let reset_secs = self.reset.as_secs();
            if guard.is_none() {
                tracing::warn!(
                    "Circuit breaker '{}' opened after {} failures — skipping for {}s",
                    self.name, now_count, reset_secs
                );
            } else {
                // Re-opened from half-open state
                tracing::warn!(
                    "Circuit breaker '{}' re-opened (half-open attempt failed) — skipping for {}s",
                    self.name, reset_secs
                );
            }
            *guard = Some(Instant::now());
        }
    }

    pub fn failure_count(&self) -> u32 {
        self.failures.load(Ordering::Relaxed)
    }
}
