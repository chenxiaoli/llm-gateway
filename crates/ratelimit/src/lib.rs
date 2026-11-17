use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// A counter for one window.
///
/// The window starts when the counter is created (or when the previous
/// window expired) and is reset once it is over — without that, a key that
/// reached its limit stayed blocked until the process restarted.
#[derive(Debug)]
struct Counter {
    count: i64,
    window_start: Instant,
}

pub struct RateLimiter {
    counters: Arc<RwLock<HashMap<String, Counter>>>,
    window_size_secs: i64,
}

impl RateLimiter {
    pub fn new(window_size_secs: i64) -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            window_size_secs,
        }
    }

    /// Check if a request is allowed. Returns true if under limit.
    pub async fn check_and_increment(
        &self,
        key_id: &str,
        model: &str,
        rpm_limit: Option<i64>,
        tpm_limit: Option<i64>,
        input_tokens: Option<i64>,
    ) -> bool {
        if rpm_limit.is_none() && tpm_limit.is_none() {
            return true;
        }

        let rpm_key = format!("rpm:{}:{}", key_id, model);
        let tpm_key = format!("tpm:{}:{}", key_id, model);

        let window = Duration::from_secs(self.window_size_secs.max(1) as u64);
        let mut counters = self.counters.write().await;
        let now = Instant::now();

        // Check RPM
        if let Some(rpm) = rpm_limit {
            let counter = counters
                .entry(rpm_key.clone())
                .or_insert_with(|| Counter {
                    count: 0,
                    window_start: now,
                });
            if now.duration_since(counter.window_start) >= window {
                counter.count = 0;
                counter.window_start = now;
            }
            if counter.count >= rpm {
                return false;
            }
        }

        // Check TPM
        if let Some(tpm) = tpm_limit {
            let counter = counters
                .entry(tpm_key.clone())
                .or_insert_with(|| Counter {
                    count: 0,
                    window_start: now,
                });
            if now.duration_since(counter.window_start) >= window {
                counter.count = 0;
                counter.window_start = now;
            }
            if counter.count + input_tokens.unwrap_or(0) > tpm {
                return false;
            }
        }

        if rpm_limit.is_some() {
            if let Some(counter) = counters.get_mut(&rpm_key) {
                counter.count += 1;
            }
        }
        if tpm_limit.is_some() {
            if let Some(counter) = counters.get_mut(&tpm_key) {
                counter.count += input_tokens.unwrap_or(0);
            }
        }

        true
    }

    pub fn window_size_secs(&self) -> i64 {
        self.window_size_secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_unlimited() {
        let limiter = RateLimiter::new(60);
        assert!(limiter.check_and_increment("key1", "model1", None, None, None).await);
    }

    #[tokio::test]
    async fn test_rpm_limit() {
        let limiter = RateLimiter::new(60);
        for _ in 0..5 {
            assert!(limiter.check_and_increment("key1", "model1", Some(5), None, None).await);
        }
        assert!(!limiter.check_and_increment("key1", "model1", Some(5), None, None).await);
    }

    #[tokio::test]
    async fn test_tpm_limit() {
        let limiter = RateLimiter::new(60);
        assert!(limiter.check_and_increment("key1", "model1", None, Some(1000), Some(500)).await);
        assert!(!limiter.check_and_increment("key1", "model1", None, Some(1000), Some(600)).await);
    }

    #[tokio::test]
    async fn test_independent_keys() {
        let limiter = RateLimiter::new(60);
        for _ in 0..5 {
            assert!(limiter.check_and_increment("key1", "model1", Some(5), None, None).await);
        }
        assert!(limiter.check_and_increment("key2", "model1", Some(5), None, None).await);
    }

    #[tokio::test]
    async fn the_window_expires_and_the_key_is_allowed_again() {
        // The counters used to never reset, so a key that reached its limit
        // stayed blocked until the process restarted.
        let limiter = RateLimiter::new(1);
        assert!(limiter.check_and_increment("key1", "model1", Some(2), None, None).await);
        assert!(limiter.check_and_increment("key1", "model1", Some(2), None, None).await);
        assert!(!limiter.check_and_increment("key1", "model1", Some(2), None, None).await);

        tokio::time::sleep(Duration::from_millis(1_100)).await;

        assert!(
            limiter.check_and_increment("key1", "model1", Some(2), None, None).await,
            "the limit must reset when the window is over"
        );
    }
}
