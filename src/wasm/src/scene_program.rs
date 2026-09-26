//! A resident Mech program with numeric browser inputs and scene-table output.
//!
//! The browser supplies measurements and renders the scene host's snapshot;
//! source code computes geometry, sensor models, and persistent application state.

use std::collections::BTreeMap;
use std::sync::Arc;

use js_sys::{Array, Float32Array, Float64Array, Object, Reflect};
use mech_core::{MResult, MechError, MechErrorKind, Value};
use mech_runtime::{
    ConfigValue, HostContextManifest, HostInstanceConfig, HostManifestConfig, MechRuntime,
    ResidentDurabilityPolicy, RunResourceGrantConfig, RuntimeBuilder, RuntimeHostFactory,
    RuntimeHostInput, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputUpdate,
    RuntimeHostInputValue, RuntimeHostInstallation, RuntimeIngress, RuntimeResourceProvider,
    RuntimeResourceReadRequest, materialize_host_manifest,
};
use mech_scene::{RecordingSceneBackend, SceneHostFactory, SceneSnapshot};
use wasm_bindgen::prelude::*;

const INPUT_BASE: &str = "input://browser/frame";
type Packet = BTreeMap<String, RuntimeHostInputValue>;

struct SceneProgramCore {
    runtime: MechRuntime,
    declarations: Arc<Packet>,
    backend: RecordingSceneBackend,
    stopped: bool,
}

impl SceneProgramCore {
    fn from_source(source: &str, declarations: Packet) -> MResult<Self> {
        if declarations.is_empty() {
            return Err(scene_error("declare at least one numeric browser input"));
        }
        for (name, value) in &declarations {
            RuntimeHostInputSource::new(INPUT_BASE, name)?;
            numeric_shape(value)?;
            value.clone().into_value()?;
        }
        let declarations = Arc::new(declarations);
        let backend = RecordingSceneBackend::new();
        let mut runtime = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .host_input_capacity(1)
            .host_factory(Box::new(NumericInputFactory::new(declarations.clone())))?
            .host_factory(Box::new(SceneHostFactory::with_backend(backend.clone())?))?
            .host_instance(HostInstanceConfig {
                name: "browser".into(),
                provider: "input".into(),
                settings: ConfigValue::Map(BTreeMap::new()),
            })
            .host_instance(HostInstanceConfig {
                name: "view".into(),
                provider: "scene".into(),
                settings: ConfigValue::Map(BTreeMap::from([(
                    "renderer".into(),
                    ConfigValue::String("output".into()),
                )])),
            })
            .run_resource_grant(RunResourceGrantConfig {
                target: "browser/frame".into(),
                operations: vec!["read".into()],
                paths: declarations.keys().cloned().collect(),
            })
            .run_resource_grant(RunResourceGrantConfig {
                target: "view/frame".into(),
                operations: vec!["write".into()],
                paths: vec!["replace".into()],
            })
            .build()?;
        runtime.load_interactive_source_program(source, ResidentDurabilityPolicy::Volatile)?;
        if !runtime.has_driven_live_input_bindings()? {
            return Err(scene_error(
                "the source must read at least one declared browser input",
            ));
        }
        runtime.start_input_drivers()?;
        Ok(Self {
            runtime,
            declarations,
            backend,
            stopped: false,
        })
    }

    fn turn(&mut self, updates: Packet) -> MResult<()> {
        if self.stopped {
            return Err(scene_error("the scene program has been stopped"));
        }
        if updates.is_empty() {
            return Err(scene_error(
                "a turn requires at least one numeric input update",
            ));
        }
        // Validate the complete packet before publishing any individual input.
        let mut packet = Vec::with_capacity(updates.len());
        for (name, value) in updates {
            let declaration = self
                .declarations
                .get(&name)
                .ok_or_else(|| scene_error(format!("undeclared browser input `{name}`")))?;
            if numeric_shape(&value)? != numeric_shape(declaration)? {
                return Err(scene_error(format!(
                    "input `{name}` changed its declared shape"
                )));
            }
            value.clone().into_value()?;
            packet.push(RuntimeHostInputUpdate {
                source: RuntimeHostInputSource::new(INPUT_BASE, &name)?,
                value,
            });
        }
        self.runtime
            .ingress()
            .submit(RuntimeHostInput::new(packet)?)?;
        self.runtime.drain_host_inputs(1)?;
        Ok(())
    }

