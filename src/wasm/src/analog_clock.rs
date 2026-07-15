use wasm_bindgen::prelude::*;
use web_sys::Element;

use mech_core::{MechError, MechErrorKind, Value};
use mech_host_console::BrowserConsoleHostFactory;
use mech_host_time::BrowserTimeHostFactory;
use mech_runtime::{ConfigProfileOptions, MechConfigDocument, MechRuntime, RuntimeBuilder};

const CLOCK_CONFIG_PATH: &str = "examples/analog-clock/mech.mcfg";
const CLOCK_CONFIG: &str = include_str!("../../../examples/analog-clock/mech.mcfg");
const CLOCK_PROJECT_SOURCES: &[(&str, &str)] = &[(
    "clock.mec",
    include_str!("../../../examples/analog-clock/clock.mec"),
)];

#[wasm_bindgen]
pub struct WasmAnalogClock {
    runtime: MechRuntime,
    hour_hand: Element,
    minute_hand: Element,
    second_hand: Element,
    center_x: f64,
    center_y: f64,
    started: bool,
}

#[wasm_bindgen]
impl WasmAnalogClock {
    #[wasm_bindgen(constructor)]
    pub fn new(
        hour_selector: &str,
        minute_selector: &str,
        second_selector: &str,
        center_x: f64,
        center_y: f64,
    ) -> Result<WasmAnalogClock, JsValue> {
        if hour_selector.trim().is_empty()
            || minute_selector.trim().is_empty()
            || second_selector.trim().is_empty()
        {
            return Err(js_error("clock hand selectors must be non-empty"));
        }
        let window = web_sys::window().ok_or_else(|| js_error("Window is unavailable"))?;
        let document = window
            .document()
            .ok_or_else(|| js_error("Document is unavailable"))?;
        let hour_hand = query_required(&document, hour_selector)?;
        let minute_hand = query_required(&document, minute_selector)?;
        let second_hand = query_required(&document, second_selector)?;
        let document = shared_clock_config().map_err(to_js_error)?;
        let mut runtime = build_runtime(&document)?;
        run_shared_clock_project(&mut runtime, &document).map_err(to_js_error)?;
        Self::from_parts(
            runtime,
            hour_hand,
            minute_hand,
            second_hand,
            center_x,
            center_y,
        )
    }

    pub fn start(&mut self) -> Result<(), JsValue> {
        if self.started {
            return Ok(());
        }
        self.runtime.start_input_drivers().map_err(to_js_error)?;
        self.started = true;
        Ok(())
    }

    #[wasm_bindgen(js_name = "pumpAndRender")]
    pub fn pump_and_render(&mut self) -> Result<u32, JsValue> {
        let pending = self
            .runtime
            .pending_host_input_count()
            .map_err(to_js_error)?;
        if pending == 0 {
            return Ok(0);
        }
        let outcomes = self
            .runtime
            .drain_host_inputs(pending)
            .map_err(to_js_error)?;
        self.render()?;
        Ok(outcomes.len() as u32)
    }

    pub fn stop(&mut self) -> Result<(), JsValue> {
        self.runtime.stop_input_drivers().map_err(to_js_error)?;
        self.started = false;
        Ok(())
    }
}

impl WasmAnalogClock {
    fn from_parts(
        runtime: MechRuntime,
        hour_hand: Element,
        minute_hand: Element,
        second_hand: Element,
        center_x: f64,
        center_y: f64,
    ) -> Result<Self, JsValue> {
        let mut clock = Self {
            runtime,
            hour_hand,
            minute_hand,
            second_hand,
            center_x,
            center_y,
            started: false,
        };
        clock.render()?;
        Ok(clock)
    }

    fn render(&mut self) -> Result<(), JsValue> {
        let rows = self
            .runtime
            .root_symbol_values(&[
                "clock-hour-angle",
                "clock-minute-angle",
                "clock-second-angle",
            ])
            .map_err(to_js_error)?;
        let hour = f64_from_value(&rows[0].1)?;
        let minute = f64_from_value(&rows[1].1)?;
        let second = f64_from_value(&rows[2].1)?;
        self.hour_hand
            .set_attribute(
                "transform",
                &svg_rotation(hour, self.center_x, self.center_y)?,
            )
            .map_err(|err| js_error(format!("failed to set hour transform: {err:?}")))?;
        self.minute_hand
            .set_attribute(
                "transform",
                &svg_rotation(minute, self.center_x, self.center_y)?,
            )
            .map_err(|err| js_error(format!("failed to set minute transform: {err:?}")))?;
        self.second_hand
            .set_attribute(
                "transform",
                &svg_rotation(second, self.center_x, self.center_y)?,
            )
            .map_err(|err| js_error(format!("failed to set second transform: {err:?}")))?;
        Ok(())
    }
}

