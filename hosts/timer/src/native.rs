use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use mech_core::MResult;
use mech_runtime::{
    ConfigValue, HostManifestConfig, RuntimeHostFactory, RuntimeHostInputDriver,
    RuntimeHostInputSource, RuntimeHostInstallation, RuntimeIngress, materialize_host_manifest,
};

use crate::delivery::{TimerSubmitState, submit_pending_timer_snapshots};
use crate::{
    FixedStepScheduler, MonotonicTimerBackend, SharedTimerSnapshot, TimerResourceProvider,
    TimerSnapshot, new_shared_snapshot, timer_error, timer_host_manifest,
    timer_settings_from_config, timer_source_matches,
};

#[derive(Clone, Debug)]
pub struct NativeMonotonicTimerBackend {
    start: Instant,
}
impl Default for NativeMonotonicTimerBackend {
    fn default() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}
impl MonotonicTimerBackend for NativeMonotonicTimerBackend {
    fn now_ms(&self) -> MResult<f64> {
        Ok(self.start.elapsed().as_secs_f64() * 1000.0)
    }
}

struct WorkerLiveReset(Arc<AtomicBool>);

impl Drop for WorkerLiveReset {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

pub struct NativeTimerInputDriver<B: MonotonicTimerBackend + Send + Sync> {
    instance: String,
    backend: B,
    scheduler: Arc<Mutex<FixedStepScheduler>>,
    snapshot: SharedTimerSnapshot,
    pending: Arc<Mutex<VecDeque<TimerSnapshot>>>,
    ingress: Arc<Mutex<Option<RuntimeIngress>>>,
    live: Arc<AtomicBool>,
    worker: Arc<Mutex<Option<JoinHandle<()>>>>,
    stop_sender: Arc<Mutex<Option<Sender<()>>>>,
}
impl<B: MonotonicTimerBackend + Send + Sync> std::fmt::Debug for NativeTimerInputDriver<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeTimerInputDriver")
            .field("instance", &self.instance)
            .field("live", &self.is_live())
            .finish_non_exhaustive()
    }
}
impl<B: MonotonicTimerBackend + Send + Sync> NativeTimerInputDriver<B> {
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
            ingress: Arc::new(Mutex::new(None)),
            live: Arc::new(AtomicBool::new(false)),
            worker: Arc::new(Mutex::new(None)),
            stop_sender: Arc::new(Mutex::new(None)),
        }
    }

    fn prepare_worker_start(&mut self) -> MResult<bool> {
        let mut stop_sender_guard = self
            .stop_sender
            .lock()
            .map_err(|_| timer_error("TimerDriverStart", "timer stop-signal lock is poisoned"))?;
        let mut worker_guard = self
            .worker
            .lock()
            .map_err(|_| timer_error("TimerDriverStart", "timer worker lock is poisoned"))?;

        if self.live.load(Ordering::SeqCst)
            && worker_guard
                .as_ref()
                .is_some_and(|handle| !handle.is_finished())
        {
            return Ok(true);
        }

        let stop_sender = stop_sender_guard.take();
        let worker = worker_guard.take();
        drop(worker_guard);
        drop(stop_sender_guard);

        self.live.store(false, Ordering::SeqCst);

        if let Some(sender) = stop_sender {
            let _ = sender.send(());
        }

        let worker_panicked = worker.is_some_and(|handle| handle.join().is_err());

        self.scheduler
            .lock()
            .map_err(|_| timer_error("TimerDriverStart", "timer scheduler lock is poisoned"))?
            .pause();

        if worker_panicked {
            return Err(timer_error(
                "TimerDriverStart",
                "native timer worker panicked before restart",
            ));
        }

        Ok(false)
    }
}
impl<B: MonotonicTimerBackend + Send + Sync> RuntimeHostInputDriver for NativeTimerInputDriver<B> {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        timer_source_matches(&self.instance, source)
    }

    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        if self.is_live() {
            return Err(timer_error(
                "TimerDriverAttach",
                "cannot attach native timer driver while live",
            ));
        }
        let mut guard = self
            .ingress
            .lock()
            .map_err(|_| timer_error("TimerDriverAttach", "timer ingress lock is poisoned"))?;
        if guard.is_some() {
            return Err(timer_error(
                "TimerDriverAttach",
                "native timer driver is already attached",
            ));
        }
        *guard = Some(ingress);
        Ok(())
    }
    fn start(&mut self) -> MResult<()> {
        if self.prepare_worker_start()? {
            return Ok(());
        }
        let ingress = self
            .ingress
            .lock()
            .map_err(|_| timer_error("TimerDriverStart", "timer ingress lock is poisoned"))?
            .clone()
            .ok_or_else(|| {
                timer_error(
                    "TimerDriverStart",
                    "native timer driver must be attached before start",
                )
            })?;
        let now = self.backend.now_ms()?;
        self.scheduler
            .lock()
            .map_err(|_| timer_error("TimerDriverStart", "timer scheduler lock is poisoned"))?
            .start_or_resume(now);
        let (stop_sender, stop_receiver) = mpsc::channel();
        *self
            .stop_sender
            .lock()
            .map_err(|_| timer_error("TimerDriverStart", "timer stop-signal lock is poisoned"))? =
            Some(stop_sender);
        self.live.store(true, Ordering::SeqCst);
        let live = self.live.clone();
        let backend = self.backend.clone();
        let scheduler = self.scheduler.clone();
        let snapshot = self.snapshot.clone();
        let pending = self.pending.clone();
        let instance = self.instance.clone();
        let worker = thread::spawn(move || {
            let _live_reset = WorkerLiveReset(live.clone());
            while live.load(Ordering::SeqCst) {
                let state = pending.lock().map_err(|_| ()).and_then(|mut pending| {
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
                    Ok(TimerSubmitState::Full) => {
                        let wait = backend
                            .now_ms()
                            .ok()
                            .and_then(|now| {
                                scheduler
                                    .lock()
                                    .ok()
                                    .map(|scheduler| native_wait_duration(&scheduler, now))
                            })
                            .unwrap_or_else(|| Duration::from_millis(1));
                        match stop_receiver.recv_timeout(wait) {
                            Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                            Err(RecvTimeoutError::Timeout) => continue,
                        }
                    }
                    Ok(TimerSubmitState::Closed) => {
                        live.store(false, Ordering::SeqCst);
                        break;
                    }
                    Err(()) => {
                        live.store(false, Ordering::SeqCst);
                        break;
                    }
                }

                let Ok(now) = backend.now_ms() else {
                    live.store(false, Ordering::SeqCst);
                    break;
                };
                if !live.load(Ordering::SeqCst) {
                    break;
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
                    break;
                }
                let state = pending.lock().map_err(|_| ()).and_then(|mut pending| {
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
                    break;
                }
                let wait = scheduler
                    .lock()
                    .ok()
                    .map(|s| native_wait_duration(&s, now))
                    .unwrap_or_else(|| Duration::from_millis(1));
                match stop_receiver.recv_timeout(wait) {
                    Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                    Err(RecvTimeoutError::Timeout) => {}
                }
            }
        });
        *self
            .worker
            .lock()
            .map_err(|_| timer_error("TimerDriverStart", "timer worker lock is poisoned"))? =
            Some(worker);
        Ok(())
    }
    fn stop(&mut self) -> MResult<()> {
        self.live.store(false, Ordering::SeqCst);
        if let Some(sender) = self
            .stop_sender
            .lock()
            .map_err(|_| timer_error("TimerDriverStop", "timer stop lock is poisoned"))?
            .take()
        {
            let _ = sender.send(());
        }
        if let Some(handle) = self
            .worker
            .lock()
            .map_err(|_| timer_error("TimerDriverStop", "timer worker lock is poisoned"))?
            .take()
        {
            handle.join().map_err(|_| {
                timer_error(
                    "TimerDriverStop",
                    "native timer worker panicked during shutdown",
                )
            })?;
        }
        if let Ok(mut scheduler) = self.scheduler.lock() {
            scheduler.pause();
        }
        Ok(())
    }
    fn is_live(&self) -> bool {
        self.live.load(Ordering::SeqCst)
    }
}

pub fn native_wait_duration(scheduler: &FixedStepScheduler, now_ms: f64) -> Duration {
    let millis = scheduler
        .time_until_next_boundary(now_ms)
        .clamp(1.0, 1000.0);
    Duration::from_millis(millis.ceil() as u64)
}
impl<B: MonotonicTimerBackend + Send + Sync> Drop for NativeTimerInputDriver<B> {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;

    fn wait_for_finished_worker<B>(driver: &NativeTimerInputDriver<B>)
    where
        B: MonotonicTimerBackend + Send + Sync,
    {
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let finished = driver
                .worker
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|handle| handle.is_finished());
            if finished {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for native timer worker to finish"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn wait_for_pending_inputs(runtime: &mech_runtime::MechRuntime, count: usize) {
        let deadline = Instant::now() + Duration::from_secs(1);
        while runtime.pending_host_input_count().unwrap() < count {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for native timer input"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[derive(Clone, Debug)]
    struct ControlledBackend {
        calls: Arc<AtomicUsize>,
        panic_on_worker_call: bool,
    }

    impl MonotonicTimerBackend for ControlledBackend {
        fn now_ms(&self) -> MResult<f64> {
            match self.calls.fetch_add(1, Ordering::SeqCst) {
                0 => Ok(0.0),
                1 if self.panic_on_worker_call => panic!("test timer backend panic"),
                1 => Err(timer_error("TimerBackend", "test backend failure")),
                2 => Ok(10_000.0),
                3 => Ok(10_010.0),
                _ => Ok(10_010.0),
            }
        }
    }

    #[derive(Clone, Debug)]
    struct WorkingBackend {
        calls: Arc<AtomicUsize>,
    }

    impl MonotonicTimerBackend for WorkingBackend {
        fn now_ms(&self) -> MResult<f64> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(0.0)
        }
    }

    #[derive(Clone, Debug)]
    struct FullIngressPanickingBackend {
        calls: Arc<AtomicUsize>,
    }

    impl MonotonicTimerBackend for FullIngressPanickingBackend {
        fn now_ms(&self) -> MResult<f64> {
            match self.calls.fetch_add(1, Ordering::SeqCst) {
                0 => Ok(0.0),
                1 => Ok(10.0),
                2 => panic!("test timer backend panic while ingress is full"),
                _ => Ok(10_000.0),
            }
        }
    }

    fn snapshot() -> SharedTimerSnapshot {
        new_shared_snapshot(TimerSnapshot::new(0, 100, 0))
    }

    #[test]
    fn restart_after_backend_error_rebases_scheduler() {
        let runtime = mech_runtime::MechRuntime::builder()
            .host_input_capacity(4)
            .build()
            .unwrap();
        let shared_snapshot = snapshot();
        let backend = ControlledBackend {
            calls: Arc::new(AtomicUsize::new(0)),
            panic_on_worker_call: false,
        };
        let mut driver = NativeTimerInputDriver::new(
            "physics",
            backend,
            FixedStepScheduler::new(100, 8),
            shared_snapshot.clone(),
        );
        driver.attach(runtime.ingress()).unwrap();

        driver.start().unwrap();
        wait_for_finished_worker(&driver);
        assert!(!driver.is_live());

        driver.start().unwrap();
        wait_for_pending_inputs(&runtime, 1);
        assert_eq!(runtime.pending_host_input_count().unwrap(), 1);
        let snapshot = *shared_snapshot.lock().unwrap();
        assert_eq!(snapshot.tick, 1);
        assert_eq!(snapshot.skipped_steps, 0);
        driver.stop().unwrap();
    }

    #[test]
    fn panicked_worker_is_reported_once_and_can_restart() {
        let runtime = mech_runtime::MechRuntime::builder()
            .host_input_capacity(4)
            .build()
            .unwrap();
        let shared_snapshot = snapshot();
        let backend = ControlledBackend {
            calls: Arc::new(AtomicUsize::new(0)),
            panic_on_worker_call: true,
        };
        let mut driver = NativeTimerInputDriver::new(
            "physics",
            backend,
            FixedStepScheduler::new(100, 8),
            shared_snapshot.clone(),
        );
        driver.attach(runtime.ingress()).unwrap();

        driver.start().unwrap();
        wait_for_finished_worker(&driver);
        assert!(!driver.is_live());

        let error = driver.start().unwrap_err();
        assert_eq!(error.kind_name(), "TimerDriverStart");
        assert!(format!("{error:?}").contains("native timer worker panicked before restart"));
        assert!(!driver.is_live());
        assert!(driver.worker.lock().unwrap().is_none());
        assert!(driver.stop_sender.lock().unwrap().is_none());
        assert_eq!(
            driver
                .scheduler
                .lock()
                .unwrap()
                .time_until_next_boundary(10_000.0),
            0.0
        );

        driver.start().unwrap();
        wait_for_pending_inputs(&runtime, 1);
        let snapshot = *shared_snapshot.lock().unwrap();
        assert_eq!(snapshot.tick, 1);
        assert_eq!(snapshot.skipped_steps, 0);
        driver.stop().unwrap();
    }

    #[test]
    fn full_ingress_panic_leaves_scheduler_recoverable() {
        let mut runtime = mech_runtime::MechRuntime::builder()
            .host_input_capacity(1)
            .build()
            .unwrap();
        let shared_snapshot = snapshot();
        let backend = FullIngressPanickingBackend {
            calls: Arc::new(AtomicUsize::new(0)),
        };
        let mut driver = NativeTimerInputDriver::new(
            "physics",
            backend,
            FixedStepScheduler::new(100, 8),
            shared_snapshot.clone(),
        );
        driver.attach(runtime.ingress()).unwrap();
        runtime
            .ingress()
            .submit(
                TimerSnapshot::new(0, 100, 0)
                    .into_host_input("filler")
                    .unwrap(),
            )
            .unwrap();

        driver.start().unwrap();
        wait_for_finished_worker(&driver);
        assert!(!driver.is_live());

        let error = driver.start().unwrap_err();
        assert_eq!(error.kind_name(), "TimerDriverStart");
        assert!(format!("{error:?}").contains("native timer worker panicked before restart"));
        assert!(driver.scheduler.lock().is_ok());

        runtime.drain_host_inputs(1).unwrap();
        driver.start().unwrap();
        wait_for_pending_inputs(&runtime, 1);
        assert_eq!(shared_snapshot.lock().unwrap().tick, 1);
        driver.stop().unwrap();
    }

    #[test]
    fn native_start_is_idempotent_while_worker_is_live() {
        let runtime = mech_runtime::MechRuntime::builder()
            .host_input_capacity(4)
            .build()
            .unwrap();
        let backend = WorkingBackend {
            calls: Arc::new(AtomicUsize::new(0)),
        };
        let mut driver = NativeTimerInputDriver::new(
            "physics",
            backend,
            FixedStepScheduler::new(100, 8),
            snapshot(),
        );
        driver.attach(runtime.ingress()).unwrap();

        driver.start().unwrap();
        let first_thread = driver
            .worker
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .thread()
            .id();
        driver.start().unwrap();
        let second_thread = driver
            .worker
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .thread()
            .id();
        assert_eq!(first_thread, second_thread);
        driver.stop().unwrap();
    }
}

#[derive(Debug)]
pub struct NativeTimerHostFactory<B: MonotonicTimerBackend + Send + Sync> {
    backend: B,
    manifest: HostManifestConfig,
}
impl NativeTimerHostFactory<NativeMonotonicTimerBackend> {
    pub fn new() -> MResult<Self> {
        Self::with_backend(NativeMonotonicTimerBackend::default())
    }
}
impl<B: MonotonicTimerBackend + Send + Sync> NativeTimerHostFactory<B> {
    pub fn with_backend(backend: B) -> MResult<Self> {
        Ok(Self {
            backend,
            manifest: timer_host_manifest()?,
        })
    }
}
impl<B: MonotonicTimerBackend + Send + Sync> RuntimeHostFactory for NativeTimerHostFactory<B> {
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
            input_drivers: vec![Box::new(NativeTimerInputDriver::new(
                instance_name,
                self.backend.clone(),
                FixedStepScheduler::new(settings.frequency_hz, settings.max_catch_up_steps),
                snapshot,
            ))],
        })
    }
}