    fn read_numbers(&self, name: &str) -> MResult<Vec<f64>> {
        let value = self.runtime.root_symbol_value(name)?;
        match RuntimeHostInputValue::from_value(value.value())? {
            RuntimeHostInputValue::F64(value) => Ok(vec![value]),
            RuntimeHostInputValue::F32(value) => Ok(vec![f64::from(value)]),
            RuntimeHostInputValue::F64Matrix { values, .. } => Ok(values),
            RuntimeHostInputValue::F32Matrix { values, .. } => {
                Ok(values.into_iter().map(f64::from).collect())
            }
            _ => Err(scene_error(format!(
                "`{name}` is not a floating-point scalar or matrix"
            ))),
        }
    }

    fn scene(&self) -> Option<SceneSnapshot> {
        self.backend.latest()
    }

    fn stop(&mut self) -> MResult<()> {
        if !self.stopped {
            self.runtime.shutdown()?;
            self.stopped = true;
        }
        Ok(())
    }
}

/// Compile and activate once, then submit numeric packets to a resident Mech
/// scene program. Only declared `input://browser/frame` reads and
/// `scene://view/frame/replace` writes are granted.
#[wasm_bindgen]
pub struct WasmSceneProgram {
    core: SceneProgramCore,
}

#[wasm_bindgen]
impl WasmSceneProgram {
    /// Input values are f64 numbers, arrays (column vectors), or
    /// `{rows, columns, values}` descriptors in logical row-major order.
    #[wasm_bindgen(js_name = fromSource)]
    pub fn from_source(source: &str, initial_inputs: JsValue) -> Result<WasmSceneProgram, JsValue> {
        Ok(Self {
            core: SceneProgramCore::from_source(source, input_packet(initial_inputs)?)
                .map_err(js_error)?,
        })
    }

    /// Submit one atomic input packet and synchronously execute one checked
    /// resident turn. Rejection retains the last accepted state and scene.
    pub fn turn(&mut self, updates: JsValue) -> Result<(), JsValue> {
        self.core.turn(input_packet(updates)?).map_err(js_error)
    }

    /// Read a named scalar or matrix. Matrices use logical row-major order.
    #[wasm_bindgen(js_name = readNumbers)]
    pub fn read_numbers(&self, name: &str) -> Result<Float64Array, JsValue> {
        Ok(Float64Array::from(
            self.core.read_numbers(name).map_err(js_error)?.as_slice(),
        ))
    }

    /// The scene host's last accepted snapshot, or null before first publication.
    pub fn scene(&self) -> Result<JsValue, JsValue> {
        match self.core.scene() {
            Some(scene) => serde_wasm_bindgen::to_value(&scene).map_err(js_error),
            None => Ok(JsValue::NULL),
        }
    }

    pub fn stop(&mut self) -> Result<(), JsValue> {
        self.core.stop().map_err(js_error)
    }
}

fn numeric_shape(value: &RuntimeHostInputValue) -> MResult<Option<(usize, usize)>> {
    match value {
        RuntimeHostInputValue::F64(_) => Ok(None),
        RuntimeHostInputValue::F64Matrix {
            rows,
            columns,
            values,
        } if *rows > 0 && *columns > 0 && rows.checked_mul(*columns) == Some(values.len()) => {
            Ok(Some((*rows, *columns)))
        }
        _ => Err(scene_error(
            "browser inputs must be f64 scalars or nonempty f64 matrices",
        )),
    }
}

