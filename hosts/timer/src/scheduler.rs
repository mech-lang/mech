use crate::TimerSnapshot;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SchedulerEmission {
    pub snapshot: TimerSnapshot,
}

#[derive(Clone, Debug)]
pub struct FixedStepScheduler {
    frequency_hz: u64,
    max_catch_up_steps: u64,
    delta_ms: f64,
    start_ms: Option<f64>,
    next_boundary_ms: f64,
    tick: u64,
    skipped_steps: u64,
}

impl FixedStepScheduler {
    pub fn new(frequency_hz: u64, max_catch_up_steps: u64) -> Self {
        Self {
            frequency_hz,
            max_catch_up_steps,
            delta_ms: 1000.0 / frequency_hz as f64,
            start_ms: None,
            next_boundary_ms: 0.0,
            tick: 0,
            skipped_steps: 0,
        }
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn skipped_steps(&self) -> u64 {
        self.skipped_steps
    }
    pub fn current_snapshot(&self) -> TimerSnapshot {
        TimerSnapshot::new(self.tick, self.frequency_hz, self.skipped_steps)
    }
    pub fn delta_ms(&self) -> f64 {
        self.delta_ms
    }
    pub fn next_deadline_ms(&self) -> Option<f64> {
        self.start_ms.map(|_| self.next_boundary_ms)
    }
    pub fn time_until_next_boundary(&self, now_ms: f64) -> f64 {
        match self.next_deadline_ms() {
            Some(deadline) => (deadline - now_ms).max(0.0),
            None => 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.start_ms = None;
        self.next_boundary_ms = 0.0;
        self.tick = 0;
        self.skipped_steps = 0;
    }
    pub fn start_or_resume(&mut self, now_ms: f64) {
        self.start_ms = Some(now_ms);
        self.next_boundary_ms = now_ms + self.delta_ms;
    }
    pub fn pause(&mut self) {
        self.start_ms = None;
        self.next_boundary_ms = 0.0;
    }

    pub fn due_steps(&mut self, now_ms: f64) -> Vec<SchedulerEmission> {
        if self.start_ms.is_none() {
            self.start_ms = Some(now_ms);
            self.next_boundary_ms = now_ms + self.delta_ms;
            return Vec::new();
        }
        if now_ms + f64::EPSILON < self.next_boundary_ms {
            return Vec::new();
        }

        let mut due = ((now_ms - self.next_boundary_ms) / self.delta_ms).floor() as u64 + 1;
        let emit = due.min(self.max_catch_up_steps);
        due -= emit;
        if due > 0 {
            self.skipped_steps += due;
            self.tick += due;
            self.next_boundary_ms += self.delta_ms * due as f64;
        }

        let mut emissions = Vec::with_capacity(emit as usize);
        for _ in 0..emit {
            self.tick += 1;
            self.next_boundary_ms += self.delta_ms;
            emissions.push(SchedulerEmission {
                snapshot: self.current_snapshot(),
            });
        }
        emissions
    }

    pub fn emit_exact_steps(&mut self, count: usize) -> Vec<SchedulerEmission> {
        let mut emissions = Vec::with_capacity(count);
        if self.start_ms.is_none() {
            self.start_ms = Some(0.0);
            self.next_boundary_ms = self.delta_ms;
        }
        for _ in 0..count {
            self.tick += 1;
            self.next_boundary_ms += self.delta_ms;
            emissions.push(SchedulerEmission {
                snapshot: self.current_snapshot(),
            });
        }
        emissions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_tick_zero() {
        assert_eq!(FixedStepScheduler::new(60, 8).tick(), 0);
    }

    #[test]
    fn constructing_runtime_emits_no_step() {
        assert!(FixedStepScheduler::new(60, 8).due_steps(10.0).is_empty());
    }

    #[test]
    fn one_fixed_step() {
        let mut s = FixedStepScheduler::new(60, 8);
        s.due_steps(0.0);
        assert_eq!(s.due_steps(1000.0 / 60.0).len(), 1);
    }

    #[test]
    fn hz_120_delta() {
        assert!((TimerSnapshot::new(1, 120, 0).delta_ms - 8.333333333333334).abs() < 0.000001);
    }

    #[test]
    fn elapsed_derived_from_tick() {
        let snap = TimerSnapshot::new(3, 120, 0);
        assert!((snap.elapsed_ms - 25.0).abs() < 0.000001);
    }

    #[test]
    fn late_wake_emits_multiple_steps() {
        let mut s = FixedStepScheduler::new(100, 8);
        s.due_steps(0.0);
        assert_eq!(s.due_steps(35.0).len(), 3);
    }

    #[test]
    fn catch_up_capped() {
        let mut s = FixedStepScheduler::new(100, 2);
        s.due_steps(0.0);
        assert_eq!(s.due_steps(100.0).len(), 2);
    }

    #[test]
    fn excess_steps_increment_skipped_count() {
        let mut s = FixedStepScheduler::new(100, 2);
        s.due_steps(0.0);
        let out = s.due_steps(100.0);
        assert_eq!(out.last().unwrap().snapshot.skipped_steps, 8.0 as u64);
    }

    #[test]
    fn callback_jitter_does_not_change_delta() {
        let mut s = FixedStepScheduler::new(100, 8);
        s.due_steps(0.0);
        let out = s.due_steps(11.7);
        assert_eq!(out[0].snapshot.delta_ms, 10.0);
    }
}
