use std::sync::{Arc, LazyLock};
use std::time::Duration;

use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
use tokio::time::Instant;

pub fn shared() -> RateLimiter {
    static SHARED: LazyLock<RateLimiter> =
        LazyLock::new(|| RateLimiter::new(3, Duration::from_secs(1)));
    SHARED.clone()
}

#[derive(Clone)]
pub struct RateLimiter {
    interval: Duration,
    in_flight: Arc<Semaphore>,
    last_dispatch: Arc<Mutex<Option<Instant>>>,
    resume_at: Arc<Mutex<Option<Instant>>>,
}

impl RateLimiter {
    pub fn new(requests: u32, per: Duration) -> Self {
        Self {
            interval: per / requests.max(1),
            in_flight: Arc::new(Semaphore::new(requests.max(1) as usize)),
            last_dispatch: Arc::new(Mutex::new(None)),
            resume_at: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn acquire(&self) -> OwnedSemaphorePermit {
        let Ok(permit) = Arc::clone(&self.in_flight).acquire_owned().await else {
            unreachable!()
        };
        let mut last = self.last_dispatch.lock().await;

        let resume_at = *self.resume_at.lock().await;
        if let Some(resume) = resume_at {
            tokio::time::sleep_until(resume).await;
        }

        if let Some(previous) = *last
            && let Some(remaining) = self.interval.checked_sub(previous.elapsed())
        {
            tokio::time::sleep(remaining).await;
        }

        *last = Some(Instant::now());
        permit
    }

    pub async fn back_off(&self, after: Duration) {
        *self.resume_at.lock().await = Some(Instant::now() + after);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test(start_paused = true)]
    async fn configured_interval_pacing() {
        let limiter = RateLimiter::new(3, Duration::from_secs(1));
        let start = tokio::time::Instant::now();
        for _ in 0..4 {
            drop(limiter.acquire().await);
        }
        assert!(start.elapsed() >= Duration::from_millis(333));
    }

    #[tokio::test(start_paused = true)]
    async fn back_off_hold() {
        let limiter = RateLimiter::new(3, Duration::from_secs(1));
        limiter.back_off(Duration::from_secs(45)).await;
        let start = tokio::time::Instant::now();
        drop(limiter.acquire().await);
        assert!(start.elapsed() >= Duration::from_secs(45));
        let resumed = tokio::time::Instant::now();
        drop(limiter.acquire().await);
        assert!(resumed.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test(start_paused = true)]
    async fn caps_requests_in_flight() {
        let limiter = RateLimiter::new(3, Duration::from_secs(1));
        let held: Vec<_> = [
            limiter.acquire().await,
            limiter.acquire().await,
            limiter.acquire().await,
        ]
        .into();
        assert_eq!(limiter.in_flight.available_permits(), 0);
        let fourth = tokio::spawn({
            let limiter = limiter.clone();
            async move { limiter.acquire().await }
        });
        tokio::time::sleep(Duration::from_secs(5)).await;
        assert!(!fourth.is_finished());
        drop(held);
        let _permit = fourth.await.unwrap();
        assert_eq!(limiter.in_flight.available_permits(), 2);
    }
}
