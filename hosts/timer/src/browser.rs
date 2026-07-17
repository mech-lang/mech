use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use mech_core::MResult;
use mech_runtime::{
    ConfigValue, HostManifestConfig, RuntimeHostFactory, RuntimeHostInputDriver, RuntimeHostInputSource,
    RuntimeHostInstallation, RuntimeIngress, materialize_host_manifest,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;

use crate::{
    TIMER_PATHS,
    FixedStepScheduler, MonotonicTimerBackend, SharedTimerSnapshot, TimerResourceProvider,
    TimerSnapshot, new_shared_snapshot, timer_error, timer_host_manifest,
    timer_settings_from_config,
};
use crate::delivery::{TimerSubmitState, submit_pending_timer_snapshots};

#[derive(Clone, Copy, Debug, Default)]
pub struct BrowserMonotonicTimerBackend;
impl MonotonicTimerBackend for BrowserMonotonicTimerBackend {
    fn now_ms(&self) -> MResult<f64> {
        let window = web_sys::window()
            .ok_or_else(|| timer_error("BrowserTimer", "browser Window is unavailable"))?;
        let performance = window.performance().ok_or_else(|| {
            timer_error("BrowserTimer", "browser performance clock is unavailable")
        })?;
        Ok(performance.now())
    }
}

pub struct BrowserTimerInputDriver<B: MonotonicTimerBackend> {
    instance: String,
    backend: B,
    scheduler: Arc<Mutex<FixedStepScheduler>>,
    snapshot: SharedTimerSnapshot,
    pending: Arc<Mutex<VecDeque<TimerSnapshot>>>,
    ingress: Option<RuntimeIngress>,
    interval_handle: Option<i32>,
    closure: Option<Closure<dyn FnMut()>>,
    live: Arc<AtomicBool>,
}
impl<B: MonotonicTimerBackend> std::fmt::Debug for BrowserTimerInputDriver<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserTimerInputDriver")
            .field("instance", &self.instance)
            .field("live", &self.is_live())
            .finish_non_exhaustive()
    }
}
impl<B: MonotonicTimerBackend> BrowserTimerInputDriver<B> {
    pub fn new(
        instance: impl Into<String>,
        backend: B,
        scheduler: FixedStepScheduler,
        snapshot: SharedTimerSnapshot,
    ) -> Self {
        Self {
            instance: instance.into(),
            backend,
            scheduler: Arc::new(Mutex::new(scheduler)),
            snapshot,
            pending: Arc::new(Mutex::new(VecDeque::new())),
            ingress: None,
            interval_handle: None,
            closure: None,
            live: Arc::new(AtomicBool::new(false)),
        }
    }
}
impl<B: MonotonicTimerBackend> RuntimeHostInputDriver for BrowserTimerInputDriver<B> {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == format!("timer://{}/tick", self.instance) && TIMER_PATHS.contains(&source.path())
    }

    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        if self.is_live() {
            return Err(timer_error(
                "TimerDriverAttach",
                "cannot attach browser timer driver while live",
            ));
        }
        if self.ingress.is_some() {
            return Err(timer_error(
                "TimerDriverAttach",
                "browser timer driver is already attached",
            ));
        }
        self.ingress = Some(ingress);
        Ok(())
    }
    fn start(&mut self) -> MResult<()> {
        if self.is_live() {
            return Ok(());
        }
        let ingress = self.ingress.clone().ok_or_else(|| {
            timer_error(
                "TimerDriverStart",
                "browser timer driver must be attached before start",
            )
        })?;
        let window = web_sys::window()
            .ok_or_else(|| timer_error("TimerDriverStart", "browser Window is unavailable"))?;
        let now = self.backend.now_ms()?;
        self.scheduler
            .lock()
            .map_err(|_| timer_error("TimerDriverStart", "timer scheduler lock is poisoned"))?
            .start_or_resume(now);
        let backend = self.backend.clone();
        let scheduler = self.scheduler.clone();
        let snapshot = self.snapshot.clone();
        let pending = self.pending.clone();
        let instance = self.instance.clone();
        let live = self.live.clone();
        let callback = Closure::wrap(Box::new(move || {
            if !live.load(Ordering::SeqCst) {
                return;
            }
            let state = pending
                .lock()
                .map_err(|_| ())
                .and_then(|mut pending| {
                    submit_pending_timer_snapshots(
                        &instance,
                        Some(&ingress),
                        &snapshot,
                        &mut pending,
                    )
                    .map(|(_, state)| state)
                    .map_err(|_| ())
                });
            match state {
                Ok(TimerSubmitState::Drained) => {}
                Ok(TimerSubmitState::Full) => return,
                Ok(TimerSubmitState::Closed) | Err(()) => {
                    live.store(false, Ordering::SeqCst);
                    return;
                }
            }
            let Ok(now) = backend.now_ms() else {
                live.store(false, Ordering::SeqCst);
                return;
            };
            if !live.load(Ordering::SeqCst) {
                return;
            }
            let emissions = scheduler
                .lock()
                .ok()
                .map(|mut s| s.due_steps(now))
                .unwrap_or_default();
            if let Ok(mut pending) = pending.lock() {
                pending.extend(emissions.into_iter().map(|e| e.snapshot));
            } else {
                live.store(false, Ordering::SeqCst);
                return;
            }
            let state = pending
                .lock()
                .map_err(|_| ())
                .and_then(|mut pending| {
                    submit_pending_timer_snapshots(
                        &instance,
                        Some(&ingress),
                        &snapshot,
                        &mut pending,
                    )
                    .map(|(_, state)| state)
                    .map_err(|_| ())
                });
            if matches!(state, Ok(TimerSubmitState::Closed) | Err(())) {
                live.store(false, Ordering::SeqCst);
            }
        }) as Box<dyn FnMut()>);
        let wake_interval_ms = self
            .scheduler
            .lock()
            .map_err(|_| timer_error("TimerDriverStart", "timer scheduler lock is poisoned"))
            .map(|s| browser_wake_interval_ms(&s))?;
        let handle = window
            .set_interval_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                wake_interval_ms,
            )
            .map_err(|_| {
                timer_error("TimerDriverStart", "failed to start browser timer interval")
            })?;
        self.interval_handle = Some(handle);
        self.closure = Some(callback);
        self.live.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn stop(&mut self) -> MResult<()> {
        self.live.store(false, Ordering::SeqCst);
        if let Some(handle) = self.interval_handle.take() {
            if let Some(window) = web_sys::window() {
                window.clear_interval_with_handle(handle);
            }
        }
        self.closure = None;
        if let Ok(mut scheduler) = self.scheduler.lock() {
            scheduler.pause();
        }
        Ok(())
    }
    fn is_live(&self) -> bool {
        self.live.load(Ordering::SeqCst)
    }
}

