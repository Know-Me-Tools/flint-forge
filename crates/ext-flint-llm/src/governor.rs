//! In-process token-bucket rate governor for the Flint Ember background worker.
//!
//! v1 enforces three hard caps per worker process:
//!   * RPM  (requests per minute)
//!   * TPM  (tokens per minute)
//!   * daily cost cap in micro-dollars
//!
//! These are process-local limits. A future release will move the budget into a
//! flint-gate quota service so limits can be shared across Postgres workers.

use std::time::{Duration, Instant};

/// Default requests-per-minute cap for embedding calls.
const DEFAULT_RPM: u64 = 3_000;
/// Default tokens-per-minute cap.
const DEFAULT_TPM: u64 = 1_000_000;
/// Default daily spend cap in micro-dollars (10 USD).
const DEFAULT_DAILY_CAP_MICRODOLLARS: u64 = 10_000_000;

/// Model prices in micro-dollars per 1M tokens.
fn price_per_1m_tokens(model: &str) -> u64 {
    // OpenAI list prices as of 2025-07; rounded up for safety.
    if model.contains("text-embedding-3-large") {
        130_000 // $0.13 / 1M tokens
    } else if model.contains("text-embedding-3-small") {
        20_000 // $0.02 / 1M tokens
    } else {
        // Conservative default: treat unknown embedding models as large.
        130_000
    }
}

/// A single leaky/token bucket with a per-second refill rate.
struct TokenBucket {
    capacity: u64,
    tokens: f64,
    last: Instant,
    refill_per_second: f64,
}

impl TokenBucket {
    fn new(capacity: u64, refill_per_minute: u64) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            last: Instant::now(),
            refill_per_second: refill_per_minute as f64 / 60.0,
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_per_second).min(self.capacity as f64);
        self.last = now;
    }

    /// Try to consume `n` tokens. On failure, returns the minimum wait time
    /// until the request could succeed (assuming no other traffic).
    fn try_acquire(&mut self, n: u64) -> Result<(), Duration> {
        self.refill();
        if n > self.capacity {
            // Request can never fit in this bucket.
            return Err(Duration::from_secs(60));
        }
        if self.tokens >= n as f64 {
            self.tokens -= n as f64;
            Ok(())
        } else {
            let deficit = n as f64 - self.tokens;
            let wait = Duration::from_secs_f64(deficit / self.refill_per_second);
            Err(wait)
        }
    }
}

/// Reason a request was rejected by the governor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitReason {
    Rpm,
    Tpm,
    Cost,
}

/// Per-process rate and budget governor.
pub struct Governor {
    rpm: TokenBucket,
    tpm: TokenBucket,
    cost: TokenBucket,
    daily_cap: u64,
    day_start: Instant,
}

impl Default for Governor {
    fn default() -> Self {
        Self::new(DEFAULT_RPM, DEFAULT_TPM, DEFAULT_DAILY_CAP_MICRODOLLARS)
    }
}

impl Governor {
    pub fn new(rpm: u64, tpm: u64, daily_cap_microdollars: u64) -> Self {
        // Cost bucket is refilled once per day; capacity is the daily cap.
        Self {
            rpm: TokenBucket::new(rpm, rpm),
            tpm: TokenBucket::new(tpm, tpm),
            cost: TokenBucket::new(daily_cap_microdollars, daily_cap_microdollars),
            daily_cap: daily_cap_microdollars,
            day_start: Instant::now(),
        }
    }

    fn reset_day_if_needed(&mut self) {
        if Instant::now().duration_since(self.day_start) >= Duration::from_secs(86_400) {
            self.day_start = Instant::now();
            self.cost = TokenBucket::new(self.daily_cap, self.daily_cap);
        }
    }

    /// Estimate token count for a piece of text.
    pub fn estimate_tokens(text: &str) -> u64 {
        // Very rough heuristic: ~4 characters per token for English text.
        text.len().div_ceil(4).max(1) as u64
    }

    /// Cost tokens in micro-dollars for the given model and token count.
    pub fn cost_tokens(token_count: u64, model: &str) -> u64 {
        let price = price_per_1m_tokens(model);
        // cost = tokens * price / 1_000_000
        (token_count as u128)
            .saturating_mul(price as u128)
            .saturating_div(1_000_000) as u64
    }

    /// Try to reserve capacity for one embedding request.
    pub fn try_acquire(&mut self, token_count: u64, model: &str) -> Result<(), LimitReason> {
        self.reset_day_if_needed();
        self.rpm.try_acquire(1).map_err(|_| LimitReason::Rpm)?;
        self.tpm
            .try_acquire(token_count)
            .map_err(|_| LimitReason::Tpm)?;
        let cost = Self::cost_tokens(token_count, model);
        if cost > 0 {
            self.cost.try_acquire(cost).map_err(|_| LimitReason::Cost)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpm_cap_blocks_after_burst() {
        let mut g = Governor::new(2, 1_000_000, 10_000_000);
        assert!(g.try_acquire(10, "text-embedding-3-small").is_ok());
        assert!(g.try_acquire(10, "text-embedding-3-small").is_ok());
        assert_eq!(
            g.try_acquire(10, "text-embedding-3-small"),
            Err(LimitReason::Rpm)
        );
    }

    #[test]
    fn cost_cap_blocks_expensive_models() {
        let mut g = Governor::new(1_000_000, 1_000_000, 1);
        assert_eq!(
            g.try_acquire(1_000, "text-embedding-3-small"),
            Err(LimitReason::Cost)
        );
    }

    #[test]
    fn token_estimate_is_positive() {
        assert!(Governor::estimate_tokens("hello world") >= 1);
        assert!(Governor::estimate_tokens("") >= 1);
    }
}
