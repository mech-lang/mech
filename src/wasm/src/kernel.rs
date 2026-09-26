//! Source-driven numerical kernels without document hosts or a coordinator.
//!
//! CPU turns execute the Rust scalar instruction evaluator. GPU manifests come
//! from the same compiled fixed-shape program and are consumed by the shared
//! `MechBrowserCompute.Device` runner. Backend changes explicitly reset state;
//! this interface does not claim to migrate a running GPU session to the CPU.

use std::collections::{BTreeMap, BTreeSet};

use js_sys::{Array, Float32Array, Object, Reflect};
use mech_core::{CellSlotId, ComputePlacement, MechCode, Program, SectionElement};
use mech_gpu::{BatchedCpuSession, ComputeLowerer, FixedShapeKernel, GpuKernelPlanSource};
use mech_runtime::RuntimeBuilder;
use wasm_bindgen::prelude::*;

use crate::gpu::{CompileTimings, gpu_program_manifest};

struct Export {
    slot: CellSlotId,
    width: usize,
    output_name: Option<String>,
}

struct KernelCore {
    program: FixedShapeKernel,
    cpu: BatchedCpuSession,
    initial_inputs: BTreeMap<String, Vec<f32>>,
    current_inputs: BTreeMap<String, Vec<f32>>,
    input_widths: BTreeMap<String, usize>,
    exports: BTreeMap<String, Export>,
}

impl KernelCore {
    fn from_source(
        source: &str,
        inputs: BTreeMap<String, Vec<f32>>,
        export_names: Vec<String>,
    ) -> Result<Self, String> {
        for (name, values) in &inputs {
            if values.is_empty() {
                return Err(format!("input `{name}` has no values"));
            }
        }
        let mut names = BTreeSet::new();
        for name in &export_names {
            if !names.insert(name.clone()) {
                return Err(format!("export `{name}` was supplied more than once"));
            }
        }
        let tree = mech_syntax::parse(source.trim()).map_err(|e| format!("{e:?}"))?;
        validate_source(&tree)?;
        let artifact = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_native_plan_catalog())
            .build_compiler()
            .map_err(|e| format!("{e:?}"))?
            .compile_tree_artifact_with_interface(
                &tree,
                &BTreeMap::new(),
                &inputs.keys().cloned().collect(),
                &export_names.iter().map(String::as_str).collect::<Vec<_>>(),
            )
            .map_err(|e| format!("{e:?}"))?
            .into_artifact();
        let program = ComputeLowerer
            .compile_broadcast(&artifact, &inputs)
            .map_err(|e| format!("{e:?}"))?;
        let input_widths: BTreeMap<_, _> = program
            .inputs()
            .map(|(name, width)| (name.to_owned(), width))
            .collect();
        for name in inputs.keys() {
            if !input_widths.contains_key(name) {
                return Err(format!("`{name}` is not a live input of this kernel"));
            }
        }
        let state_widths: BTreeMap<_, _> = program.state_layout().collect();
        let bindings: Vec<_> = artifact.interactive_symbol_bindings().collect();
        let mut exports = BTreeMap::new();
        for name in export_names {
            let binding = bindings
                .iter()
                .find(|binding| binding.lexical_name == name)
                .ok_or_else(|| format!("unknown export `{name}`"))?;
            let width = *state_widths
                .get(&binding.storage)
                .ok_or_else(|| format!("export `{name}` must name persistent state"))?;
            let output_name = program
                .compute_program()
                .interface()
                .outputs
                .iter()
                .find(|port| port.slot == binding.storage)
                .map(|port| port.name.to_string());
            exports.insert(
                name,
                Export {
                    slot: binding.storage,
                    width,
                    output_name,
                },
            );
        }
        let cpu = program.prepare_cpu(&inputs).map_err(|e| e.to_string())?;
        Ok(Self {
            program,
            cpu,
            current_inputs: inputs.clone(),
            initial_inputs: inputs,
            input_widths,
            exports,
        })
    }

    fn validate_updates(&self, updates: &BTreeMap<String, Vec<f32>>) -> Result<(), String> {
        for (name, values) in updates {
            let width = *self
                .input_widths
                .get(name)
                .ok_or_else(|| format!("`{name}` is not a live input of this kernel"))?;
            let batch_width = width
                .checked_mul(self.program.instances() as usize)
                .ok_or_else(|| "input size overflow".to_owned())?;
            if values.len() != width && values.len() != batch_width {
                return Err(format!(
                    "input `{name}` needs {width} broadcast or {batch_width} batch values, got {}",
                    values.len(),
                ));
            }
        }
        Ok(())
    }

    fn turn(&mut self, updates: BTreeMap<String, Vec<f32>>) -> Result<(), String> {
        // Validate the complete packet before the executor changes any inputs.
        self.validate_updates(&updates)?;
        self.cpu
            .update_inputs(&updates)
            .map_err(|e| e.to_string())?;
        self.current_inputs.extend(updates);
        self.cpu.dispatch_turns(1).map_err(|e| e.to_string())
    }

    fn state(&self, name: &str) -> Result<&[f32], String> {
        let export = self
            .exports
            .get(name)
            .ok_or_else(|| format!("state `{name}` is not exported"))?;
        self.cpu
            .state()
            .get(&export.slot)
            .map(Vec::as_slice)
            .ok_or_else(|| format!("state `{name}` has no resident storage"))
    }

    fn reset(&mut self) -> Result<(), String> {
        self.cpu = self
            .program
            .prepare_cpu(&self.initial_inputs)
            .map_err(|e| e.to_string())?;
        self.current_inputs = self.initial_inputs.clone();
        Ok(())
    }
}

