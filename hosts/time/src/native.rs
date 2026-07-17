use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use chrono::{Local, Timelike};
use mech_core::MResult;
use mech_runtime::{materialize_host_manifest, ConfigValue, HostManifestConfig, RuntimeHostFactory, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInstallation, RuntimeIngress};

use crate::{CLOCK_PATHS, new_shared_snapshot, time_error, time_host_manifest, time_settings_from_config, SharedTimeSnapshot, TimeBackend, TimeResourceProvider, TimeSnapshot};

#[derive(Clone, Copy, Debug, Default)]
pub struct NativeTimeBackend;

impl TimeBackend for NativeTimeBackend {
  fn snapshot(&self) -> MResult<TimeSnapshot> {
    let now = Local::now();
    Ok(TimeSnapshot {
      unix_ms: now.timestamp_millis() as f64,
      hour: now.hour() as f64,
      minute: now.minute() as f64,
      second: now.second() as f64,
      millisecond: (now.nanosecond() / 1_000_000) as f64,
    })
  }
}

pub struct NativeTimeInputDriver<B>
where
  B: TimeBackend + Send + Sync,
{
  instance: String,
  backend: B,
  ingress: Arc<Mutex<Option<RuntimeIngress>>>,
  live: Arc<AtomicBool>,
  snapshot: SharedTimeSnapshot,
  interval: Duration,
  worker: Arc<Mutex<Option<JoinHandle<()>>>>,
  stop_sender: Arc<Mutex<Option<Sender<()>>>>,
}

impl<B> std::fmt::Debug for NativeTimeInputDriver<B>
where
  B: TimeBackend + Send + Sync,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("NativeTimeInputDriver")
      .field("instance", &self.instance)
      .field("backend", &self.backend)
      .field("live", &self.is_live())
      .field("interval", &self.interval)
      .finish_non_exhaustive()
  }
}

impl<B> NativeTimeInputDriver<B>
where
  B: TimeBackend + Send + Sync,
{
  pub fn new(instance: impl Into<String>, backend: B, snapshot: SharedTimeSnapshot, interval: Duration) -> Self {
    Self {
      instance: instance.into(),
      backend,
      ingress: Arc::new(Mutex::new(None)),
      live: Arc::new(AtomicBool::new(false)),
      snapshot,
      interval,
      worker: Arc::new(Mutex::new(None)),
      stop_sender: Arc::new(Mutex::new(None)),
    }
  }
}

