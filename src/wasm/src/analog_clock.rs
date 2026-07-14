use std::collections::BTreeMap;

use wasm_bindgen::prelude::*;
use web_sys::Element;

use mech_core::{MechError, Value};
use mech_host_browser::BrowserHostFactory;
use mech_host_time::BrowserTimeHostFactory;
use mech_runtime::{
    ConfigValue, HostInstanceConfig, MechRuntime, RunResourceGrantConfig, RuntimeBuilder,
    RuntimeCapabilityGrant, RuntimeCapabilityOperation,
};

use crate::host::WasmBrowserDomBackend;

const CLOCK_SOURCE: &str = include_str!("../../../examples/analog-clock/clock.mec");

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
        interval_ms: u32,
    ) -> Result<WasmAnalogClock, JsValue> {
        if hour_selector.trim().is_empty()
            || minute_selector.trim().is_empty()
            || second_selector.trim().is_empty()
        {
            return Err(js_error("clock hand selectors must be non-empty"));
        }
        if interval_ms == 0 {
            return Err(js_error("interval_ms must be greater than zero"));
        }
        let window = web_sys::window().ok_or_else(|| js_error("Window is unavailable"))?;
        let document = window
            .document()
            .ok_or_else(|| js_error("Document is unavailable"))?;
        let hour_hand = query_required(&document, hour_selector)?;
        let minute_hand = query_required(&document, minute_selector)?;
        let second_hand = query_required(&document, second_selector)?;
        let mut runtime = build_runtime(interval_ms)?;
        runtime.run_string(CLOCK_SOURCE).map_err(to_js_error)?;
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

fn build_runtime(interval_ms: u32) -> Result<MechRuntime, JsValue> {
    let mut settings = BTreeMap::new();
    settings.insert(
        "interval-ms".to_string(),
        ConfigValue::Integer(interval_ms as i64),
    );
    let mut runtime = RuntimeBuilder::new()
        .host_factory(Box::new(
            BrowserHostFactory::new(WasmBrowserDomBackend::new()).map_err(to_js_error)?,
        ))
        .map_err(to_js_error)?
        .host_factory(Box::new(
            BrowserTimeHostFactory::new().map_err(to_js_error)?,
        ))
        .map_err(to_js_error)?
        .host_instance(HostInstanceConfig {
            name: "browser".to_string(),
            provider: "browser".to_string(),
            settings: ConfigValue::Map(Default::default()),
        })
        .host_instance(HostInstanceConfig {
            name: "clock".to_string(),
            provider: "time".to_string(),
            settings: ConfigValue::Map(settings),
        })
        .run_resource_grant(RunResourceGrantConfig {
            target: "clock/clock".to_string(),
            operations: vec!["read".to_string()],
            paths: vec![
                "unix-ms".to_string(),
                "hour".to_string(),
                "minute".to_string(),
                "second".to_string(),
                "millisecond".to_string(),
            ],
        })
        .build()
        .map_err(to_js_error)?;
    let subject = runtime.runtime_context().map_err(to_js_error)?.subject;
    runtime
        .grant_capability(RuntimeCapabilityGrant {
            subject,
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
        .map_err(to_js_error)?;
    Ok(runtime)
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
    use mech_host_time::{
        new_shared_snapshot, ManualTimeInputDriver, TimeResourceProvider, TimeSnapshot,
    };
    use mech_runtime::{RuntimeHostInputDriver, RuntimeResourceProvider};
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
        deterministic_runtime_with_snapshot(TimeSnapshot::default())
    }

    fn deterministic_runtime_with_snapshot(
        initial: TimeSnapshot,
    ) -> (MechRuntime, ManualTimeInputDriver) {
        let snapshot = new_shared_snapshot(initial);
        let mut runtime = RuntimeBuilder::new()
            .resource_provider(
                Box::new(TimeResourceProvider::new("clock", snapshot.clone()))
                    as Box<dyn RuntimeResourceProvider>,
            )
            .build()
            .unwrap();
        let subject = runtime.runtime_context().unwrap().subject;
        runtime
            .grant_capability(RuntimeCapabilityGrant {
                subject,
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
        runtime.run_string(CLOCK_SOURCE).unwrap_or_else(|error| {
            panic!("failed to load shared analog clock model: {error:?}")
        });
        let mut driver = ManualTimeInputDriver::new("clock", snapshot);
        driver.attach(runtime.ingress()).unwrap();
        driver.start().unwrap();
        (runtime, driver)
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
            100,
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
            100,
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
            100,
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
            100,
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
            100,
        )
        .unwrap();
        clock.start().unwrap();
        clock.stop().unwrap();
        clock.stop().unwrap();
    }
}
