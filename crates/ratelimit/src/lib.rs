use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct RateLimiter {
    counters: Arc<RwLock<HashMap<String, i64>>>,
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

        let mut counters = self.counters.write().await;

        // Check RPM
        if let Some(rpm) = rpm_limit {
            let rpm_count = counters.entry(rpm_key.clone()).or_insert(0);
            if *rpm_count >= rpm {
                return false;
            }
        }

        // Check TPM
        if let Some(tpm) = tpm_limit {
            let tpm_count = counters.entry(tpm_key.clone()).or_insert(0);
            if *tpm_count + input_tokens.unwrap_or(0) > tpm {
                return false;
            }
        }

        *counters.entry(rpm_key).or_insert(0) += 1;
        *counters.entry(tpm_key).or_insert(0) += input_tokens.unwrap_or(0);

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
        assert!(limiter.check_and_increment("key1", "model1", None, Some(100), Some(50)).await);
        assert!(limiter.check_and_increment("key1", "model1", None, Some(100), Some(50)).await);
        assert!(!limiter.check_and_increment("key1", "model1", None, Some(100), Some(10)).await);
    }

    #[tokio::test]
    async fn test_independent_keys() {
        let limiter = RateLimiter::new(60);
        assert!(limiter.check_and_increment("key1", "model1", Some(1), None, None).await);
        assert!(!limiter.check_and_increment("key1", "model1", Some(1), None, None).await);
        assert!(limiter.check_and_increment("key2", "model1", Some(1), None, None).await);
    }
}
