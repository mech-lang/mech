use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use mech_core::MResult;
use mech_runtime::{RuntimeHostInputDriver, RuntimeIngress};

use crate::{
    FixedStepScheduler, MonotonicTimerBackend, SharedTimerSnapshot, TimerSnapshot,
    new_shared_snapshot, timer_error,
};
use crate::delivery::{TimerSubmitState, submit_pending_timer_snapshots};

#[derive(Clone, Debug, Default)]
pub struct ManualMonotonicTimerBackend {
    now: Arc<Mutex<f64>>,
}

impl ManualMonotonicTimerBackend {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn advance_ms(&self, delta_ms: f64) -> MResult<()> {
        if delta_ms < 0.0 {
            return Err(timer_error(
                "ManualTimer",
                "manual timer cannot move backwards",
            ));
        }
        *self
            .now
            .lock()
            .map_err(|_| timer_error("ManualTimer", "manual timer lock is poisoned"))? += delta_ms;
        Ok(())
    }
    pub fn set_ms(&self, now_ms: f64) -> MResult<()> {
        *self
            .now
            .lock()
            .map_err(|_| timer_error("ManualTimer", "manual timer lock is poisoned"))? = now_ms;
        Ok(())
    }
}

impl MonotonicTimerBackend for ManualMonotonicTimerBackend {
    fn now_ms(&self) -> MResult<f64> {
        Ok(*self
            .now
            .lock()
            .map_err(|_| timer_error("ManualTimer", "manual timer lock is poisoned"))?)
    }
}

#[derive(Debug)]
pub struct ManualTimerInputDriver {
    instance: String,
    backend: ManualMonotonicTimerBackend,
    scheduler: FixedStepScheduler,
    snapshot: SharedTimerSnapshot,
    ingress: Option<RuntimeIngress>,
    pending: VecDeque<TimerSnapshot>,
    live: bool,
}

impl ManualTimerInputDriver {
    pub fn new(instance: impl Into<String>, frequency_hz: u64, max_catch_up_steps: u64) -> Self {
        Self::with_backend(
            instance,
            ManualMonotonicTimerBackend::new(),
            frequency_hz,
            max_catch_up_steps,
        )
    }
    pub fn with_backend(
        instance: impl Into<String>,
        backend: ManualMonotonicTimerBackend,
        frequency_hz: u64,
        max_catch_up_steps: u64,
    ) -> Self {
        Self {
            instance: instance.into(),
            backend,
            scheduler: FixedStepScheduler::new(frequency_hz, max_catch_up_steps),
            snapshot: new_shared_snapshot(TimerSnapshot::new(0, frequency_hz, 0)),
            ingress: None,
            pending: VecDeque::new(),
            live: false,
        }
    }
    pub fn backend(&self) -> ManualMonotonicTimerBackend {
        self.backend.clone()
    }
    pub fn snapshot(&self) -> SharedTimerSnapshot {
        self.snapshot.clone()
    }
    pub fn advance_ms(&self, delta_ms: f64) -> MResult<()> {
        self.backend.advance_ms(delta_ms)
    }
    pub fn publish_due_steps(&mut self) -> MResult<usize> {
        let mut submitted = self.flush_pending()?;
        if !self.pending.is_empty() {
            return Ok(submitted);
        }
        let now = self.backend.now_ms()?;
        self.pending
            .extend(self.scheduler.due_steps(now).into_iter().map(|e| e.snapshot));
        submitted += self.flush_pending()?;
        Ok(submitted)
    }
    pub fn publish_steps(&mut self, count: usize) -> MResult<usize> {
        let mut submitted = self.flush_pending()?;
        if !self.pending.is_empty() {
            return Ok(submitted);
        }
        self.pending.extend(
            self.scheduler
                .emit_exact_steps(count)
                .into_iter()
                .map(|e| e.snapshot),
        );
        submitted += self.flush_pending()?;
        Ok(submitted)
    }
    pub fn pending_emission_count(&self) -> usize {
        self.pending.len()
    }
    fn flush_pending(&mut self) -> MResult<usize> {
        let (submitted, state) = submit_pending_timer_snapshots(
            &self.instance,
            self.ingress.as_ref(),
            &self.snapshot,
            &mut self.pending,
        )?;
        if state == TimerSubmitState::Closed {
            self.live = false;
        }
        Ok(submitted)
    }
}

impl RuntimeHostInputDriver for ManualTimerInputDriver {
    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        if self.live {
            return Err(timer_error(
                "TimerDriverAttach",
                "cannot attach manual timer driver while live",
            ));
        }
        if self.ingress.is_some() {
            return Err(timer_error(
                "TimerDriverAttach",
                "manual timer driver is already attached",
            ));
        }
        self.ingress = Some(ingress);
        Ok(())
    }
    fn start(&mut self) -> MResult<()> {
        self.live = true;
        Ok(())
    }
    fn stop(&mut self) -> MResult<()> {
        self.live = false;
        Ok(())
    }
    fn is_live(&self) -> bool {
        self.live
    }
}
