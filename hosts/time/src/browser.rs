use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use mech_core::MResult;
use mech_runtime::{materialize_host_manifest, ConfigValue, HostManifestConfig, RuntimeHostFactory, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInstallation, RuntimeIngress};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;

use crate::{time_source_matches, new_shared_snapshot, time_error, time_host_manifest, time_settings_from_config, SharedTimeSnapshot, TimeBackend, TimeResourceProvider, TimeSnapshot};

#[derive(Clone, Copy, Debug, Default)]
pub struct BrowserTimeBackend;

impl TimeBackend for BrowserTimeBackend {
  fn snapshot(&self) -> MResult<TimeSnapshot> {
    let date = js_sys::Date::new_0();
    Ok(TimeSnapshot {
      unix_ms: date.get_time(),
      hour: date.get_hours() as f64,
      minute: date.get_minutes() as f64,
      second: date.get_seconds() as f64,
      millisecond: date.get_milliseconds() as f64,
    })
  }
}

pub struct BrowserTimeInputDriver<B>
where
  B: TimeBackend,
{
  instance: String,
  backend: B,
  ingress: Option<RuntimeIngress>,
  snapshot: SharedTimeSnapshot,
  interval_ms: u64,
  interval_handle: Option<i32>,
  closure: Option<Closure<dyn FnMut()>>,
  live: Arc<AtomicBool>,
}

impl<B> std::fmt::Debug for BrowserTimeInputDriver<B>
where
  B: TimeBackend,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("BrowserTimeInputDriver")
      .field("instance", &self.instance)
      .field("backend", &self.backend)
      .field("interval_ms", &self.interval_ms)
      .field("live", &self.is_live())
      .finish_non_exhaustive()
  }
}

impl<B> BrowserTimeInputDriver<B>
where
  B: TimeBackend,
{
  pub fn new(instance: impl Into<String>, backend: B, snapshot: SharedTimeSnapshot, interval_ms: u64) -> Self {
    Self { instance: instance.into(), backend, ingress: None, snapshot, interval_ms, interval_handle: None, closure: None, live: Arc::new(AtomicBool::new(false)) }
  }
}

impl<B> RuntimeHostInputDriver for BrowserTimeInputDriver<B>
where
  B: TimeBackend,
{
  fn drives(&self, source: &RuntimeHostInputSource) -> bool {
    time_source_matches(&self.instance, source)
  }

  fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
    if self.is_live() { return Err(time_error("TimeDriverAttach", "cannot attach browser time driver while live")); }
    if self.ingress.is_some() { return Err(time_error("TimeDriverAttach", "browser time driver is already attached")); }
    self.ingress = Some(ingress);
    Ok(())
  }

  fn start(&mut self) -> MResult<()> {
    if self.is_live() { return Ok(()); }
    let ingress = self.ingress.clone().ok_or_else(|| time_error("TimeDriverStart", "browser time driver must be attached before start"))?;
    let window = web_sys::window().ok_or_else(|| time_error("TimeDriverStart", "browser Window is unavailable"))?;
    let backend = self.backend.clone();
    let snapshot = self.snapshot.clone();
    let instance = self.instance.clone();
    let live = self.live.clone();
    let callback = Closure::wrap(Box::new(move || {
      if !live.load(Ordering::SeqCst) { return; }
      if let Ok(next) = backend.snapshot() {
        if let Ok(mut guard) = snapshot.lock() { *guard = next; }
        let _ = next.into_host_input(&instance).and_then(|packet| ingress.submit(packet));
      }
    }) as Box<dyn FnMut()>);
    let handle = window
      .set_interval_with_callback_and_timeout_and_arguments_0(callback.as_ref().unchecked_ref(), self.interval_ms as i32)
      .map_err(|_| time_error("TimeDriverStart", "failed to start browser time interval"))?;
    self.interval_handle = Some(handle);
    self.closure = Some(callback);
    self.live.store(true, Ordering::SeqCst);
    Ok(())
  }

  fn stop(&mut self) -> MResult<()> {
    if let Some(handle) = self.interval_handle.take() {
      if let Some(window) = web_sys::window() {
        window.clear_interval_with_handle(handle);
      }
    }
    self.closure = None;
    self.live.store(false, Ordering::SeqCst);
    Ok(())
  }

  fn is_live(&self) -> bool { self.live.load(Ordering::SeqCst) }
}

impl<B> Drop for BrowserTimeInputDriver<B>
where
  B: TimeBackend,
{
  fn drop(&mut self) { let _ = self.stop(); }
}

#[derive(Debug)]
pub struct BrowserTimeHostFactory<B>
where
  B: TimeBackend,
{
  backend: B,
  manifest: HostManifestConfig,
}

impl BrowserTimeHostFactory<BrowserTimeBackend> {
  pub fn new() -> MResult<Self> { Self::with_backend(BrowserTimeBackend) }
}

impl<B> BrowserTimeHostFactory<B>
where
  B: TimeBackend,
{
  pub fn with_backend(backend: B) -> MResult<Self> { Ok(Self { backend, manifest: time_host_manifest()? }) }
}

impl<B> RuntimeHostFactory for BrowserTimeHostFactory<B>
where
  B: TimeBackend,
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
      input_drivers: vec![Box::new(BrowserTimeInputDriver::new(instance_name, self.backend.clone(), snapshot, settings.interval_ms))],
    })
  }
}
