//! Runtime shutdown and cleanup lifecycle.

use super::{MechRuntime, extension};
use crate::RuntimeEventKind;
use mech_core::MResult;

impl MechRuntime {
    pub fn shutdown(&mut self) -> MResult<()> {
        let mut first_error = None;

        if let Err(error) = self.close_ingress() {
            first_error = Some(error);
        }

        if let Err(error) = self.stop_input_drivers() {
            if first_error.is_none() {
                first_error = Some(error);
            }
        }
        self.input_driver_cleanup_armed = false;

        match self.runtime_context() {
            Ok(mut context) => {
                if let Err(error) = self.emit_event_to_context(
                    &mut context,
                    RuntimeEventKind::RuntimeShutdown {
                        runtime_id: self.id,
                    },
                ) {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl Drop for MechRuntime {
    fn drop(&mut self) {
        if self.input_driver_cleanup_armed {
            drop(self.close_ingress());
            for driver in self.input_drivers[..self.attached_input_driver_count]
                .iter_mut()
                .rev()
            {
                drop(extension::catch_extension(
                    "host input driver",
                    "stop",
                    || driver.stop(),
                ));
            }
            self.input_driver_cleanup_armed = false;
        }
    }
}
