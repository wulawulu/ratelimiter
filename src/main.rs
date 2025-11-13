
use std::collections::HashMap;

use ratelimit::{Clock, MonotonicClock, Nanos, FakeRelativeClock, Reference};

fn main() {
    println!("Hello, world!");
}

#[derive(Debug)]
struct RateLimiter<C: Clock> {
    inner_state: InnerState<C>,
    base_state: State<C>,
}

impl<C: Clock> RateLimiter<C> {
    pub fn new(base_state: State<C>) -> Self {
        Self {
            inner_state: HashMap::new(),
            base_state,
        }
    }

    pub fn insert_key(&mut self, key: &str, state: State<C>) {
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

type InnerState<C> = HashMap<String, State<C>>;

#[derive(Debug)]
struct State<C: Clock> {
    last_update: C::Instant,
    acquired: u64,
    duration_nano: Nanos,
    allowed: u64,
    clock: C,
}

impl<C: Clock> State<C> {
    pub fn new(duration: Nanos, allowed: u64, clock: C) -> Self {
        Self {
            last_update: clock.now(),
            acquired: 0,
            duration_nano: duration,
            allowed,
            clock,
        }
    }

    pub fn acquire(&mut self) -> bool {
        let now = self.clock.now();
        let elapsed: Nanos = now.duration_since(self.last_update);
        if elapsed >= self.duration_nano {
            self.last_update = now;
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

impl State<MonotonicClock> {
    pub fn per_second(max_burst: u64) -> Self {
        let clock = MonotonicClock;
        Self {
            duration_nano: Nanos::new(1_000_000_000),
            last_update: clock.now(),
            acquired: 0,
            allowed: max_burst,
            clock,
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
        let mut state = State::new(Nanos::new(100_000_000), 2, MonotonicClock); // 100ms 内允许2次
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

    // 使用 FakeRelativeClock 的测试

    #[test]
    fn test_state_with_fake_clock_basic() {
        let clock = FakeRelativeClock::default();
        let mut state = State::new(Nanos::new(1_000_000_000), 2, clock.clone()); // 1秒内允许2次
        
        // 第一次和第二次应该成功
        assert!(state.acquire());
        assert!(state.acquire());
        
        // 第三次应该失败（配额用完）
        assert!(!state.acquire());
    }

    #[test]
    fn test_state_with_fake_clock_time_window_reset() {
        let clock = FakeRelativeClock::default();
        let mut state = State::new(Nanos::new(1_000_000_000), 3, clock.clone()); // 1秒内允许3次
        
        // 用完配额
        assert!(state.acquire());
        assert!(state.acquire());
        assert!(state.acquire());
        assert!(!state.acquire());
        
        // 推进时间 0.5 秒，还不够重置
        clock.advance(std::time::Duration::from_millis(500));
        assert!(!state.acquire());
        
        // 再推进 0.5 秒，总共1秒，窗口应该重置
        clock.advance(std::time::Duration::from_millis(500));
        assert!(state.acquire());
        assert!(state.acquire());
        assert!(state.acquire());
        assert!(!state.acquire());
    }

    #[test]
    fn test_state_with_fake_clock_multiple_resets() {
        let clock = FakeRelativeClock::default();
        let mut state = State::new(Nanos::new(100_000_000), 1, clock.clone()); // 100ms 内允许1次
        
        // 第一个窗口
        assert!(state.acquire());
        assert!(!state.acquire());
        
        // 推进 100ms，重置窗口
        clock.advance(std::time::Duration::from_millis(100));
        assert!(state.acquire());
        assert!(!state.acquire());
        
        // 再推进 100ms，再次重置窗口
        clock.advance(std::time::Duration::from_millis(100));
        assert!(state.acquire());
        assert!(!state.acquire());
    }

    #[test]
    fn test_state_with_fake_clock_precise_timing() {
        let clock = FakeRelativeClock::default();
        let mut state = State::new(Nanos::new(1_000_000_000), 5, clock.clone()); // 1秒内允许5次
        
        // 快速使用3次配额
        assert!(state.acquire());
        assert!(state.acquire());
        assert!(state.acquire());
        
        // 推进 0.9 秒（还不到1秒）
        clock.advance(std::time::Duration::from_millis(900));
        assert!(state.acquire());
        assert!(state.acquire());
        assert!(!state.acquire()); // 配额用完
        
        // 推进 0.2 秒（总共超过1秒），窗口重置
        clock.advance(std::time::Duration::from_millis(200));
        assert!(state.acquire());
        assert!(state.acquire());
        assert!(state.acquire());
        assert!(state.acquire());
        assert!(state.acquire());
        assert!(!state.acquire()); // 新窗口的配额也用完
    }

    #[test]
    fn test_state_with_fake_clock_burst_control() {
        let clock = FakeRelativeClock::default();
        let mut state = State::new(Nanos::new(500_000_000), 10, clock.clone()); // 500ms 内允许10次
        
        // 用完所有配额
        for _ in 0..10 {
            assert!(state.acquire());
        }
        assert!(!state.acquire());
        
        // 推进时间到窗口边界
        clock.advance(std::time::Duration::from_millis(500));
        
        // 新窗口，配额恢复
        for _ in 0..10 {
            assert!(state.acquire());
        }
        assert!(!state.acquire());
    }

    #[test]
    fn test_state_with_fake_clock_zero_allowed() {
        let clock = FakeRelativeClock::default();
        let mut state = State::new(Nanos::new(1_000_000_000), 0, clock.clone()); // 不允许任何请求
        
        assert!(!state.acquire());
        
        // 即使推进时间也不应该允许
        clock.advance(std::time::Duration::from_secs(10));
        assert!(!state.acquire());
    }
}