fn input_packet(value: JsValue) -> Result<Packet, JsValue> {
    if value.is_null() || !value.is_object() || Array::is_array(&value) {
        return Err(js_error(
            "inputs must be an object mapping declared names to numbers",
        ));
    }
    let object = Object::from(value);
    let mut packet = BTreeMap::new();
    for key in Object::keys(&object).iter() {
        let name = key
            .as_string()
            .ok_or_else(|| js_error("input names must be strings"))?;
        let value = Reflect::get(&object, &key)?;
        let parsed = if let Some(number) = value.as_f64() {
            RuntimeHostInputValue::F64(number)
        } else if Array::is_array(&value)
            || value.is_instance_of::<Float64Array>()
            || value.is_instance_of::<Float32Array>()
        {
            let values = numeric_array(&value)?;
            RuntimeHostInputValue::F64Matrix {
                rows: values.len(),
                columns: 1,
                values,
            }
        } else if value.is_object() && !value.is_null() {
            let dimension = |name: &str| -> Result<usize, JsValue> {
                let raw = Reflect::get(&value, &JsValue::from_str(name))?
                    .as_f64()
                    .ok_or_else(|| js_error(format!("matrix {name} must be a positive integer")))?;
                if !raw.is_finite() || raw < 1.0 || raw.fract() != 0.0 || raw > u32::MAX as f64 {
                    return Err(js_error(format!(
                        "matrix {name} must be a positive integer"
                    )));
                }
                Ok(raw as usize)
            };
            RuntimeHostInputValue::F64Matrix {
                rows: dimension("rows")?,
                columns: dimension("columns")?,
                values: numeric_array(&Reflect::get(&value, &JsValue::from_str("values"))?)?,
            }
        } else {
            return Err(js_error(format!("input `{name}` must be numeric")));
        };
        numeric_shape(&parsed).map_err(js_error)?;
        packet.insert(name, parsed);
    }
    Ok(packet)
}

fn numeric_array(value: &JsValue) -> Result<Vec<f64>, JsValue> {
    if !Array::is_array(value)
        && !value.is_instance_of::<Float64Array>()
        && !value.is_instance_of::<Float32Array>()
    {
        return Err(js_error("matrix values must be a numeric array"));
    }
    Array::from(value)
        .iter()
        .map(|value| {
            value
                .as_f64()
                .ok_or_else(|| js_error("matrix values must contain only numbers"))
        })
        .collect()
}

#[derive(Debug)]
struct NumericInputFactory {
    declarations: Arc<Packet>,
    manifest: HostManifestConfig,
}

impl NumericInputFactory {
    fn new(declarations: Arc<Packet>) -> Self {
        Self {
            declarations,
            manifest: HostManifestConfig {
                provider: "input".into(),
                contexts: vec![HostContextManifest {
                    name: "frame".into(),
                    base_uri_template: "input://{instance}/frame".into(),
                    operations: vec!["read".into()],
                }],
            },
        }
    }
}

impl RuntimeHostFactory for NumericInputFactory {
    fn provider_name(&self) -> &str {
        "input"
    }
    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }
    fn validate_settings(&self, instance: &str, settings: &ConfigValue) -> MResult<()> {
        if instance != "browser" || !matches!(settings, ConfigValue::Map(map) if map.is_empty()) {
            return Err(scene_error(
                "numeric browser host requires the browser instance and empty settings",
            ));
        }
        Ok(())
    }
    fn instantiate(
        &self,
        instance: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        self.validate_settings(instance, settings)?;
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance, &self.manifest)?,
            resource_providers: vec![Box::new(NumericInputProvider(self.declarations.clone()))],
            input_drivers: vec![Box::new(NumericInputDriver {
                declarations: self.declarations.clone(),
                live: false,
            })],
        })
    }
}

#[derive(Debug)]
struct NumericInputProvider(Arc<Packet>);

impl NumericInputProvider {
    fn initial(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        if request.base_uri != INPUT_BASE {
            return Err(scene_error("unknown numeric browser resource"));
        }
        self.0
            .get(&request.path)
            .cloned()
            .ok_or_else(|| scene_error(format!("undeclared browser input `{}`", request.path)))?
            .into_value()
    }
}

impl RuntimeResourceProvider for NumericInputProvider {
    fn scheme(&self) -> &str {
        "input"
    }
    fn base_uris(&self) -> Vec<String> {
        vec![INPUT_BASE.into()]
    }
    fn semantic_read_contract(&self) -> Option<&'static mech_core::OperationContractDeclaration> {
        Some(mech_runtime::resource_observation_contract())
    }
    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.initial(request)
    }
    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.initial(request)
    }
}

#[derive(Debug)]
struct NumericInputDriver {
    declarations: Arc<Packet>,
    live: bool,
}

impl RuntimeHostInputDriver for NumericInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == INPUT_BASE && self.declarations.contains_key(source.path())
    }
    fn attach(&mut self, _ingress: RuntimeIngress) -> MResult<()> {
        Ok(())
    }
    fn start(&mut self) -> MResult<()> {
        self.live = true;
        Ok(())
    }
    fn stop(&mut self) -> MResult<()> {
        self.live = false;
        Ok(())
    }
    fn is_live(&self) -> bool {
        self.live
    }
}

