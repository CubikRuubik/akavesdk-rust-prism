use std::time::Duration;

/// Configuration for retry behaviour with exponential back-off.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (not counting the initial attempt).
    pub max_retries: u32,
    /// Delay before the first retry.
    pub initial_delay: Duration,
    /// Upper bound on the computed delay.
    pub max_delay: Duration,
    /// Multiplicative factor applied to the delay after each failure.
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

/// Execute a fallible async operation with retry logic and exponential back-off.
///
/// The closure `f` is called on each attempt.  If it returns `Ok`, the result
/// is returned immediately.  On `Err`, the error is logged and – unless the
/// maximum number of retries has been reached – the executor sleeps for the
/// current back-off delay before trying again.  The last error is returned
/// when all attempts are exhausted.
#[cfg(not(target_arch = "wasm32"))]
pub async fn with_retry<F, Fut, T, E>(config: &RetryConfig, mut f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut delay = config.initial_delay;
    for attempt in 0..=config.max_retries {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                if attempt == config.max_retries {
                    return Err(e);
                }
                log::debug!(
                    "Attempt {} failed ({:?}), retrying in {:?}…",
                    attempt + 1,
                    e,
                    delay
                );
                tokio::time::sleep(delay).await;
                let next_ms = (delay.as_millis() as f64 * config.backoff_multiplier) as u64;
                delay = Duration::from_millis(next_ms).min(config.max_delay);
            }
        }
    }
    unreachable!()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_succeeds_on_first_try() {
        let config = RetryConfig {
            max_retries: 3,
            ..Default::default()
        };
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();
        let result: Result<u32, &str> = with_retry(&config, || {
            let c = calls_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(42)
            }
        })
        .await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retries_on_failure() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            backoff_multiplier: 2.0,
        };
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();
        let result: Result<u32, &'static str> = with_retry(&config, || {
            let c = calls_clone.clone();
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err("transient")
                } else {
                    Ok(99)
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), 99);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_exhausts_retries() {
        let config = RetryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(5),
            backoff_multiplier: 1.5,
        };
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();
        let result: Result<u32, &'static str> = with_retry(&config, || {
            let c = calls_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err("always fails")
            }
        })
        .await;
        assert!(result.is_err());
        // initial attempt + 2 retries = 3 total calls
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }
}
