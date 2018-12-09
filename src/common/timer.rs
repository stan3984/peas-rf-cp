
use std::time::{Instant, Duration};

pub struct Timer {
    start: Instant,
    duration: Duration,
    enabled: bool,
}

impl Timer {
    /// starts a timer that will return true after a while
    pub fn new(dur: Duration) -> Self {
        Timer {
            start: Instant::now(),
            duration: dur,
            enabled: true,
        }
    }
    pub fn from_millis(millis: u64) -> Self {
        Timer::new(Duration::from_millis(millis))
    }
    pub fn new_expired() -> Self {
        Timer::from_millis(0)
    }
    pub fn get_timeout(&self) -> Duration {
        self.duration
    }
    /// checks if a timer as run out.
    /// `margin` is a percentage to set safety margins
    /// always returns false if disabled
    /// // TODO: margin not implemented! Duration::mul_f64 is nightly
    pub fn expired(&self, margin: f64) -> bool {
        assert!(margin > 0.0 && margin <= 1.0);
        self.enabled && Instant::now().duration_since(self.start).checked_sub(self.duration).is_some()
    }
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    pub fn is_disabled(&self) -> bool {
        !self.enabled
    }
    /// enables and restarts the timer with the same timeout
    /// it had previously
    pub fn reset(&mut self) {
        self.enabled = true;
        self.start = Instant::now();
    }
    /// same as `reset` but also changes the timer period (correct word?)
    pub fn reset_with(&mut self, dur: Duration) {
        self.duration = dur;
        self.reset();
    }
}
