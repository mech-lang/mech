use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use mech_core::MResult;
use mech_runtime::{
    ConfigValue, HostManifestConfig, RuntimeHostFactory, RuntimeHostInputDriver,
    RuntimeHostInstallation, RuntimeIngress, materialize_host_manifest,
};

use crate::{
    FixedStepScheduler, MonotonicTimerBackend, SharedTimerSnapshot, TimerResourceProvider,
    TimerSnapshot, new_shared_snapshot, timer_error, timer_host_manifest,
    timer_settings_from_config,
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

pub struct NativeTimerInputDriver<B: MonotonicTimerBackend + Send + Sync> {
    instance: String,
    backend: B,
    scheduler: Arc<Mutex<FixedStepScheduler>>,
    snapshot: SharedTimerSnapshot,
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
            ingress: Arc::new(Mutex::new(None)),
            live: Arc::new(AtomicBool::new(false)),
            worker: Arc::new(Mutex::new(None)),
            stop_sender: Arc::new(Mutex::new(None)),
        }
    }
}
impl<B: MonotonicTimerBackend + Send + Sync> RuntimeHostInputDriver for NativeTimerInputDriver<B> {
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
        if self.is_live() {
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
        let mut worker_guard = self
            .worker
            .lock()
            .map_err(|_| timer_error("TimerDriverStart", "timer worker lock is poisoned"))?;
        if worker_guard.is_some() {
            return Ok(());
        }
        self.scheduler
            .lock()
            .map_err(|_| timer_error("TimerDriverStart", "timer scheduler lock is poisoned"))?
            .reset();
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
        let instance = self.instance.clone();
        *worker_guard = Some(thread::spawn(move || {
            while live.load(Ordering::SeqCst) {
                if let Ok(now) = backend.now_ms() {
                    let emissions = scheduler
                        .lock()
                        .ok()
                        .map(|mut s| s.due_steps(now))
                        .unwrap_or_default();
                    for emission in emissions {
                        if let Ok(mut guard) = snapshot.lock() {
                            *guard = emission.snapshot;
                        }
                        match emission
                            .snapshot
                            .into_host_input(&instance)
                            .and_then(|p| ingress.submit(p))
                        {
                            Ok(()) => {}
                            Err(err) if err.kind_name() == "RuntimeIngressFull" => {}
                            Err(err) if err.kind_name() == "RuntimeIngressClosed" => {
                                live.store(false, Ordering::SeqCst);
                                break;
                            }
                            Err(_) => {
                                live.store(false, Ordering::SeqCst);
                                break;
                            }
                        }
                    }
                } else {
                    live.store(false, Ordering::SeqCst);
                    break;
                }
                match stop_receiver.recv_timeout(Duration::from_millis(1)) {
                    Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                    Err(RecvTimeoutError::Timeout) => {}
                }
            }
            live.store(false, Ordering::SeqCst);
        }));
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
        Ok(())
    }
    fn is_live(&self) -> bool {
        self.live.load(Ordering::SeqCst)
    }
}
impl<B: MonotonicTimerBackend + Send + Sync> Drop for NativeTimerInputDriver<B> {
    fn drop(&mut self) {
        let _ = self.stop();
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
