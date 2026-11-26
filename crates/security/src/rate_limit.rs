//! Bộ giới hạn tần suất (In-memory Sliding Window Rate Limiter) không khóa toàn cục dùng `DashMap`.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use dashmap::DashMap;

/// Bộ đếm giới hạn số request theo từng khóa (IP hoặc `user_id`) trong một cửa sổ thời gian.
#[derive(Debug)]
pub struct RateLimiter {
    max_requests: usize,
    window: Duration,
    buckets: DashMap<String, VecDeque<Instant>>,
}

impl RateLimiter {
    /// Tạo `RateLimiter` cho phép tối đa `max_requests` trong mỗi khoảng `window`.
    #[must_use]
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            buckets: DashMap::new(),
        }
    }

    /// Kiểm tra và ghi nhận một request mới. Nếu vượt ngưỡng trả về `Err(retry_after_secs)`.
    pub fn check(&self, key: &str) -> Result<(), u64> {
        let now = Instant::now();
        let mut entry = self.buckets.entry(key.to_owned()).or_default();
        let queue = entry.value_mut();

        while let Some(&oldest) = queue.front() {
            if now.duration_since(oldest) >= self.window {
                queue.pop_front();
            } else {
                break;
            }
        }

        if queue.len() >= self.max_requests {
            let retry_after = queue.front().map_or(1, |&oldest| {
                self.window
                    .saturating_sub(now.duration_since(oldest))
                    .as_secs()
                    .max(1)
            });
            return Err(retry_after);
        }

        queue.push_back(now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_max_requests_within_window() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        assert!(limiter.check("1.2.3.4").is_ok());
        assert!(limiter.check("1.2.3.4").is_ok());
        assert!(limiter.check("1.2.3.4").is_err());
        // Khóa khác vẫn được phép
        assert!(limiter.check("5.6.7.8").is_ok());
    }
}
