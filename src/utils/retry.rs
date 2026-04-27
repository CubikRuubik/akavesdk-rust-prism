use std::future::Future;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::sleep;

#[derive(Debug)]
pub enum RetryError<E> {
    Aborted,
    Failed(E),
}

impl<E: std::fmt::Display> std::fmt::Display for RetryError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aborted => write!(f, "retry aborted"),
            Self::Failed(e) => write!(f, "{e}"),
        }
    }
}

/// Retries an async operation up to `max_attempts` additional times after the
/// initial call (total calls = max_attempts + 1).
///
/// The closure returns `Ok(T)` on success or `Err((should_retry, E))` on
/// failure. When `should_retry` is false the error is returned immediately
/// without retrying.
///
/// Pass a `watch::Receiver<bool>` as the cancellation signal — send `true` to
/// abort the retry loop early, which yields `Err(RetryError::Aborted)`.
/// Use `tokio::sync::watch::channel(false)` and keep the sender alive for
/// the lifetime of the call.
pub struct WithRetry {
    pub max_attempts: usize,
    pub base_delay: Duration,
}

impl WithRetry {
    pub async fn do_retry<F, Fut, T, E>(
        &self,
        mut cancel: watch::Receiver<bool>,
        mut f: F,
    ) -> Result<T, RetryError<E>>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, (bool, E)>>,
    {
        let mut attempt = 0usize;
        loop {
            if *cancel.borrow() {
                return Err(RetryError::Aborted);
            }
            match f().await {
                Ok(val) => return Ok(val),
                Err((should_retry, err)) => {
                    if !should_retry || attempt >= self.max_attempts {
                        return Err(RetryError::Failed(err));
                    }
                    tokio::select! {
                        _ = sleep(self.base_delay) => {}
                        _ = cancel.changed() => {}
                    }
                    if *cancel.borrow() {
                        return Err(RetryError::Aborted);
                    }
                    attempt += 1;
                }
            }
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn make_cancel() -> (watch::Sender<bool>, watch::Receiver<bool>) {
        watch::channel(false)
    }

    #[tokio::test]
    async fn test_with_retry_success_on_first_attempt() {
        let retry = WithRetry { max_attempts: 3, base_delay: Duration::from_millis(1) };
        let calls = Arc::new(Mutex::new(0usize));
        let c = calls.clone();
        let (_tx, rx) = make_cancel();
        let result: Result<i32, RetryError<String>> = retry
            .do_retry(rx, || {
                let c = c.clone();
                async move {
                    *c.lock().unwrap() += 1;
                    Ok(42)
                }
            })
            .await;
        assert!(result.is_ok());
        assert_eq!(*calls.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_with_retry_failure_without_retry() {
        let retry = WithRetry { max_attempts: 3, base_delay: Duration::from_millis(1) };
        let calls = Arc::new(Mutex::new(0usize));
        let c = calls.clone();
        let (_tx, rx) = make_cancel();
        let result: Result<(), RetryError<String>> = retry
            .do_retry(rx, || {
                let c = c.clone();
                async move {
                    *c.lock().unwrap() += 1;
                    Err((false, "permanent failure".to_string()))
                }
            })
            .await;
        assert!(matches!(result, Err(RetryError::Failed(_))));
        assert_eq!(*calls.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_with_retry_retry_and_success() {
        let retry = WithRetry { max_attempts: 3, base_delay: Duration::from_millis(1) };
        let calls = Arc::new(Mutex::new(0usize));
        let c = calls.clone();
        let (_tx, rx) = make_cancel();
        let result: Result<i32, RetryError<String>> = retry
            .do_retry(rx, || {
                let c = c.clone();
                async move {
                    let n = {
                        let mut lock = c.lock().unwrap();
                        *lock += 1;
                        *lock
                    };
                    if n < 3 {
                        Err((true, "transient".to_string()))
                    } else {
                        Ok(n as i32)
                    }
                }
            })
            .await;
        assert!(result.is_ok());
        assert_eq!(*calls.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn test_with_retry_exceeds_max_attempts() {
        // MaxAttempts=2 → 1 initial + 2 retries = 3 total calls
        let retry = WithRetry { max_attempts: 2, base_delay: Duration::from_millis(1) };
        let calls = Arc::new(Mutex::new(0usize));
        let c = calls.clone();
        let (_tx, rx) = make_cancel();
        let result: Result<(), RetryError<String>> = retry
            .do_retry(rx, || {
                let c = c.clone();
                async move {
                    *c.lock().unwrap() += 1;
                    Err((true, "always fails".to_string()))
                }
            })
            .await;
        assert!(matches!(result, Err(RetryError::Failed(_))));
        assert_eq!(*calls.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn test_with_retry_context_cancellation() {
        let retry = WithRetry {
            max_attempts: 5,
            base_delay: Duration::from_millis(100),
        };
        let (cancel_tx, cancel_rx) = make_cancel();
        tokio::spawn(async move {
            sleep(Duration::from_millis(300)).await;
            let _ = cancel_tx.send(true);
        });
        let result: Result<(), RetryError<String>> = retry
            .do_retry(cancel_rx, || async { Err((true, "always fails".to_string())) })
            .await;
        let err_str = match &result {
            Err(e) => e.to_string(),
            Ok(_) => panic!("expected error"),
        };
        assert!(
            err_str.contains("retry aborted"),
            "expected 'retry aborted' in error, got: {err_str}"
        );
    }

    #[tokio::test]
    async fn test_with_retry_max_attempts_zero() {
        // MaxAttempts=0 → no retries, single call then fail
        let retry = WithRetry { max_attempts: 0, base_delay: Duration::from_millis(1) };
        let calls = Arc::new(Mutex::new(0usize));
        let c = calls.clone();
        let (_tx, rx) = make_cancel();
        let result: Result<(), RetryError<String>> = retry
            .do_retry(rx, || {
                let c = c.clone();
                async move {
                    *c.lock().unwrap() += 1;
                    Err((true, "fails".to_string()))
                }
            })
            .await;
        assert!(matches!(result, Err(RetryError::Failed(_))));
        assert_eq!(*calls.lock().unwrap(), 1);
    }
}