fn build_runtime(document: &MechConfigDocument) -> Result<MechRuntime, JsValue> {
    let mut builder = RuntimeBuilder::new()
        .host_factory(Box::new(
            BrowserTimeHostFactory::new().map_err(to_js_error)?,
        ))
        .map_err(to_js_error)?
        .host_factory(Box::new(
            BrowserConsoleHostFactory::new().map_err(to_js_error)?,
        ))
        .map_err(to_js_error)?;

    for host in &document.hosts {
        builder = builder.host_instance(host.clone());
    }
    if let Some(run) = &document.run {
        for grant in &run.grants {
            builder = builder.run_resource_grant(grant.clone());
        }
    }
    builder.build().map_err(to_js_error)
}

fn embedded_project_source(path: &str) -> mech_core::MResult<&'static str> {
    CLOCK_PROJECT_SOURCES
        .iter()
        .find_map(|(candidate, source)| (*candidate == path).then_some(*source))
        .ok_or_else(|| {
            MechError::new(
                AnalogClockProjectError {
                    reason: format!("analog clock project source `{path}` is not embedded"),
                },
                None,
            )
        })
}

fn run_shared_clock_project(
    runtime: &mut MechRuntime,
    document: &MechConfigDocument,
) -> mech_core::MResult<()> {
    let run = document.run.as_ref().ok_or_else(|| {
        MechError::new(
            AnalogClockProjectError {
                reason: "analog clock config must contain run settings".to_string(),
            },
            None,
        )
    })?;
    if run.paths.is_empty() {
        return Err(MechError::new(
            AnalogClockProjectError {
                reason: "analog clock config must contain at least one run path".to_string(),
            },
            None,
        ));
    }
    for path in &run.paths {
        let path = path.to_string_lossy();
        let source = embedded_project_source(path.as_ref())?;
        runtime.run_string(source)?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct AnalogClockProjectError {
    reason: String,
}

impl MechErrorKind for AnalogClockProjectError {
    fn name(&self) -> &str {
        "AnalogClockProjectError"
    }
    fn message(&self) -> String {
        self.reason.clone()
    }
}

fn shared_clock_config() -> mech_core::MResult<MechConfigDocument> {
    mech_runtime::parse_config_document(
        CLOCK_CONFIG_PATH,
        CLOCK_CONFIG,
        ConfigProfileOptions::default(),
    )
}

fn query_required(document: &web_sys::Document, selector: &str) -> Result<Element, JsValue> {
    document
        .query_selector(selector)
        .map_err(|err| js_error(format!("selector lookup failed for `{selector}`: {err:?}")))?
        .ok_or_else(|| js_error(format!("selector `{selector}` did not match an element")))
}

pub(crate) fn svg_rotation(angle: f64, center_x: f64, center_y: f64) -> Result<String, JsValue> {
    if !angle.is_finite() || !center_x.is_finite() || !center_y.is_finite() {
        return Err(js_error("SVG rotation values must be finite"));
    }
    Ok(format!(
        "rotate({} {} {})",
        compact_f64(angle),
        compact_f64(center_x),
        compact_f64(center_y)
    ))
}

fn compact_f64(value: f64) -> String {
    let text = format!("{value:.12}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn f64_from_value(value: &Value) -> Result<f64, JsValue> {
    match value {
        Value::F64(value) => Ok(*value.borrow()),
        other => Err(js_error(format!(
            "expected f64 clock symbol, got {other:?}"
        ))),
    }
}

fn js_error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}
fn to_js_error(error: MechError) -> JsValue {
    js_error(format!("{error:?}"))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn shared_clock_run_paths_resolve_embedded_sources() {
        let config = shared_clock_config().unwrap();
        let run = config.run.as_ref().unwrap();
        assert_eq!(run.paths.len(), 1);
        assert_eq!(run.paths[0].to_string_lossy(), "clock.mec");
        assert!(
            embedded_project_source(run.paths[0].to_string_lossy().as_ref())
                .unwrap()
                .contains("clock-output")
        );
    }

    #[test]
    fn shared_clock_project_rejects_unknown_embedded_path() {
        let err = embedded_project_source("missing.mec").unwrap_err();
        assert!(format!("{err:?}").contains("missing.mec"));
    }

    #[test]
    fn shared_clock_project_requires_run_paths() {
        let mut config = shared_clock_config().unwrap();
        config.run.as_mut().unwrap().paths.clear();
        let mut runtime = RuntimeBuilder::new().build().unwrap();
        let err = run_shared_clock_project(&mut runtime, &config).unwrap_err();
        assert!(format!("{err:?}").contains("at least one run path"));
    }

    #[test]
    fn svg_rotation_formats_values() {
        assert_eq!(
            svg_rotation(97.5, 100.0, 100.0).unwrap(),
            "rotate(97.5 100 100)"
        );
    }

    #[test]
    fn svg_rotation_rejects_nan() {
        assert!(svg_rotation(f64::NAN, 100.0, 100.0).is_err());
    }

    #[test]
    fn svg_rotation_rejects_infinity() {
        assert!(svg_rotation(f64::INFINITY, 100.0, 100.0).is_err());
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod browser_tests {
    use super::*;
    use mech_host_console::{
        BrowserConsoleHostFactory, ConsoleHostFactory, RecordingConsoleBackend,
    };
    use mech_host_time::{
        new_shared_snapshot, ManualTimeInputDriver, TimeResourceProvider, TimeSnapshot,
    };
    use mech_runtime::{
        ConfigValue, HostInstanceConfig, RuntimeCapabilityGrant, RuntimeCapabilityOperation,
        RuntimeHostFactory, RuntimeHostInputDriver, RuntimeResourceProvider,
    };
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    const SVG_NS: &str = "http://www.w3.org/2000/svg";

    struct SvgFixture {
        root: Element,
        hour: Element,
        minute: Element,
        second: Element,
        hour_selector: String,
        minute_selector: String,
        second_selector: String,
    }

    impl SvgFixture {
        fn new(id: &str, include_hour: bool, include_minute: bool, include_second: bool) -> Self {
            let document = document();
            let svg = document.create_element_ns(Some(SVG_NS), "svg").unwrap();
            svg.set_attribute("id", &format!("{id}-svg")).unwrap();
            let hour = create_line(&document, &format!("{id}-hour"));
            let minute = create_line(&document, &format!("{id}-minute"));
            let second = create_line(&document, &format!("{id}-second"));
            if include_hour {
                svg.append_child(&hour).unwrap();
            }
            if include_minute {
                svg.append_child(&minute).unwrap();
            }
            if include_second {
                svg.append_child(&second).unwrap();
            }
            document.body().unwrap().append_child(&svg).unwrap();
            Self {
                root: svg,
                hour,
                minute,
                second,
                hour_selector: format!("#{id}-hour"),
                minute_selector: format!("#{id}-minute"),
                second_selector: format!("#{id}-second"),
            }
        }
    }

    impl Drop for SvgFixture {
        fn drop(&mut self) {
            self.root.remove();
        }
    }

    fn document() -> web_sys::Document {
        web_sys::window().unwrap().document().unwrap()
    }

    fn create_line(document: &web_sys::Document, id: &str) -> Element {
        let line = document.create_element_ns(Some(SVG_NS), "line").unwrap();
        line.set_attribute("id", id).unwrap();
        line
    }

    fn deterministic_runtime() -> (MechRuntime, ManualTimeInputDriver) {
        let (runtime, driver, _console) =
            deterministic_runtime_with_console(TimeSnapshot::default());
        (runtime, driver)
    }

    fn deterministic_runtime_with_snapshot(
        initial: TimeSnapshot,
    ) -> (MechRuntime, ManualTimeInputDriver) {
        let (runtime, driver, _console) = deterministic_runtime_with_console(initial);
        (runtime, driver)
    }

    fn deterministic_runtime_with_console(
        initial: TimeSnapshot,
    ) -> (MechRuntime, ManualTimeInputDriver, RecordingConsoleBackend) {
        let snapshot = new_shared_snapshot(initial);
        let console = RecordingConsoleBackend::new();
        let console_factory = ConsoleHostFactory::with_backend(console.clone()).unwrap();
        let mut runtime = RuntimeBuilder::new()
            .host_factory(Box::new(console_factory))
            .unwrap()
            .host_instance(HostInstanceConfig {
                name: "console".to_string(),
                provider: "console".to_string(),
                settings: ConfigValue::Map(Default::default()),
            })
            .resource_provider(
                Box::new(TimeResourceProvider::new("clock", snapshot.clone()))
                    as Box<dyn RuntimeResourceProvider>,
            )
            .build()
            .unwrap();
        let subject = runtime.runtime_context().unwrap().subject;
        runtime
            .grant_capability(RuntimeCapabilityGrant {
                subject: subject.clone(),
                resource: "time://clock/clock".to_string(),
                operations: vec![RuntimeCapabilityOperation::Read],
                paths: vec![
                    "unix-ms".to_string(),
                    "hour".to_string(),
                    "minute".to_string(),
                    "second".to_string(),
                    "millisecond".to_string(),
                ],
            })
            .unwrap();
        runtime
            .grant_capability(RuntimeCapabilityGrant {
                subject,
                resource: "console://console/output".to_string(),
                operations: vec![RuntimeCapabilityOperation::Write],
                paths: vec!["line".to_string()],
            })
            .unwrap();
        let config = shared_clock_config().unwrap();
        run_shared_clock_project(&mut runtime, &config).unwrap_or_else(|error| {
            panic!("failed to load shared analog clock project: {error:?}")
        });
        let mut driver = ManualTimeInputDriver::new("clock", snapshot);
        driver.attach(runtime.ingress()).unwrap();
        driver.start().unwrap();
        (runtime, driver, console)
    }

    fn js_message(value: JsValue) -> String {
        value.as_string().unwrap_or_else(|| format!("{value:?}"))
    }

    fn assert_missing_selector(result: Result<WasmAnalogClock, JsValue>, selector: &str) {
        let message = js_message(result.err().expect("constructor should fail"));
        assert!(
            message.contains(selector),
            "expected `{selector}` in `{message}`"
        );
    }

    fn transform_angle(element: &Element) -> f64 {
        let transform = element
            .get_attribute("transform")
            .expect("transform attribute");
        assert!(
            transform.starts_with("rotate("),
            "unexpected transform `{transform}`"
        );
        transform[7..]
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<f64>()
            .unwrap()
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.000001,
            "expected {expected}, got {actual}"
        );
    }

    #[wasm_bindgen_test]
    fn browser_console_factory_installs_configured_console_host() {
        let factory = BrowserConsoleHostFactory::new().unwrap();
        let installation = factory
            .instantiate("console", &ConfigValue::Map(Default::default()))
            .unwrap();
        assert_eq!(installation.interface.provider, "console");
        assert_eq!(installation.interface.instance, "console");
        assert_eq!(installation.resource_providers.len(), 1);
    }

    #[wasm_bindgen_test]
    fn shared_config_clock_load_and_time_packet_write_console() {
        let config = shared_clock_config().unwrap();
        assert!(config.hosts.iter().any(|host| host.provider == "time"));
        assert!(config.hosts.iter().any(|host| host.provider == "console"));
        let run_paths = &config.run.as_ref().unwrap().paths;
        assert_eq!(run_paths.len(), 1);
        assert_eq!(run_paths[0].to_string_lossy(), "clock.mec");

        let initial = TimeSnapshot {
            unix_ms: 0.0,
            hour: 1.0,
            minute: 2.0,
            second: 3.0,
            millisecond: 4.0,
        };
        let (mut runtime, driver, console) = deterministic_runtime_with_console(initial);
        let initial_lines = console.lines();
        assert_eq!(initial_lines.len(), 1);
        driver
            .publish(TimeSnapshot {
                unix_ms: 1.0,
                hour: 5.0,
                minute: 6.0,
                second: 7.0,
                millisecond: 8.0,
            })
            .unwrap();
        runtime.drain_host_inputs(1).unwrap();
        let lines = console.lines();
        assert_eq!(lines.len(), 2);
        assert_ne!(lines[0], lines[1]);
    }

    #[wasm_bindgen_test]
    fn runtime_turn_timing_works_in_browser() {
        let mut runtime = RuntimeBuilder::new().build().unwrap();

        runtime
            .run_string("browser-time-test := 1.0 + 2.0")
            .unwrap();

        let value = runtime.root_symbol_value("browser-time-test").unwrap();

        match value {
            Value::F64(value) => {
                assert_eq!(*value.borrow(), 3.0);
            }
            other => {
                panic!("expected f64 result, got {other:?}");
            }
        }
    }

    #[wasm_bindgen_test]
    fn profiled_runtime_execution_works_in_browser() {
        let mut config = mech_runtime::RuntimeConfig::default();
        config.diagnostics.profile_enabled = true;

        let mut runtime = RuntimeBuilder::new().config(config).build().unwrap();

        runtime
            .run_string("profiled-browser-time-test := 2.0 * 3.0")
            .unwrap();

        let value = runtime
            .root_symbol_value("profiled-browser-time-test")
            .unwrap();

        match value {
            Value::F64(value) => {
                assert_eq!(*value.borrow(), 6.0);
            }
            other => {
                panic!("expected f64 result, got {other:?}");
            }
        }
    }

    #[wasm_bindgen_test]
    fn analog_clock_feature_set_loads_shared_model() {
        let (runtime, _driver) = deterministic_runtime_with_snapshot(TimeSnapshot {
            unix_ms: 0.0,
            hour: 3.0,
            minute: 15.0,
            second: 30.0,
            millisecond: 500.0,
        });

        let values = runtime
            .root_symbol_values(&[
                "clock-hour-angle",
                "clock-minute-angle",
                "clock-second-angle",
            ])
            .unwrap();

        assert_eq!(values.len(), 3);
    }

    #[wasm_bindgen_test]
    fn analog_clock_feature_set_supports_variable_reads() {
        let mut runtime = RuntimeBuilder::new().build().unwrap();
        runtime
            .run_string(
                "feature-input := 1.0\n\
                 feature-output := feature-input + 2.0",
            )
            .unwrap();
        let value = runtime.root_symbol_value("feature-output").unwrap();
        match value {
            Value::F64(value) => {
                assert_eq!(*value.borrow(), 3.0);
            }
            other => {
                panic!("expected f64 result, got {other:?}");
            }
        }
    }

    #[wasm_bindgen_test]
    fn constructor_rejects_missing_hour_hand() {
        let fixture = SvgFixture::new("missing-hour", false, true, true);
        let result = WasmAnalogClock::new(
            &fixture.hour_selector,
            &fixture.minute_selector,
            &fixture.second_selector,
            100.0,
            100.0,
        );
        assert_missing_selector(result, &fixture.hour_selector);
    }

    #[wasm_bindgen_test]
    fn constructor_rejects_missing_minute_hand() {
        let fixture = SvgFixture::new("missing-minute", true, false, true);
        let result = WasmAnalogClock::new(
            &fixture.hour_selector,
            &fixture.minute_selector,
            &fixture.second_selector,
            100.0,
            100.0,
        );
        assert_missing_selector(result, &fixture.minute_selector);
    }

    #[wasm_bindgen_test]
    fn constructor_rejects_missing_second_hand() {
        let fixture = SvgFixture::new("missing-second", true, true, false);
        let result = WasmAnalogClock::new(
            &fixture.hour_selector,
            &fixture.minute_selector,
            &fixture.second_selector,
            100.0,
            100.0,
        );
        assert_missing_selector(result, &fixture.second_selector);
    }

    #[wasm_bindgen_test]
    fn pump_with_no_packets_returns_zero() {
        let fixture = SvgFixture::new("no-packets", true, true, true);
        let (runtime, _driver) = deterministic_runtime();
        let mut clock = WasmAnalogClock::from_parts(
            runtime,
            fixture.hour.clone(),
            fixture.minute.clone(),
            fixture.second.clone(),
            100.0,
            100.0,
        )
        .unwrap();
        assert_eq!(clock.pump_and_render().unwrap(), 0);
    }

    #[wasm_bindgen_test]
    fn manual_snapshot_updates_all_three_hand_transforms() {
        let fixture = SvgFixture::new("manual-update", true, true, true);
        let (runtime, driver) = deterministic_runtime();
        let mut clock = WasmAnalogClock::from_parts(
            runtime,
            fixture.hour.clone(),
            fixture.minute.clone(),
            fixture.second.clone(),
            100.0,
            100.0,
        )
        .unwrap();
        driver
            .publish(TimeSnapshot {
                unix_ms: 0.0,
                hour: 3.0,
                minute: 15.0,
                second: 30.0,
                millisecond: 500.0,
            })
            .unwrap();
        assert_eq!(clock.runtime.pending_host_input_count().unwrap(), 1);
        assert_eq!(clock.pump_and_render().unwrap(), 1);
        assert_close(transform_angle(&fixture.hour), 97.7541666667);
        assert_close(transform_angle(&fixture.minute), 93.05);
        assert_close(transform_angle(&fixture.second), 183.0);
    }

    #[wasm_bindgen_test]
    fn start_is_idempotent() {
        let fixture = SvgFixture::new("start-idempotent", true, true, true);
        let mut clock = WasmAnalogClock::new(
            &fixture.hour_selector,
            &fixture.minute_selector,
            &fixture.second_selector,
            100.0,
            100.0,
        )
        .unwrap();
        clock.start().unwrap();
        clock.start().unwrap();
        clock.stop().unwrap();
    }

    #[wasm_bindgen_test]
    fn stop_is_idempotent() {
        let fixture = SvgFixture::new("stop-idempotent", true, true, true);
        let mut clock = WasmAnalogClock::new(
            &fixture.hour_selector,
            &fixture.minute_selector,
            &fixture.second_selector,
            100.0,
            100.0,
        )
        .unwrap();
        clock.start().unwrap();
        clock.stop().unwrap();
        clock.stop().unwrap();
    }
}
