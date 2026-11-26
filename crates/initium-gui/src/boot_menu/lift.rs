use std::time::{Duration, Instant};

const TRAVEL: f64 = -560.0;
const DURATION: Duration = Duration::from_millis(250);

#[derive(Default)]
pub(super) struct Lift {
    transition: Option<(Instant, f64)>,
    target: f64,
}

impl Lift {
    pub fn set_open(&mut self, open: bool, now: Instant) {
        self.transition = Some((now, self.position(now)));
        self.target = if open { TRAVEL } else { 0.0 };
    }

    pub fn offset(&self, now: Instant) -> i32 {
        self.position(now).round() as i32
    }

    pub fn is_settled(&self, now: Instant) -> bool {
        self.transition
            .is_none_or(|(start, _)| now.saturating_duration_since(start) >= DURATION)
    }

    fn position(&self, now: Instant) -> f64 {
        let Some((start, from)) = self.transition else {
            return self.target;
        };
        let progress =
            (now.saturating_duration_since(start).as_secs_f64() / DURATION.as_secs_f64()).min(1.0);
        self.target + (from - self.target) * (1.0 - progress).powi(3)
    }
}

#[cfg(test)]
#[path = "lift.tests.rs"]
mod tests;
