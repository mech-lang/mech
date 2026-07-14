use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use mech_core::MResult;
use mech_runtime::{RuntimeHostInputDriver, RuntimeIngress};

use crate::{time_error, SharedTimeSnapshot, TimeSnapshot};

#[derive(Clone, Debug)]
pub struct ManualTimeInputDriver {
  instance: String,
  ingress: Arc<Mutex<Option<RuntimeIngress>>>,
  live: Arc<AtomicBool>,
  snapshot: SharedTimeSnapshot,
}

impl ManualTimeInputDriver {
  pub fn new(instance: impl Into<String>, snapshot: SharedTimeSnapshot) -> Self {
    Self { instance: instance.into(), ingress: Arc::new(Mutex::new(None)), live: Arc::new(AtomicBool::new(false)), snapshot }
  }

  pub fn publish(&self, snapshot: TimeSnapshot) -> MResult<()> {
    if !self.is_live() {
      return Err(time_error("TimeDriverNotLive", "manual time driver is not live"));
    }
    *self.snapshot.lock().map_err(|_| time_error("TimeDriverNotLive", "time snapshot lock is poisoned"))? = snapshot;
    let ingress = self.ingress.lock().map_err(|_| time_error("TimeDriverNotLive", "time ingress lock is poisoned"))?
      .clone()
      .ok_or_else(|| time_error("TimeDriverNotLive", "manual time driver has no ingress attachment"))?;
    ingress.submit(snapshot.into_host_input(&self.instance)?)
  }
}

impl RuntimeHostInputDriver for ManualTimeInputDriver {
  fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
    let mut guard = self.ingress.lock().map_err(|_| time_error("TimeDriverAttach", "time ingress lock is poisoned"))?;
    if guard.is_some() {
      return Err(time_error("TimeDriverAttach", "time input driver is already attached"));
    }
    *guard = Some(ingress);
    Ok(())
  }

  fn start(&mut self) -> MResult<()> {
    if self.ingress.lock().map_err(|_| time_error("TimeDriverStart", "time ingress lock is poisoned"))?.is_none() {
      return Err(time_error("TimeDriverStart", "time input driver must be attached before start"));
    }
    self.live.store(true, Ordering::SeqCst);
    Ok(())
  }

  fn stop(&mut self) -> MResult<()> {
    self.live.store(false, Ordering::SeqCst);
    Ok(())
  }

  fn is_live(&self) -> bool { self.live.load(Ordering::SeqCst) }
}
