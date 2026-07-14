#[cfg(feature = "clock_demo_cli")]
fn main() -> mech_core::MResult<()> {
  clock_cli::run()
}

#[cfg(feature = "clock_demo_cli")]
mod clock_cli {
  use std::collections::BTreeMap;
  use std::sync::atomic::{AtomicBool, Ordering};
  use std::sync::Arc;
  use std::thread;
  use std::time::Duration;

  use clap::{Arg, Command};
  use mech_core::{MResult, MechError, MechErrorKind, Value};
  use mech_host_time::NativeTimeHostFactory;
  use mech_runtime::{ConfigValue, HostInstanceConfig, MechRuntime, RunResourceGrantConfig, RuntimeBuilder, RuntimeCapabilityOperation, RuntimeCapabilityGrant};

  const CLOCK_SOURCE: &str = include_str!("../../examples/analog-clock/clock.mec");

  pub fn run() -> MResult<()> {
    let args = Args::parse()?;
    let stopped = Arc::new(AtomicBool::new(false));
    let stop_flag = stopped.clone();
    ctrlc::set_handler(move || {
      stop_flag.store(true, Ordering::SeqCst);
    }).map_err(|err| clock_error("ClockCli", format!("failed to install Ctrl-C handler: {err}")))?;

    let mut runtime = build_runtime(args.interval_ms)?;
    runtime.run_string(CLOCK_SOURCE)?;
    runtime.start_input_drivers()?;
    let result = pump(&mut runtime, args.ticks, &stopped);
    let stop_result = runtime.stop_input_drivers();
    let close_result = runtime.close_ingress();
    result?;
    stop_result?;
    close_result?;
    Ok(())
  }

  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub struct Args {
    pub interval_ms: u64,
    pub ticks: Option<usize>,
  }

  impl Args {
    pub fn parse() -> MResult<Self> {
      let matches = Command::new("mech-clock")
        .arg(Arg::new("interval-ms").long("interval-ms").num_args(1).value_parser(clap::value_parser!(u64)).default_value("100"))
        .arg(Arg::new("ticks").long("ticks").num_args(1).value_parser(clap::value_parser!(usize)))
        .try_get_matches()
        .map_err(|err| clock_error("ClockCliArgs", err.to_string()))?;
      let interval_ms = *matches.get_one::<u64>("interval-ms").unwrap_or(&100);
      if interval_ms == 0 { return Err(clock_error("ClockCliArgs", "--interval-ms must be greater than zero")); }
      let ticks = matches.get_one::<usize>("ticks").copied();
      if ticks == Some(0) { return Err(clock_error("ClockCliArgs", "--ticks must be greater than zero")); }
      Ok(Self { interval_ms, ticks })
    }
  }

  fn build_runtime(interval_ms: u64) -> MResult<MechRuntime> {
    let mut settings = BTreeMap::new();
    settings.insert("interval-ms".to_string(), ConfigValue::Integer(interval_ms as i64));
    let mut runtime = RuntimeBuilder::new()
      .host_factory(Box::new(NativeTimeHostFactory::new()?))?
      .host_instance(HostInstanceConfig { name: "clock".to_string(), provider: "time".to_string(), settings: ConfigValue::Map(settings) })
      .run_resource_grant(RunResourceGrantConfig {
        target: "clock/clock".to_string(),
        operations: vec!["read".to_string()],
        paths: vec!["unix-ms".to_string(), "hour".to_string(), "minute".to_string(), "second".to_string(), "millisecond".to_string()],
      })
      .build()?;
    grant_time_reads(&mut runtime)?;
    Ok(runtime)
  }

  fn grant_time_reads(runtime: &mut MechRuntime) -> MResult<()> {
    let subject = runtime.runtime_context()?.subject;
    runtime.grant_capability(RuntimeCapabilityGrant {
      subject,
      resource: "time://clock/clock".to_string(),
      operations: vec![RuntimeCapabilityOperation::Read],
      paths: vec!["unix-ms".to_string(), "hour".to_string(), "minute".to_string(), "second".to_string(), "millisecond".to_string()],
    })?;
    Ok(())
  }

  fn pump(runtime: &mut MechRuntime, ticks: Option<usize>, stopped: &AtomicBool) -> MResult<()> {
    let mut printed = 0usize;
    let mut last_displayed = None;
    while !stopped.load(Ordering::SeqCst) {
      if runtime.pending_host_input_count()? == 0 {
        thread::sleep(Duration::from_millis(10));
        continue;
      }
      runtime.drain_host_inputs(64)?;
      let rows = runtime.root_symbol_values(&["clock-hour", "clock-minute", "clock-second"])?;
      let line = format_clock_line(f64_from_value(&rows[0].1)?, f64_from_value(&rows[1].1)?, f64_from_value(&rows[2].1)?);
      if last_displayed.as_deref() != Some(line.as_str()) {
        println!("{line}");
        last_displayed = Some(line);
        printed += 1;
        if ticks.is_some_and(|limit| printed >= limit) { break; }
      }
    }
    Ok(())
  }

  pub fn format_clock_line(hour: f64, minute: f64, second: f64) -> String {
    format!("{:02}:{:02}:{:02}", hour.floor() as u64, minute.floor() as u64, second.floor() as u64)
  }

