use std::sync::{Arc, Mutex};

use mech_core::MResult;
use mech_runtime::{RuntimeHostInputDriver, RuntimeIngress};

use crate::{
    FixedStepScheduler, MonotonicTimerBackend, SharedTimerSnapshot, TimerSnapshot,
    new_shared_snapshot, timer_error,
};

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
        let now = self.backend.now_ms()?;
        let emissions = self.scheduler.due_steps(now);
        self.publish_emissions(emissions)
    }
    pub fn publish_steps(&mut self, count: usize) -> MResult<usize> {
        let mut emitted = 0;
        for _ in 0..count {
            let next = self.scheduler.current_snapshot().tick + 1;
            let frequency_hz =
                (1000.0 / self.scheduler.current_snapshot().delta_ms.max(0.000001)).round() as u64;
            let snap =
                TimerSnapshot::new(next, frequency_hz.max(1), self.scheduler.skipped_steps());
            if let Ok(mut guard) = self.snapshot.lock() {
                *guard = snap;
            }
            if self.ingress.is_none() {
                emitted += 1;
                continue;
            }
            if let Some(ingress) = &self.ingress {
                match ingress.submit(snap.into_host_input(&self.instance)?) {
                    Ok(()) => emitted += 1,
                    Err(err) if err.kind_name() == "RuntimeIngressFull" => {}
                    Err(err) if err.kind_name() == "RuntimeIngressClosed" => {
                        self.live = false;
                        break;
                    }
                    Err(err) => return Err(err),
                }
            }
        }
        Ok(emitted)
    }
    fn publish_emissions(&mut self, emissions: Vec<crate::SchedulerEmission>) -> MResult<usize> {
        let mut emitted = 0;
        for emission in emissions {
            if let Ok(mut guard) = self.snapshot.lock() {
                *guard = emission.snapshot;
            }
            if self.ingress.is_none() {
                emitted += 1;
                continue;
            }
            if let Some(ingress) = &self.ingress {
                match ingress.submit(emission.snapshot.into_host_input(&self.instance)?) {
                    Ok(()) => emitted += 1,
                    Err(err) if err.kind_name() == "RuntimeIngressFull" => {}
                    Err(err) if err.kind_name() == "RuntimeIngressClosed" => {
                        self.live = false;
                        break;
                    }
                    Err(err) => return Err(err),
                }
            }
        }
        Ok(emitted)
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
