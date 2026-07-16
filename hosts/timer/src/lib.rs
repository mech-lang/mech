pub mod config;
mod delivery;
pub mod manual;
pub mod module;
pub mod provider;
pub mod scheduler;
pub mod snapshot;

#[cfg(feature = "browser")]
pub mod browser;
#[cfg(feature = "native")]
pub mod native;

pub use config::{TimerHostSettings, timer_settings_from_config};
pub use manual::{ManualMonotonicTimerBackend, ManualTimerInputDriver};
pub use module::{TIMER_HOST_MCFG, timer_host_manifest};
pub use provider::TimerResourceProvider;
pub use scheduler::{FixedStepScheduler, SchedulerEmission};
pub use snapshot::{
    MonotonicTimerBackend, SharedTimerSnapshot, TIMER_PATHS, TimerSnapshot, new_shared_snapshot,
};

#[cfg(feature = "browser")]
pub use browser::{BrowserMonotonicTimerBackend, BrowserTimerHostFactory, BrowserTimerInputDriver};
#[cfg(feature = "native")]
pub use native::{NativeMonotonicTimerBackend, NativeTimerHostFactory, NativeTimerInputDriver};

use mech_core::{MechError, MechErrorKind};

#[derive(Debug, Clone)]
pub struct TimerHostError {
    pub name: &'static str,
    pub message: String,
}
impl MechErrorKind for TimerHostError {
    fn name(&self) -> &str {
        self.name
    }
    fn message(&self) -> String {
        self.message.clone()
    }
}

pub(crate) fn timer_error(name: &'static str, message: impl Into<String>) -> MechError {
    MechError::new(
        TimerHostError {
            name,
            message: message.into(),
        },
        None,
    )
}