#[derive(Debug, Clone)]
struct SceneProgramError(String);
impl MechErrorKind for SceneProgramError {
    fn name(&self) -> &str {
        "SceneProgramError"
    }
    fn message(&self) -> String {
        self.0.clone()
    }
}
fn scene_error(message: impl Into<String>) -> MechError {
    MechError::new(SceneProgramError(message.into()), None)
}
fn js_error(error: impl std::fmt::Debug) -> JsValue {
    js_sys::Error::new(&format!("{error:?}")).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r##"
@input := input://browser/frame{:read(step), :read(position)}
@scene := scene://view/frame{:write(replace)}
step := @input/step
position := @input/position
~count := 0.0
count = count + step
nonnegative! := position[1] >= 0
circles := |id<string> x<f64> y<f64> radius<f64> fill<string> stroke<string> stroke-width<f64> opacity<f64>|
  | "robot" position[1] position[2] 2 "#ffd166" "none" 0.0 1.0 |
presentation := {width: 100, height: 80, background: "#101010", circles: circles}
@scene/replace <- presentation
"##;

    fn inputs(step: f64, x: f64, y: f64) -> Packet {
        BTreeMap::from([
            ("step".into(), RuntimeHostInputValue::F64(step)),
            (
                "position".into(),
                RuntimeHostInputValue::F64Matrix {
                    rows: 2,
                    columns: 1,
                    values: vec![x, y],
                },
            ),
        ])
    }

    #[test]
    fn numeric_packets_update_resident_state_and_scene_tables() {
        let mut program = SceneProgramCore::from_source(SOURCE, inputs(0.0, 5.0, 6.0)).unwrap();
        program.turn(inputs(1.0, 10.0, 20.0)).unwrap();
        assert_eq!(program.read_numbers("position").unwrap(), vec![10.0, 20.0]);
        assert_eq!(program.read_numbers("count").unwrap(), vec![1.0]);
        let scene = program.scene().unwrap();
        assert_eq!((scene.circles[0].x, scene.circles[0].y), (10.0, 20.0));
        program.turn(inputs(1.0, 15.0, 25.0)).unwrap();
        assert_eq!(program.read_numbers("count").unwrap(), vec![2.0]);
        assert_eq!(program.scene().unwrap().circles[0].x, 15.0);
        assert_eq!(
            program
                .runtime
                .program_execution_info()
                .resident_accepted_turns,
            2
        );
    }

    #[test]
    fn rejection_retains_state_and_published_scene_then_recovers() {
        let mut program = SceneProgramCore::from_source(SOURCE, inputs(0.0, 5.0, 6.0)).unwrap();
        program.turn(inputs(1.0, 10.0, 20.0)).unwrap();
        let accepted = program.scene();
        assert!(program.turn(inputs(1.0, -1.0, 50.0)).is_err());
        assert_eq!(program.scene(), accepted);
        assert_eq!(program.read_numbers("count").unwrap(), vec![1.0]);
        program.turn(inputs(1.0, 30.0, 40.0)).unwrap();
        assert_eq!(program.read_numbers("count").unwrap(), vec![2.0]);
        assert_eq!(program.scene().unwrap().circles[0].x, 30.0);
    }

    #[test]
    fn malformed_or_unknown_updates_cannot_partially_submit() {
        let mut program = SceneProgramCore::from_source(SOURCE, inputs(0.0, 5.0, 6.0)).unwrap();
        let mut malformed = inputs(100.0, 9.0, 8.0);
        malformed.insert("position".into(), RuntimeHostInputValue::F64(4.0));
        assert!(program.turn(malformed).is_err());
        assert_eq!(program.runtime.pending_host_input_count().unwrap(), 0);
        let mut unknown = inputs(100.0, 9.0, 8.0);
        unknown.insert("undeclared".into(), RuntimeHostInputValue::F64(0.0));
        assert!(program.turn(unknown).is_err());
        assert_eq!(program.runtime.pending_host_input_count().unwrap(), 0);
        program.turn(inputs(1.0, 3.0, 4.0)).unwrap();
        assert_eq!(program.read_numbers("count").unwrap(), vec![1.0]);
    }

    #[test]
    fn undeclared_resources_and_capabilities_are_not_granted() {
        assert!(
            SceneProgramCore::from_source(
                "@input := input://browser/frame{:read(secret)}\nx := @input/secret",
                inputs(0.0, 5.0, 6.0),
            )
            .is_err()
        );
        assert!(
            SceneProgramCore::from_source(
                "@console := console://other/output{:write(line)}\n@console/line <- 1",
                inputs(0.0, 5.0, 6.0),
            )
            .is_err()
        );
    }

    #[test]
    fn stop_is_idempotent_and_blocks_later_turns() {
        let mut program = SceneProgramCore::from_source(SOURCE, inputs(0.0, 5.0, 6.0)).unwrap();
        program.stop().unwrap();
        program.stop().unwrap();
        assert!(program.turn(inputs(1.0, 1.0, 1.0)).is_err());
    }

    fn workshop_inputs(instances: usize) -> Packet {
        let mut values: Packet = [
            ("commit", 0.0),
            ("velocity", 1.0),
            ("omega", 0.015),
            ("noise", 0.02),
            ("camera-index", 1.0),
            ("camera-range", 100.0),
        ]
        .into_iter()
        .map(|(name, value)| (name.into(), RuntimeHostInputValue::F64(value)))
        .collect();
        values.insert(
            "lane-indices".into(),
            RuntimeHostInputValue::F64Matrix {
                rows: instances,
                columns: 1,
                values: (1..=instances).map(|index| index as f64).collect(),
            },
        );
        values.insert(
            "estimate".into(),
            RuntimeHostInputValue::F64Matrix {
                rows: 3,
                columns: 1,
                values: vec![55.0, 25.0, 0.4],
            },
        );
        values.insert(
            "covariance".into(),
            RuntimeHostInputValue::F64Matrix {
                rows: 3,
                columns: 3,
                values: vec![100.0, 0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 0.15],
            },
        );
        values
    }

    #[test]
    fn workshop_camera_and_scene_execute_from_mech_source() {
        let source = include_str!("../../../benchmarks/iros-2026/blog/source/scene.mec");
        let initial = workshop_inputs(256);
        let mut program = SceneProgramCore::from_source(source, initial.clone()).unwrap();
        program.turn(initial.clone()).unwrap();
        assert_eq!(
            program.read_numbers("truth").unwrap(),
            vec![55.0, 25.0, 0.4]
        );
        assert_eq!(program.read_numbers("accepted-turns").unwrap(), vec![0.0]);
        assert_eq!(
            program.read_numbers("camera-position").unwrap(),
            vec![140.0, 12.0]
        );
        assert_eq!(
            program.read_numbers("measurement-visible").unwrap(),
            vec![1.0]
        );
        let readings = program.read_numbers("readings").unwrap();
        assert_eq!(readings.len(), 256);
        assert!(readings.iter().all(|reading| reading.is_finite()));
        let predicted_x = 55.0 + 0.1 * 0.4_f64.cos();
        let predicted_y = 25.0 + 0.1 * 0.4_f64.sin();
        let expected_bearing =
            (12.0 - predicted_y).atan2(140.0 - predicted_x) - 0.4015 + 0.02 * 1.73_f64.sin();
        assert!((readings[0] - expected_bearing).abs() < 1e-10);
        let initial_path = program.read_numbers("truth-path").unwrap();
        let prepared_scene = program.scene().unwrap();
        assert_eq!(
            (prepared_scene.width, prepared_scene.height),
            (200.0, 130.0)
        );
        assert!(
            prepared_scene
                .line_strips
                .iter()
                .any(|line| line.id == "estimate-path")
        );
        program.turn(initial).unwrap();
        assert_eq!(program.read_numbers("readings").unwrap(), readings);
        assert_eq!(program.read_numbers("accepted-turns").unwrap(), vec![0.0]);
        assert_eq!(program.read_numbers("truth-path").unwrap(), initial_path);
        program
            .turn(BTreeMap::from([(
                "commit".into(),
                RuntimeHostInputValue::F64(1.0),
            )]))
            .unwrap();
        assert_eq!(program.read_numbers("accepted-turns").unwrap(), vec![1.0]);
        let truth = program.read_numbers("truth").unwrap();
        assert!((truth[0] - (55.0 + 0.1 * 0.4_f64.cos())).abs() < 1e-10);
        assert!((truth[1] - (25.0 + 0.1 * 0.4_f64.sin())).abs() < 1e-10);
        assert!((truth[2] - 0.4015).abs() < 1e-10);
        let accepted_path = program.read_numbers("truth-path").unwrap();
        assert_eq!(accepted_path.len(), 700);
        assert_eq!(&accepted_path[..698], &initial_path[2..]);
        assert!((accepted_path[698] - truth[0]).abs() < 1e-10);
        assert!((accepted_path[699] - (130.0 - truth[1])).abs() < 1e-10);
        let estimate_path = program.read_numbers("estimate-path").unwrap();
        assert_eq!(&estimate_path[698..], &[55.0, 105.0]);
        assert_ne!(&accepted_path[698..], &estimate_path[698..]);
        program
            .turn(BTreeMap::from([
                ("commit".into(), RuntimeHostInputValue::F64(0.0)),
                ("camera-index".into(), RuntimeHostInputValue::F64(2.0)),
            ]))
            .unwrap();
        assert_eq!(program.read_numbers("truth").unwrap(), truth);
        assert_eq!(program.read_numbers("truth-path").unwrap(), accepted_path);
        assert_eq!(
            program.read_numbers("camera-position").unwrap(),
            vec![35.0, 110.0]
        );
        let distance = program.read_numbers("camera-distance").unwrap()[0];
        for (range, expected) in [
            (distance, 1.0),
            (distance - 0.001, 0.0),
            (distance + 0.001, 1.0),
        ] {
            program
                .turn(BTreeMap::from([(
                    "camera-range".into(),
                    RuntimeHostInputValue::F64(range),
                )]))
                .unwrap();
            assert_eq!(
                program.read_numbers("measurement-visible").unwrap(),
                vec![expected]
            );
            assert_eq!(program.read_numbers("truth-path").unwrap(), accepted_path);
        }
        program
            .turn(BTreeMap::from([(
                "camera-range".into(),
                RuntimeHostInputValue::F64(1.0),
            )]))
            .unwrap();
        assert_eq!(
            program.read_numbers("measurement-visible").unwrap(),
            vec![0.0]
        );
        program
            .turn(BTreeMap::from([(
                "camera-range".into(),
                RuntimeHostInputValue::F64(100.0),
            )]))
            .unwrap();
        assert_eq!(
            program.read_numbers("measurement-visible").unwrap(),
            vec![1.0]
        );
        assert_eq!(program.read_numbers("truth-path").unwrap(), accepted_path);
    }

    #[test]
    fn workshop_camera_supports_every_displayed_batch_size() {
        let source = include_str!("../../../benchmarks/iros-2026/blog/source/scene.mec");
        for instances in [65_536, 4_096, 1] {
            let initial = workshop_inputs(instances);
            let mut program = SceneProgramCore::from_source(source, initial.clone())
                .unwrap_or_else(|error| panic!("scene batch {instances}: {error:?}"));
            program.turn(initial).unwrap();
            let readings = program.read_numbers("readings").unwrap();
            assert_eq!(readings.len(), instances);
            assert!(readings.iter().all(|reading| reading.is_finite()));
            assert_eq!(program.read_numbers("accepted-turns").unwrap(), vec![0.0]);
            assert!(program.scene().is_some());
            program.stop().unwrap();
        }
    }

    #[test]
    fn browser_profile_preserves_singleton_matrix_transport() {
        for (rows, columns) in [(1, 1), (1, 3), (3, 1), (2, 3)] {
            let values: Vec<f64> = (0..rows * columns)
                .map(|index| if index == 0 { -0.0 } else { index as f64 })
                .collect();
            let snapshot = RuntimeHostInputValue::F64Matrix {
                rows,
                columns,
                values: values.clone(),
            }
            .into_value()
            .unwrap();
            let RuntimeHostInputValue::F64Matrix {
                rows: actual_rows,
                columns: actual_columns,
                values: actual,
            } = RuntimeHostInputValue::from_numeric_value(&snapshot).unwrap()
            else {
                panic!("a singleton host matrix must not become a scalar");
            };
            assert_eq!((actual_rows, actual_columns), (rows, columns));
            assert_eq!(
                actual
                    .iter()
                    .map(|value| value.to_bits())
                    .collect::<Vec<_>>(),
                values
                    .iter()
                    .map(|value| value.to_bits())
                    .collect::<Vec<_>>()
            );
        }
    }
}