  fn f64_from_value(value: &Value) -> MResult<f64> {
    match value {
      Value::F64(value) => Ok(*value.borrow()),
      other => Err(clock_error("ClockCli", format!("expected f64 clock symbol, got {other:?}"))),
    }
  }

  #[derive(Debug, Clone)]
  struct ClockCliError { name: &'static str, message: String }
  impl MechErrorKind for ClockCliError {
    fn name(&self) -> &str { self.name }
    fn message(&self) -> String { self.message.clone() }
  }
  fn clock_error(name: &'static str, message: impl Into<String>) -> MechError {
    MechError::new(ClockCliError { name, message: message.into() }, None)
  }

  #[cfg(test)]
  mod tests {
    use super::*;
    use mech_host_time::{new_shared_snapshot, ManualTimeInputDriver, TimeResourceProvider, TimeSnapshot};
    use mech_runtime::{RuntimeHostInputDriver, RuntimeResourceProvider};

    const EPS: f64 = 1e-6;

    fn manual_runtime(initial: TimeSnapshot) -> (MechRuntime, ManualTimeInputDriver) {
      let shared = new_shared_snapshot(initial);
      let mut runtime = RuntimeBuilder::new()
        .resource_provider(Box::new(TimeResourceProvider::new("clock", shared.clone())) as Box<dyn RuntimeResourceProvider>)
        .build()
        .unwrap();
      grant_time_reads(&mut runtime).unwrap();
      runtime.run_string(CLOCK_SOURCE).unwrap();
      let mut driver = ManualTimeInputDriver::new("clock", shared);
      driver.attach(runtime.ingress()).unwrap();
      driver.start().unwrap();
      (runtime, driver)
    }

    fn read_outputs(runtime: &MechRuntime) -> Vec<f64> {
      runtime.root_symbol_values(&[
        "clock-second",
        "clock-minute",
        "clock-hour",
        "clock-second-angle",
        "clock-minute-angle",
        "clock-hour-angle",
      ]).unwrap().into_iter().map(|(_, value)| f64_from_value(&value).unwrap()).collect()
    }

    fn publish(runtime: &mut MechRuntime, driver: &ManualTimeInputDriver, snapshot: TimeSnapshot) {
      driver.publish(snapshot).unwrap();
      runtime.drain_host_inputs(1).unwrap();
    }

    fn assert_close(actual: f64, expected: f64) { assert!((actual - expected).abs() < EPS, "{actual} != {expected}"); }

    #[test]
    fn clock_model_031530500_angles() {
      let initial = TimeSnapshot { hour: 3.0, minute: 15.0, second: 30.0, millisecond: 500.0, unix_ms: 0.0 };
      let (mut runtime, driver) = manual_runtime(initial);
      publish(&mut runtime, &driver, initial);
      let values = read_outputs(&runtime);
      assert_close(values[0], 30.5);
      assert_close(values[1], 15.5083333333);
      assert_close(values[2], 3.2584722222);
      assert_close(values[3], 183.0);
      assert_close(values[4], 93.05);
      assert_close(values[5], 97.7541666667);
    }

    #[test]
    fn clock_model_120000000_angles() {
      let s = TimeSnapshot { hour: 12.0, minute: 0.0, second: 0.0, millisecond: 0.0, unix_ms: 0.0 };
      let (mut runtime, driver) = manual_runtime(s);
      publish(&mut runtime, &driver, s);
      let values = read_outputs(&runtime);
      assert_close(values[2], 12.0);
      assert_close(values[5], 360.0);
    }

    #[test]
    fn clock_model_235959000_angles() {
      let s = TimeSnapshot { hour: 23.0, minute: 59.0, second: 59.0, millisecond: 0.0, unix_ms: 0.0 };
      let (mut runtime, driver) = manual_runtime(s);
      publish(&mut runtime, &driver, s);
      let values = read_outputs(&runtime);
      assert_close(values[0], 59.0);
      assert_close(values[3], 354.0);
    }

    #[test]
    fn clock_snapshot_updates_all_values_before_solve() {
      let initial = TimeSnapshot { hour: 1.0, minute: 0.0, second: 0.0, millisecond: 0.0, unix_ms: 0.0 };
      let next = TimeSnapshot { hour: 2.0, minute: 30.0, second: 0.0, millisecond: 0.0, unix_ms: 0.0 };
      let (mut runtime, driver) = manual_runtime(initial);
      publish(&mut runtime, &driver, next);
      let values = read_outputs(&runtime);
      assert_close(values[1], 30.0);
      assert_close(values[2], 2.5);
    }

    #[test]
    fn clock_cli_format_zero_pads() { assert_eq!(format_clock_line(3.0, 4.0, 5.0), "03:04:05"); }

    #[test]
    fn clock_cli_suppresses_same_displayed_second() {
      let a = format_clock_line(3.0, 4.0, 5.1);
      let b = format_clock_line(3.0, 4.0, 5.9);
      assert_eq!(a, b);
    }

    #[test]
    fn clock_cli_prints_successive_seconds() {
      assert_ne!(format_clock_line(3.0, 4.0, 5.9), format_clock_line(3.0, 4.0, 6.0));
    }
  }
}
