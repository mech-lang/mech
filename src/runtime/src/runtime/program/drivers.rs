use crate::runtime::{MechRuntime, RuntimeInvalidOperationError, extension};
use mech_core::{MResult, MechError};

impl MechRuntime {
    pub fn ingress(&self) -> crate::RuntimeIngress {
        crate::RuntimeIngress::new(self.host_input_queue.clone())
    }

    pub fn has_pending_host_inputs(&self) -> MResult<bool> {
        Ok(self.pending_host_input_count()? > 0)
    }

    pub fn input_driver_count(&self) -> usize {
        self.attached_input_driver_count
    }

    pub fn has_input_drivers(&self) -> bool {
        self.input_driver_count() > 0
    }

    pub fn driven_live_input_binding_count(&self) -> MResult<usize> {
        let mut count = 0;
        #[cfg(feature = "resident-routing")]
        if let crate::runtime::program::ActiveProgramExecution::ResidentExternal(execution) =
            &self.active_program
        {
            for source in execution.trigger_sources.iter() {
                let mut driven = false;
                for driver in &self.input_drivers[..self.attached_input_driver_count] {
                    if extension::invoke_extension_value("host input driver", "drives", || {
                        driver.drives(source)
                    })? {
                        driven = true;
                        break;
                    }
                }
                count += usize::from(driven);
            }
        }
        Ok(count)
    }

    pub fn has_driven_live_input_bindings(&self) -> MResult<bool> {
        Ok(self.driven_live_input_binding_count()? > 0)
    }

    pub fn pending_host_input_count(&self) -> MResult<usize> {
        let guard = self.host_input_queue.lock().map_err(|_| {
            crate::input::input_error(
                "RuntimeIngressUnavailable",
                "host input queue lock is poisoned",
            )
        })?;
        Ok(guard.queue.len())
    }

    pub fn drain_host_inputs(
        &mut self,
        max_inputs: usize,
    ) -> MResult<Vec<crate::RuntimeHostInputOutcome>> {
        if max_inputs == 0 || self.pending_host_input_count()? == 0 {
            return Ok(Vec::new());
        }
        #[cfg(feature = "resident-routing")]
        if matches!(
            self.active_program,
            crate::runtime::program::ActiveProgramExecution::ResidentExternal(_)
        ) {
            let outcome = self.drain_resident_host_inputs(max_inputs)?;
            if let Some(turn) = outcome.turn.as_ref() {
                if let Some(error) = crate::runtime::program::resident_host_turn_error(turn) {
                    return Err(error);
                }
            }
            let last = outcome.dequeued_packets.saturating_sub(1);
            return Ok((0..outcome.dequeued_packets)
                .map(|index| crate::RuntimeHostInputOutcome {
                    update_count: 0,
                    ignored_update_count: 0,
                    binding_count: 0,
                    resident_turn: (index == last).then(|| outcome.turn.clone()).flatten(),
                })
                .collect());
        }
        Err(MechError::new(
            RuntimeInvalidOperationError {
                operation: "drain_host_inputs",
                reason: "queued host input requires an active resident external program".to_owned(),
            },
            None,
        ))
    }

    pub fn close_ingress(&mut self) -> MResult<()> {
        let mut guard = self.host_input_queue.lock().map_err(|_| {
            crate::input::input_error(
                "RuntimeIngressUnavailable",
                "host input queue lock is poisoned",
            )
        })?;
        guard.closed = true;
        Ok(())
    }

    pub fn start_input_drivers(&mut self) -> MResult<()> {
        if self.ingress().is_closed()? {
            return Err(crate::input::input_error(
                "RuntimeIngressClosed",
                "cannot start input drivers after ingress is closed",
            ));
        }
        let mut started = vec![false; self.attached_input_driver_count];
        for index in 0..self.attached_input_driver_count {
            if extension::invoke_extension_value("host input driver", "is_live", || {
                self.input_drivers[index].is_live()
            })? {
                continue;
            }
            let has_driven_input = {
                let driver = &self.input_drivers[index];
                #[cfg(feature = "resident-routing")]
                let resident = match &self.active_program {
                    crate::runtime::program::ActiveProgramExecution::ResidentExternal(
                        execution,
                    ) => execution
                        .trigger_sources
                        .iter()
                        .try_fold(false, |driven, source| {
                            if driven {
                                return Ok(true);
                            }
                            extension::invoke_extension_value("host input driver", "drives", || {
                                driver.drives(source)
                            })
                        })?,
                    _ => false,
                };
                #[cfg(not(feature = "resident-routing"))]
                let resident = false;
                resident
            };
            if !has_driven_input {
                continue;
            }
            if let Err(error) = extension::invoke_extension("host input driver", "start", || {
                self.input_drivers[index].start()
            }) {
                let mut cleanup_failures = Vec::new();
                for cleanup_index in (0..self.attached_input_driver_count).rev() {
                    if !started[cleanup_index] {
                        continue;
                    }
                    if let Err(cleanup_error) =
                        extension::invoke_extension("host input driver", "stop", || {
                            self.input_drivers[cleanup_index].stop()
                        })
                    {
                        cleanup_failures.push(format!(
                            "input driver {} stop failed: {:?}",
                            cleanup_index, cleanup_error,
                        ));
                    }
                }
                if !cleanup_failures.is_empty() {
                    return Err(self.poison_runtime_operation(
                        "start_input_drivers",
                        None,
                        format!("{:?}", error),
                        cleanup_failures,
                    ));
                }
                return Err(error);
            }
            started[index] = true;
        }
        Ok(())
    }

    pub fn stop_input_drivers(&mut self) -> MResult<()> {
        let mut first_error = None;
        let mut panic_failures = Vec::new();
        for driver in self.input_drivers[..self.attached_input_driver_count]
            .iter_mut()
            .rev()
        {
            if let Err(error) =
                extension::invoke_extension("host input driver", "stop", || driver.stop())
            {
                if error.kind_name() == "RuntimeExtensionPanicked" {
                    panic_failures.push(format!("{:?}", error));
                }
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
        if !panic_failures.is_empty() {
            return Err(self.poison_runtime_operation(
                "stop_input_drivers",
                None,
                first_error
                    .as_ref()
                    .map(|error| format!("{:?}", error))
                    .unwrap_or_else(|| "input driver cleanup panicked".to_string()),
                panic_failures,
            ));
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        Ok(())
    }
}