impl<B> RuntimeHostInputDriver for NativeTimeInputDriver<B>
where
  B: TimeBackend + Send + Sync,
{
  fn drives(&self, source: &RuntimeHostInputSource) -> bool {
    source.base_uri() == format!("time://{}/clock", self.instance) && CLOCK_PATHS.contains(&source.path())
  }

  fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
    if self.is_live() {
      return Err(time_error("TimeDriverAttach", "cannot attach native time driver while live"));
    }
    let mut guard = self.ingress.lock().map_err(|_| time_error("TimeDriverAttach", "time ingress lock is poisoned"))?;
    if guard.is_some() {
      return Err(time_error("TimeDriverAttach", "native time driver is already attached"));
    }
    *guard = Some(ingress);
    Ok(())
  }

  fn start(&mut self) -> MResult<()> {
    if self.is_live() && self.worker.lock().map_err(|_| time_error("TimeDriverStart", "time worker lock is poisoned"))?.is_some() {
      return Ok(());
    }
    let ingress = self.ingress.lock().map_err(|_| time_error("TimeDriverStart", "time ingress lock is poisoned"))?
      .clone()
      .ok_or_else(|| time_error("TimeDriverStart", "native time driver must be attached before start"))?;
    let mut worker_guard = self.worker.lock().map_err(|_| time_error("TimeDriverStart", "time worker lock is poisoned"))?;
    if worker_guard.is_some() { return Ok(()); }
    let (stop_sender, stop_receiver) = mpsc::channel();
    *self.stop_sender.lock().map_err(|_| time_error("TimeDriverStart", "time stop-signal lock is poisoned"))? = Some(stop_sender);
    self.live.store(true, Ordering::SeqCst);
    let live = self.live.clone();
    let backend = self.backend.clone();
    let snapshot = self.snapshot.clone();
    let interval = self.interval;
    let instance = self.instance.clone();
    *worker_guard = Some(thread::spawn(move || {
      while live.load(Ordering::SeqCst) {
        match backend.snapshot() {
          Ok(next) => {
            if let Ok(mut guard) = snapshot.lock() { *guard = next; }
            match next.into_host_input(&instance).and_then(|packet| ingress.submit(packet)) {
              Ok(()) => {}
              Err(err) => match err.kind_name().as_str() {
                "RuntimeIngressFull" => {
                  // Skip this snapshot and let the next interval try again.
                }
                "RuntimeIngressClosed" => {
                  live.store(false, Ordering::SeqCst);
                  break;
                }
                _ => {
                  live.store(false, Ordering::SeqCst);
                  break;
                }
              },
            }
          }
          Err(_) => {
            live.store(false, Ordering::SeqCst);
            break;
          }
        }
        match stop_receiver.recv_timeout(interval) {
          Ok(()) => break,
          Err(RecvTimeoutError::Disconnected) => break,
          Err(RecvTimeoutError::Timeout) => {}
        }
      }
      live.store(false, Ordering::SeqCst);
    }));
    Ok(())
  }

  fn stop(&mut self) -> MResult<()> {
    self.live.store(false, Ordering::SeqCst);
    let stop_sender = self.stop_sender.lock().map_err(|_| time_error("TimeDriverStop", "time stop-signal lock is poisoned"))?.take();
    if let Some(sender) = stop_sender {
      let _ = sender.send(());
    }
    let handle = self.worker.lock().map_err(|_| time_error("TimeDriverStop", "time worker lock is poisoned"))?.take();
    if let Some(handle) = handle {
      handle.join().map_err(|_| time_error("TimeDriverStop", "native time worker panicked during shutdown"))?;
    }
    Ok(())
  }

  fn is_live(&self) -> bool { self.live.load(Ordering::SeqCst) }
}

impl<B> Drop for NativeTimeInputDriver<B>
where
  B: TimeBackend + Send + Sync,
{
  fn drop(&mut self) { let _ = self.stop(); }
}

#[derive(Debug)]
pub struct NativeTimeHostFactory<B>
where
  B: TimeBackend + Send + Sync,
{
  backend: B,
  manifest: HostManifestConfig,
}

impl NativeTimeHostFactory<NativeTimeBackend> {
  pub fn new() -> MResult<Self> { Self::with_backend(NativeTimeBackend) }
}

impl<B> NativeTimeHostFactory<B>
where
  B: TimeBackend + Send + Sync,
{
  pub fn with_backend(backend: B) -> MResult<Self> {
    Ok(Self { backend, manifest: time_host_manifest()? })
  }
}

impl<B> RuntimeHostFactory for NativeTimeHostFactory<B>
where
  B: TimeBackend + Send + Sync,
{
  fn provider_name(&self) -> &str { "time" }
  fn manifest(&self) -> &HostManifestConfig { &self.manifest }
  fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
    time_settings_from_config(settings).map(|_| ())
  }
  fn instantiate(&self, instance_name: &str, settings: &ConfigValue) -> MResult<RuntimeHostInstallation> {
    let settings = time_settings_from_config(settings)?;
    let initial = self.backend.snapshot()?;
    let snapshot = new_shared_snapshot(initial);
    Ok(RuntimeHostInstallation {
      interface: materialize_host_manifest(instance_name, &self.manifest)?,
      resource_providers: vec![Box::new(TimeResourceProvider::new(instance_name, snapshot.clone()))],
      input_drivers: vec![Box::new(NativeTimeInputDriver::new(
        instance_name,
        self.backend.clone(),
        snapshot,
        Duration::from_millis(settings.interval_ms),
      ))],
    })
  }
}