pub fn browser_wake_interval_ms(scheduler: &FixedStepScheduler) -> i32 {
    (scheduler.delta_ms() / 2.0).floor().clamp(1.0, 16.0) as i32
}
impl<B: MonotonicTimerBackend> Drop for BrowserTimerInputDriver<B> {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[derive(Debug)]
pub struct BrowserTimerHostFactory<B: MonotonicTimerBackend> {
    backend: B,
    manifest: HostManifestConfig,
}
impl BrowserTimerHostFactory<BrowserMonotonicTimerBackend> {
    pub fn new() -> MResult<Self> {
        Self::with_backend(BrowserMonotonicTimerBackend)
    }
}
impl<B: MonotonicTimerBackend> BrowserTimerHostFactory<B> {
    pub fn with_backend(backend: B) -> MResult<Self> {
        Ok(Self {
            backend,
            manifest: timer_host_manifest()?,
        })
    }
}
impl<B: MonotonicTimerBackend> RuntimeHostFactory for BrowserTimerHostFactory<B> {
    fn provider_name(&self) -> &str {
        "timer"
    }
    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }
    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        timer_settings_from_config(settings).map(|_| ())
    }
    fn instantiate(
        &self,
        instance_name: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        let settings = timer_settings_from_config(settings)?;
        let initial = TimerSnapshot::new(0, settings.frequency_hz, 0);
        let snapshot = new_shared_snapshot(initial);
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(TimerResourceProvider::new(
                instance_name,
                snapshot.clone(),
            ))],
            input_drivers: vec![Box::new(BrowserTimerInputDriver::new(
                instance_name,
                self.backend.clone(),
                FixedStepScheduler::new(settings.frequency_hz, settings.max_catch_up_steps),
                snapshot,
            ))],
        })
    }
}
