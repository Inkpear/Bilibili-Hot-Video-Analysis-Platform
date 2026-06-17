use std::time::Duration;

use rand::Rng;
use tokio::time::Instant;
use tracing::debug;

/// Per-worker rate limiter with independent delay and cooldown.
///
/// Each Worker owns one instance — there is no shared global state.
pub struct PerWorkerRateLimiter {
    min_delay: Duration,
    max_delay: Duration,
    cooldown_until: Option<Instant>,
}

impl PerWorkerRateLimiter {
    pub fn new(min_ms: u64, max_ms: u64) -> Self {
        Self {
            min_delay: Duration::from_millis(min_ms),
            max_delay: Duration::from_millis(max_ms),
            cooldown_until: None,
        }
    }

    /// Sleep for a random duration in [min_delay, max_delay].
    /// If currently in cooldown, waits for the cooldown to expire first.
    pub async fn delay(&self) {
        if let Some(until) = self.cooldown_until {
            let now = Instant::now();
            if now < until {
                let wait = until - now;
                debug!(cooldown_secs = wait.as_secs_f32(), "Waiting for cooldown");
                tokio::time::sleep(wait).await;
            }
        }

        let delay_ms = rand::thread_rng()
            .gen_range(self.min_delay.as_millis() as u64..=self.max_delay.as_millis() as u64);
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }

    /// Set a cooldown period (e.g. after receiving a 429).
    pub fn set_cooldown(&mut self, seconds: u64) {
        self.cooldown_until = Some(Instant::now() + Duration::from_secs(seconds));
        debug!(seconds, "Cooldown set");
    }

    /// Check if currently in cooldown.
    pub fn is_in_cooldown(&self) -> bool {
        self.cooldown_until
            .map(|until| Instant::now() < until)
            .unwrap_or(false)
    }

    /// Clear the cooldown (e.g. after a successful request).
    pub fn reset_cooldown(&mut self) {
        self.cooldown_until = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let limiter = PerWorkerRateLimiter::new(500, 2000);
        assert_eq!(limiter.min_delay, Duration::from_millis(500));
        assert_eq!(limiter.max_delay, Duration::from_millis(2000));
        assert!(!limiter.is_in_cooldown());
    }

    #[test]
    fn test_cooldown() {
        let mut limiter = PerWorkerRateLimiter::new(500, 2000);
        assert!(!limiter.is_in_cooldown());

        limiter.set_cooldown(60);
        assert!(limiter.is_in_cooldown());

        limiter.reset_cooldown();
        assert!(!limiter.is_in_cooldown());
    }
}
