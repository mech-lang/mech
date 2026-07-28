use super::*;

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
    for (source, bindings) in &self.live_input_bindings {
      let mut driven = false;
      for driver in &self.input_drivers[..self.attached_input_driver_count] {
        if extension::invoke_extension_value(
          "host input driver",
          "drives",
          || driver.drives(source),
        )? {
          driven = true;
          break;
        }
      }
      if driven {
        count += bindings.len();
      }
    }
    Ok(count)
  }

  pub fn has_driven_live_input_bindings(&self) -> MResult<bool> {
    Ok(self.driven_live_input_binding_count()? > 0)
  }

  pub fn pending_host_input_count(&self) -> MResult<usize> {
    let guard = self.host_input_queue.lock().map_err(|_| crate::input::input_error("RuntimeIngressUnavailable", "host input queue lock is poisoned"))?;
    Ok(guard.queue.len())
  }

  pub fn drain_host_inputs(&mut self, max_inputs: usize) -> MResult<Vec<crate::RuntimeHostInputOutcome>> {
    let mut outcomes = Vec::new();
    for _ in 0..max_inputs {
      let input = {
        let mut guard = self.host_input_queue.lock().map_err(|_| crate::input::input_error("RuntimeIngressUnavailable", "host input queue lock is poisoned"))?;
        guard.queue.pop_front()
      };
      let Some(input) = input else { break; };
      outcomes.push(self.apply_host_input(input)?);
    }
    Ok(outcomes)
  }

  pub fn close_ingress(&mut self) -> MResult<()> {
    let mut guard = self.host_input_queue.lock().map_err(|_| crate::input::input_error("RuntimeIngressUnavailable", "host input queue lock is poisoned"))?;
    guard.closed = true;
    Ok(())
  }

  pub fn start_input_drivers(&mut self) -> MResult<()> {
    if self.ingress().is_closed()? {
      return Err(crate::input::input_error("RuntimeIngressClosed", "cannot start input drivers after ingress is closed"));
    }
    let mut started = vec![false; self.attached_input_driver_count];
    for index in 0..self.attached_input_driver_count {
      if extension::invoke_extension_value(
        "host input driver",
        "is_live",
        || self.input_drivers[index].is_live(),
      )? {
        continue;
      }
      let has_driven_input = {
        let driver = &self.input_drivers[index];
        self
          .live_input_bindings
          .iter()
          .try_fold(false, |driven, (source, bindings)| {
            if driven || bindings.is_empty() {
              return Ok(driven);
            }
            extension::invoke_extension_value(
              "host input driver",
              "drives",
              || driver.drives(source),
            )
          })?
      };
      if !has_driven_input { continue; }
      if let Err(error) = extension::invoke_extension(
        "host input driver",
        "start",
        || self.input_drivers[index].start(),
      ) {
        let mut cleanup_failures = Vec::new();
        for cleanup_index in (0..self.attached_input_driver_count).rev() {
          if !started[cleanup_index] { continue; }
          if let Err(cleanup_error) = extension::invoke_extension(
            "host input driver",
            "stop",
            || self.input_drivers[cleanup_index].stop(),
          ) {
            cleanup_failures.push(format!(
              "input driver {} stop failed: {:?}",
              cleanup_index,
              cleanup_error,
            ));
          }
        }
        if !cleanup_failures.is_empty() {
          return Err(self.poison_program_operation(
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
    for driver in self.input_drivers[..self.attached_input_driver_count].iter_mut().rev() {
      if let Err(error) = extension::invoke_extension(
        "host input driver",
        "stop",
        || driver.stop(),
      ) {
        if error.kind_name() == "RuntimeExtensionPanicked" {
          panic_failures.push(format!("{:?}", error));
        }
        if first_error.is_none() { first_error = Some(error); }
      }
    }
    if !panic_failures.is_empty() {
      return Err(self.poison_program_operation(
        "stop_input_drivers",
        None,
        first_error
          .as_ref()
          .map(|error| format!("{:?}", error))
          .unwrap_or_else(|| "input driver cleanup panicked".to_string()),
        panic_failures,
      ));
    }
    if let Some(error) = first_error { return Err(error); }
    Ok(())
  }
}