fn validate_source(tree: &Program) -> Result<(), String> {
    let mut regions = 0;
    for section in &tree.body.sections {
        match mech_engine::section_compute_placement(section).map_err(|e| format!("{e:?}"))? {
            Some(ComputePlacement::Compute) => regions += 1,
            Some(_) => {
                return Err(
                    "select a backend in the host; use one neutral @compute section".into(),
                );
            }
            None if section.elements.iter().all(|element| {
                matches!(element,
                    SectionElement::MechCode(code) if code.iter().all(|(code, comment)|
                        matches!(code, MechCode::Import(_)) && comment.is_none())
                )
            }) => {}
            None => return Err("only imports may appear outside the @compute section".into()),
        }
    }
    if regions != 1 {
        return Err(format!(
            "kernel source requires one @compute section, found {regions}"
        ));
    }
    Ok(())
}

/// Compile one fixed-shape f32 Mech kernel for Rust/WASM CPU or generated WGSL.
/// Matrices use column-major elements within each consecutive batch instance.
#[wasm_bindgen]
pub struct WasmKernel {
    core: KernelCore,
}

#[wasm_bindgen]
impl WasmKernel {
    /// `inputs` is an object of Float32Array or number[] values; `exports` is a
    /// string[]. A non-singleton input extent determines the fixed batch size.
    #[wasm_bindgen(js_name = fromSource)]
    pub fn from_source(
        source: &str,
        inputs: JsValue,
        exports: JsValue,
    ) -> Result<WasmKernel, JsValue> {
        if !Array::is_array(&exports) {
            return Err(js_error("exports must be an array of state names"));
        }
        let names = Array::from(&exports)
            .iter()
            .map(|value| {
                value
                    .as_string()
                    .ok_or_else(|| js_error("every export must be a state name"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            core: KernelCore::from_source(source, input_packet(inputs)?, names)
                .map_err(js_error)?,
        })
    }

    /// Execute one checked scalar CPU turn. Integrity rejection leaves every
    /// published state unchanged; the newly supplied inputs remain bound.
    pub fn turn(&mut self, updates: JsValue) -> Result<(), JsValue> {
        self.core.turn(input_packet(updates)?).map_err(js_error)
    }

    /// Copy a full exported CPU state array, including every batch instance.
    pub fn state(&self, name: &str) -> Result<Float32Array, JsValue> {
        Ok(Float32Array::from(self.core.state(name).map_err(js_error)?))
    }

    /// Copy just one CPU instance, avoiding a full-batch readback for rendering.
    #[wasm_bindgen(js_name = stateSample)]
    pub fn state_sample(&self, name: &str, instance_index: u32) -> Result<Float32Array, JsValue> {
        if instance_index >= self.instances() {
            return Err(js_error(
                "state sample instance is outside the compiled batch",
            ));
        }
        let width = self.state_width(name)?;
        let offset = instance_index as usize * width;
        let values = self.core.state(name).map_err(js_error)?;
        Ok(Float32Array::from(&values[offset..offset + width]))
    }

    #[wasm_bindgen(js_name = stateWidth)]
    pub fn state_width(&self, name: &str) -> Result<usize, JsValue> {
        self.core
            .exports
            .get(name)
            .map(|export| export.width)
            .ok_or_else(|| js_error(format!("state `{name}` is not exported")))
    }

    pub fn instances(&self) -> u32 {
        self.core.program.instances()
    }

    #[wasm_bindgen(js_name = attemptedTurns)]
    pub fn attempted_turns(&self) -> f64 {
        self.core.cpu.attempted_turns() as f64
    }

    #[wasm_bindgen(js_name = faultCount)]
    pub fn fault_count(&self) -> f64 {
        self.core.cpu.fault_count() as f64
    }

    /// Reset CPU state and input bindings to their initial values. A caller
    /// must also dispose/recreate its GPU Device to reset GPU state.
    pub fn reset(&mut self) -> Result<(), JsValue> {
        self.core.reset().map_err(js_error)
    }

    /// A Device-compatible manifest from the same compiled program, with an
    /// `exports` array mapping public state names to GPU sampled output names.
    /// Device outputs currently sample instance zero, not the complete batch.
    #[wasm_bindgen(js_name = computeManifest)]
    pub fn compute_manifest(&self) -> Result<JsValue, JsValue> {
        let retained = self
            .core
            .exports
            .values()
            .filter_map(|export| export.output_name.clone())
            .collect();
        let manifest = gpu_program_manifest(
            GpuKernelPlanSource::FixedShape(&self.core.program),
            &self.core.initial_inputs,
            "wgpu",
            &retained,
            CompileTimings::default(),
        )?;
        let exports = Array::new();
        for (name, export) in &self.core.exports {
            let value = Object::new();
            Reflect::set(&value, &"name".into(), &name.as_str().into())?;
            Reflect::set(
                &value,
                &"slot".into(),
                &JsValue::from_f64(export.slot.get() as f64),
            )?;
            Reflect::set(
                &value,
                &"elementsPerInstance".into(),
                &JsValue::from_f64(export.width as f64),
            )?;
            Reflect::set(
                &value,
                &"outputName".into(),
                &export
                    .output_name
                    .as_deref()
                    .map(JsValue::from_str)
                    .unwrap_or(JsValue::NULL),
            )?;
            exports.push(&value);
        }
        Reflect::set(&manifest, &"exports".into(), &exports)?;
        Reflect::set(
            &manifest,
            &"instances".into(),
            &JsValue::from_f64(self.instances() as f64),
        )?;
        Ok(manifest)
    }

    /// Validate and expand an input packet into the shared GPU runner's
    /// [{name, values: Float32Array}] format. This does not execute a CPU turn.
    /// Omitted inputs retain their prior values; all current values are returned.
    #[wasm_bindgen(js_name = gpuInputs)]
    pub fn gpu_inputs(&mut self, updates: JsValue) -> Result<Array, JsValue> {
        let updates = input_packet(updates)?;
        self.core.validate_updates(&updates).map_err(js_error)?;
        self.core.current_inputs.extend(updates);
        let inputs = self
            .core
            .program
            .physical_inputs(&self.core.current_inputs)
            .map_err(|error| js_error(error.to_string()))?;
        let result = Array::new();
        for input in inputs {
            let value = Object::new();
            Reflect::set(&value, &"name".into(), &input.name.as_str().into())?;
            Reflect::set(
                &value,
                &"values".into(),
                &Float32Array::from(input.initial_values.as_slice()),
            )?;
            result.push(&value);
        }
        Ok(result)
    }
}

fn input_packet(value: JsValue) -> Result<BTreeMap<String, Vec<f32>>, JsValue> {
    if value.is_null() || value.is_undefined() {
        return Ok(BTreeMap::new());
    }
    if !value.is_object() || Array::is_array(&value) || value.is_instance_of::<Float32Array>() {
        return Err(js_error(
            "inputs must be an object mapping names to Float32Array or number[]",
        ));
    }
    let mut result = BTreeMap::new();
    for entry in Object::entries(&Object::from(value)).iter() {
        let entry = Array::from(&entry);
        let name = entry
            .get(0)
            .as_string()
            .ok_or_else(|| js_error("invalid input name"))?;
        let values = entry.get(1);
        let values = if values.is_instance_of::<Float32Array>() {
            Float32Array::new(&values).to_vec()
        } else if Array::is_array(&values) {
            Array::from(&values)
                .iter()
                .map(|element| {
                    element
                        .as_f64()
                        .map(|n| n as f32)
                        .ok_or_else(|| js_error(format!("input `{name}` contains a non-number")))
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            return Err(js_error(format!(
                "input `{name}` must be Float32Array or number[]"
            )));
        };
        result.insert(name, values);
    }
    Ok(result)
}

fn js_error(message: impl AsRef<str>) -> JsValue {
    JsValue::from_str(message.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAPER: &str = include_str!("../tests/fixtures/paper-ekf.mec");
    const LIVE_EKF: &str = include_str!("../../../benchmarks/iros-2026/blog/source/ekf.mec");

    fn build() -> KernelCore {
        KernelCore::from_source(
            PAPER,
            BTreeMap::from([("bearing".into(), vec![-0.55; 4])]),
            vec!["state".into(), "covariance".into()],
        )
        .unwrap()
    }

    fn bits(core: &KernelCore) -> Vec<u32> {
        ["state", "covariance"]
            .into_iter()
            .flat_map(|name| {
                core.state(name)
                    .unwrap()
                    .iter()
                    .map(|value| value.to_bits())
            })
            .collect()
    }

    #[test]
    fn paper_kernel_cpu_turn_rejects_nan_without_publication_and_recovers() {
        let mut core = build();
        let initial = bits(&core);
        core.turn(BTreeMap::from([("bearing".into(), vec![-0.54])]))
            .unwrap();
        let accepted = bits(&core);
        assert_ne!(initial, accepted);
        assert_eq!(core.state("state").unwrap().len(), 12);
        let error = core
            .turn(BTreeMap::from([(
                "bearing".into(),
                vec![-0.55, -0.55, -0.55, f32::NAN],
            )]))
            .unwrap_err();
        assert!(error.contains("finite-candidate!"), "{error}");
        assert_eq!(bits(&core), accepted);
        assert_eq!(core.cpu.last_fault().unwrap().instance, 3);
        core.turn(BTreeMap::from([("bearing".into(), vec![-0.53])]))
            .unwrap();
        assert_ne!(bits(&core), accepted);
        core.reset().unwrap();
        assert_eq!(bits(&core), initial);
        assert_eq!(core.cpu.attempted_turns(), 0);
    }

    #[test]
    fn malformed_packet_does_not_partially_update_inputs() {
        let mut core = build();
        let before = bits(&core);
        assert!(
            core.turn(BTreeMap::from([
                ("bearing".into(), vec![-0.1]),
                ("unknown".into(), vec![1.0]),
            ]))
            .is_err()
        );
        assert_eq!(core.current_inputs["bearing"], vec![-0.55; 4]);
        assert_eq!(bits(&core), before);
    }

    #[test]
    fn exact_paper_source_generates_wgsl_and_two_exported_state_buffers() {
        let core = build();
        let plan = mech_gpu::GpuExecutionPlan::build(
            GpuKernelPlanSource::FixedShape(&core.program),
            &core.initial_inputs,
        )
        .unwrap();
        assert!(plan.wgsl.contains("@compute"));
        // The compact document uses both fmax and its negation. Its shortest
        // Rust decimal exceeds WGSL's f32 range, so emit the exact endpoint.
        assert!(plan.wgsl.contains("bitcast<f32>(0x7f7fffffu)"));
        assert!(
            !plan
                .wgsl
                .contains("340282350000000000000000000000000000000")
        );
        assert_eq!(plan.dispatch_elements, 4);
        assert_eq!(plan.states.len(), 2);
        assert_eq!(plan.constraints.len(), 3);
        for export in core.exports.values() {
            assert!(export.output_name.is_some());
            assert!(
                plan.outputs
                    .iter()
                    .any(|output| output.slot == export.slot.get())
            );
        }
    }

    fn live_inputs(instances: usize, visible: f32) -> BTreeMap<String, Vec<f32>> {
        BTreeMap::from([
            ("bearing".into(), vec![-0.55; instances]),
            ("u".into(), vec![1.0, 0.015, visible]),
            ("m".into(), vec![140.0, 12.0]),
        ])
    }

    fn live_kernel(instances: usize, visible: f32) -> KernelCore {
        KernelCore::from_source(
            LIVE_EKF,
            live_inputs(instances, visible),
            vec!["μ".into(), "Σ".into()],
        )
        .unwrap()
    }

    fn named_bits(core: &KernelCore, names: &[&str]) -> Vec<u32> {
        names
            .iter()
            .flat_map(|name| {
                core.state(name)
                    .unwrap()
                    .iter()
                    .map(|value| value.to_bits())
            })
            .collect()
    }

    #[test]
    fn live_ekf_camera_visibility_selects_prediction_and_resumes_correction() {
        let mut hidden = live_kernel(256, 0.0);
        let bearings = (0..256).map(|lane| -2.0 + lane as f32 * 0.02).collect();
        hidden
            .turn(BTreeMap::from([
                ("bearing".into(), bearings),
                ("u".into(), vec![2.0, 0.1, 0.0]),
            ]))
            .unwrap();
        let mean = hidden.state("μ").unwrap();
        let covariance = hidden.state("Σ").unwrap();
        for lane in 1..256 {
            assert_eq!(&mean[lane * 3..(lane + 1) * 3], &mean[..3]);
            assert_eq!(&covariance[lane * 9..(lane + 1) * 9], &covariance[..9]);
        }
        let (sin, cos) = 0.4_f32.sin_cos();
        let d = 0.2_f32;
        let expected_mean = [55.0 + d * cos, 25.0 + d * sin, 0.41];
        for (actual, expected) in mean[..3].iter().zip(expected_mean) {
            assert!((actual - expected).abs() <= 1e-5, "{actual} vs {expected}");
        }
        // G P G' + V Q V', independently expanded for the initial diagonal P.
        let xy = -0.15 * d * d * sin * cos + 0.0001 * sin * cos;
        let expected_covariance = [
            100.0 + 0.15 * d * d * sin * sin + 0.0001 * cos * cos,
            xy,
            -0.15 * d * sin,
            xy,
            100.0 + 0.15 * d * d * cos * cos + 0.0001 * sin * sin,
            0.15 * d * cos,
            -0.15 * d * sin,
            0.15 * d * cos,
            0.150025,
        ];
        for (actual, expected) in covariance[..9].iter().zip(expected_covariance) {
            assert!((actual - expected).abs() <= 2e-5, "{actual} vs {expected}");
        }
        let mut corrected = live_kernel(256, 0.0);
        corrected.turn(hidden.current_inputs.clone()).unwrap();
        assert_eq!(
            named_bits(&corrected, &["μ", "Σ"]),
            named_bits(&hidden, &["μ", "Σ"])
        );
        corrected
            .turn(BTreeMap::from([
                ("bearing".into(), vec![-0.3]),
                ("u".into(), vec![2.0, 0.1, 1.0]),
            ]))
            .unwrap();
        hidden
            .turn(BTreeMap::from([("bearing".into(), vec![-0.3])]))
            .unwrap();
        assert_ne!(corrected.state("μ").unwrap(), hidden.state("μ").unwrap());
        let trace = |core: &KernelCore| {
            let p = core.state("Σ").unwrap();
            p[0] + p[4] + p[8]
        };
        assert!(trace(&corrected) < trace(&hidden));
    }

    #[test]
    fn live_ekf_switches_landmark_and_rejects_nan_atomically_then_recovers() {
        let mut changed = live_kernel(256, 1.0);
        let mut reference = live_kernel(256, 1.0);
        changed
            .turn(BTreeMap::from([("m".into(), vec![35.0, 110.0])]))
            .unwrap();
        reference.turn(BTreeMap::new()).unwrap();
        assert_ne!(
            named_bits(&changed, &["μ", "Σ"]),
            named_bits(&reference, &["μ", "Σ"])
        );
        reference.reset().unwrap();
        reference
            .turn(BTreeMap::from([("m".into(), vec![35.0, 110.0])]))
            .unwrap();
        let accepted = named_bits(&changed, &["μ", "Σ"]);
        let mut bearings = vec![-0.5; 256];
        bearings[127] = f32::NAN;
        let error = changed
            .turn(BTreeMap::from([("bearing".into(), bearings)]))
            .unwrap_err();
        assert!(error.contains("finite-candidate!"), "{error}");
        assert_eq!(changed.cpu.last_fault().unwrap().instance, 127);
        assert_eq!(named_bits(&changed, &["μ", "Σ"]), accepted);
        let update = BTreeMap::from([("bearing".into(), vec![-0.5])]);
        changed.turn(update.clone()).unwrap();
        reference.turn(update).unwrap();
        assert_eq!(
            named_bits(&changed, &["μ", "Σ"]),
            named_bits(&reference, &["μ", "Σ"])
        );
    }

    #[test]
    fn live_ekf_vector_tolerance_matches_scalar_formula_state_shapes_and_faults() {
        const VECTOR: &str = "τ := ε + ρ * abs(Σraw[[4 7 8]]) + ρ * abs(Σraw[[2 3 6]])";
        const SCALAR: &str = "τxy := ε + ρ * abs(Σraw[4]) + ρ * abs(Σraw[2])\nτxθ := ε + ρ * abs(Σraw[7]) + ρ * abs(Σraw[3])\nτyθ := ε + ρ * abs(Σraw[8]) + ρ * abs(Σraw[6])\nτ := [τxy τxθ τyθ]'";
        assert_eq!(LIVE_EKF.matches(VECTOR).count(), 1);
        // Test-only persistent copies expose the two intermediates without
        // changing production exports or numerical operations.
        let instrumented = LIVE_EKF.replace("~μ<[f32]>",
            "~tolerance-snapshot<[f32]> := [0 0 0]'\n~symmetry-snapshot<[f32]> := [0 0 0]'\n~μ<[f32]>")
            .replace("(μ, Σ)", "tolerance-snapshot = τ\nsymmetry-snapshot = ΔΣ\n(μ, Σ)");
        let scalar_source = instrumented.replace(VECTOR, SCALAR);
        let names = ["μ", "Σ", "tolerance-snapshot", "symmetry-snapshot"];
        let build = |source: &str| {
            let mut inputs = live_inputs(256, 1.0);
            inputs.insert("ε".into(), vec![0.0001]);
            KernelCore::from_source(
                source,
                inputs,
                names.iter().map(|name| (*name).into()).collect(),
            )
            .unwrap()
        };
        let mut vector = build(&instrumented);
        let mut scalar = build(&scalar_source);
        for core in [&vector, &scalar] {
            for name in &names[2..] {
                let slot = core.exports[*name].slot;
                let storage = core
                    .program
                    .compute_program()
                    .fixed_shape_storage()
                    .unwrap();
                let state = storage
                    .states
                    .iter()
                    .find(|state| state.slot == slot)
                    .unwrap();
                assert_eq!((state.shape.rows, state.shape.columns), (3, 1));
            }
        }
        for turn in 0..40 {
            let updates = BTreeMap::from([
                (
                    "bearing".into(),
                    (0..256)
                        .map(|lane| -0.55 + 0.02 * (turn as f32 * 1.73 + lane as f32 * 0.37).sin())
                        .collect(),
                ),
                (
                    "u".into(),
                    vec![
                        1.0 + turn as f32 * 0.01,
                        0.015,
                        if turn % 7 == 0 { 0.0 } else { 1.0 },
                    ],
                ),
                (
                    "m".into(),
                    if turn < 20 {
                        vec![140.0, 12.0]
                    } else {
                        vec![35.0, 110.0]
                    },
                ),
            ]);
            vector.turn(updates.clone()).unwrap();
            scalar.turn(updates).unwrap();
            assert_eq!(
                named_bits(&vector, &names),
                named_bits(&scalar, &names),
                "turn {turn}"
            );
        }
        for (updates, expected_constraint) in [
            (
                BTreeMap::from([("ε".into(), vec![-0.01])]),
                "symmetric-covariance!",
            ),
            (
                BTreeMap::from([("bearing".into(), {
                    let mut values = vec![-0.55; 256];
                    values[129] = f32::NAN;
                    values
                })]),
                "finite-candidate!",
            ),
        ] {
            let before = named_bits(&vector, &names);
            let vector_error = vector.turn(updates.clone()).unwrap_err();
            let scalar_error = scalar.turn(updates).unwrap_err();
            assert!(vector_error.contains(expected_constraint), "{vector_error}");
            assert_eq!(vector_error, scalar_error);
            assert_eq!(vector.cpu.last_fault(), scalar.cpu.last_fault());
            assert_eq!(named_bits(&vector, &names), before);
            assert_eq!(named_bits(&scalar, &names), before);
            let recovery =
                BTreeMap::from([("ε".into(), vec![0.0001]), ("bearing".into(), vec![-0.55])]);
            vector.turn(recovery.clone()).unwrap();
            scalar.turn(recovery).unwrap();
            assert_eq!(named_bits(&vector, &names), named_bits(&scalar, &names));
        }
    }

    #[test]
    fn elementwise_absolute_preserves_scalar_vector_and_nonsquare_matrix_shapes() {
        for (literal, rows, columns) in [
            ("0f32", 1, 1),
            ("[0f32 0f32 0f32 0f32 0f32]", 1, 5),
            ("[0f32; 0f32; 0f32; 0f32]", 4, 1),
            ("[0f32 0f32 0f32; 0f32 0f32 0f32]", 2, 3),
        ] {
            let elements = rows * columns;
            let source = format!(
                "+> math/abs\n\nAbsolute @compute\n------------------\ninput := {literal}\n~output := {literal}\noutput = abs(input)\noutput"
            );
            // Five instances exercise the SIMD tail as well as each component
            // in column-major storage. Negative zero tests abs sign handling.
            let input: Vec<f32> = (0..5 * elements).map(|index| -(index as f32)).collect();
            let expected: Vec<u32> = input.iter().map(|value| value.abs().to_bits()).collect();
            let mut core = KernelCore::from_source(
                &source,
                BTreeMap::from([("input".into(), input)]),
                vec!["output".into()],
            )
            .unwrap();
            let slot = core.exports["output"].slot;
            let storage = core
                .program
                .compute_program()
                .fixed_shape_storage()
                .unwrap();
            let output = storage
                .states
                .iter()
                .find(|state| state.slot == slot)
                .unwrap();
            assert_eq!((output.shape.rows, output.shape.columns), (rows, columns));
            let mech_compute::ComputeKernel::FixedShape(ir) =
                core.program.compute_program().kernel()
            else {
                panic!("expected scalarized fixed-shape program");
            };
            assert_eq!(
                ir.instructions
                    .iter()
                    .filter(|instruction| matches!(
                        instruction.computation,
                        mech_compute::ScalarComputation::Absolute(_)
                    ))
                    .count(),
                elements
            );
            let mut simd = core.program.prepare_simd_cpu(&core.initial_inputs).unwrap();
            core.turn(BTreeMap::new()).unwrap();
            simd.dispatch_turns(1).unwrap();
            assert_eq!(named_bits(&core, &["output"]), expected);
            assert_eq!(
                simd.state()[&slot]
                    .iter()
                    .map(|value| value.to_bits())
                    .collect::<Vec<_>>(),
                expected
            );
            let gpu = mech_gpu::GpuExecutionPlan::build(
                GpuKernelPlanSource::FixedShape(&core.program),
                &core.initial_inputs,
            )
            .unwrap();
            assert!(gpu.wgsl.matches("abs(").count() >= elements);
            assert_eq!(gpu.dispatch_elements, 5);
        }
    }
}
