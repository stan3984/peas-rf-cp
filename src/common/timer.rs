
use std::time::{Instant, Duration};

pub struct Timer {
    start: Instant,
    duration: Duration,
    enabled: bool,
}

impl Timer {
    /// a timer that will return true after a while
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
    /// checks if a timer as run out. Has the precision of whole seconds
    /// `margin` is a percentage to set safety margins
    /// always returns false if disabled
    pub fn expired(&self, margin: f64) -> bool {
        self.enabled && Instant::now().duration_since(self.start).as_secs() >= (self.duration.as_secs() as f64 * margin) as u64
    }
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    pub fn is_disabled(&self) -> bool {
        !self.enabled
    }
    /// enables and restarts the timer
    pub fn reset(&mut self) {
        self.enabled = true;
        self.start = Instant::now();
    }
}
