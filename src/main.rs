use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

fn main() {
    println!("Hello, world!");
}

#[derive(Debug)]
struct RateLimiter {
    inner_state: InnerState,
    base_state: State,
}

impl RateLimiter {
    pub fn new(base_state: State) -> Self {
        Self {
            inner_state: HashMap::new(),
            base_state,
        }
    }

    pub fn insert_key(&mut self, key: &str, state: State) {
        self.inner_state.insert(key.to_string(), state);
    }

    pub fn acquire(&mut self) -> bool {
        self.base_state.acquire()
    }

    pub fn acquire_by_key(&mut self, key: &str) -> bool {
        self.inner_state
            .get_mut(key)
            .map_or(false, |state| state.acquire())
    }
}

type InnerState = HashMap<String, State>;

#[derive(Debug)]
struct State {
    last_update_time: u128,
    acquired: u64,
    duration: u128,
    allowed: u64,
}

impl State {
    pub fn new(duration: u128, allowed: u64) -> Self {
        Self {
            last_update_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            acquired: 0,
            duration,
            allowed,
        }
    }

    pub fn per_second(max_burst: u64) -> Self {
        Self {
            duration: 1000,
            last_update_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            acquired: 0,
            allowed: max_burst,
        }
    }

    pub fn acquire(&mut self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        if now - self.last_update_time >= self.duration {
            self.last_update_time = now;
            self.acquired = 0;
        }
        if self.acquired < self.allowed {
            self.acquired += 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_acquire_by_key() {
        let base = State::per_second(1);
        let mut limiter = RateLimiter::new(base);

        // 为特定 key 配置限流
        limiter.insert_key("vip_user", State::per_second(5));

        // 配置过的 key 可以正常使用
        assert!(limiter.acquire_by_key("vip_user"));
        assert!(limiter.acquire_by_key("vip_user"));

        // 未配置的 key 直接拒绝
        assert!(!limiter.acquire_by_key("unknown_user"));
    }

    #[test]
    fn test_rate_limiter_independent_limits() {
        let base = State::per_second(1);
        let mut limiter = RateLimiter::new(base);

        limiter.insert_key("user1", State::per_second(2));
        limiter.insert_key("user2", State::per_second(3));

        // user1 和 user2 的限流是独立的
        assert!(limiter.acquire_by_key("user1"));
        assert!(limiter.acquire_by_key("user1"));
        assert!(!limiter.acquire_by_key("user1")); // user1 用完配额

        // user2 不受影响
        assert!(limiter.acquire_by_key("user2"));
        assert!(limiter.acquire_by_key("user2"));
        assert!(limiter.acquire_by_key("user2"));
        assert!(!limiter.acquire_by_key("user2")); // user2 用完配额
    }

    #[test]
    fn test_rate_limiter_base_state() {
        let base = State::per_second(1);
        let mut limiter = RateLimiter::new(base);

        assert!(limiter.acquire());
        assert!(!limiter.acquire());
    }

    #[test]
    fn test_state() {
        let mut state = State::per_second(1);
        assert!(state.acquire());
        assert!(!state.acquire());
    }

    #[test]
    fn test_state_reset_after_duration() {
        let mut state = State::new(100, 2); // 100ms 内允许2次
        assert!(state.acquire());
        assert!(state.acquire());
        assert!(!state.acquire()); // 第3次应该失败

        std::thread::sleep(std::time::Duration::from_millis(101));
        assert!(state.acquire()); // 窗口重置后应该成功
    }

    #[test]
    fn test_state_zero_allowed() {
        let mut state = State::per_second(0);
        assert!(!state.acquire()); // 应该一直失败
    }
}
