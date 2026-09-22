//! 系统时钟：让「现在」可被注入，测试里就能把时间钉住。
//!
//! 与 zed `crates/clock/src/system_clock.rs` 同构。命名上保留了两套：
//! - [`SystemClock::now`] 是**推荐**的名字（zed 的 `utc_now()` 返回 [`Instant`]
//!   单调时钟，跟 UTC/时区毫无关系，是个命名坑）；
//! - [`SystemClock::utc_now`] 只是为了**让照搬的 zed 代码零改动**而保留的别名，
//!   默认实现直接转发到 `now()`。新写的代码请用 `now()`。
//!
//! 真要 UTC 日历时间请另配 `chrono`。

use std::time::Instant;

/// A source of the current time.
///
/// 生产环境传 [`RealSystemClock`]，测试传 [`FakeSystemClock`]。
pub trait SystemClock: Send + Sync {
    /// Returns the current monotonic time.
    fn now(&self) -> Instant;

    /// zed 的旧名字，等价于 [`SystemClock::now`]。
    ///
    /// 仅为了兼容从 zed 直接搬过来的调用点（如 `client::telemetry`）。
    /// 新代码请直接用 `now()`。
    fn utc_now(&self) -> Instant {
        self.now()
    }
}

pub struct RealSystemClock;

impl SystemClock for RealSystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[cfg(any(test, feature = "test-support"))]
pub struct FakeSystemClockState {
    now: Instant,
}

/// A clock whose time only moves when you say so.
#[cfg(any(test, feature = "test-support"))]
pub struct FakeSystemClock {
    // Use an unfair lock to ensure tests are deterministic.
    state: parking_lot::Mutex<FakeSystemClockState>,
}

#[cfg(any(test, feature = "test-support"))]
impl FakeSystemClock {
    pub fn new() -> Self {
        let state = FakeSystemClockState {
            now: Instant::now(),
        };

        Self {
            state: parking_lot::Mutex::new(state),
        }
    }

    pub fn set_now(&self, now: Instant) {
        self.state.lock().now = now;
    }

    pub fn advance(&self, duration: std::time::Duration) {
        self.state.lock().now += duration;
    }
}

#[cfg(any(test, feature = "test-support"))]
impl SystemClock for FakeSystemClock {
    fn now(&self) -> Instant {
        self.state.lock().now
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_real_clock_advances() {
        let clock = RealSystemClock;
        let before = clock.now();
        // 单调时钟至少不会倒退；是否「前进」取决于平台时钟分辨率，故只断言 >=。
        assert!(clock.now() >= before);
    }

    #[test]
    fn test_fake_clock_is_frozen_until_advanced() {
        let clock = FakeSystemClock::new();
        let start = clock.now();
        assert_eq!(clock.now(), start, "不 advance 就不应该动");

        clock.advance(Duration::from_secs(30));
        assert_eq!(clock.now(), start + Duration::from_secs(30));

        clock.set_now(start);
        assert_eq!(clock.now(), start);
    }

    #[test]
    fn test_system_clock_is_object_safe() {
        let clocks: Vec<Box<dyn SystemClock>> =
            vec![Box::new(RealSystemClock), Box::new(FakeSystemClock::new())];
        assert_eq!(clocks.len(), 2);
        let _ = clocks[1].now();
    }
}
