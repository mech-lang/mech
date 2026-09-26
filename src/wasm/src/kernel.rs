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
        assert!(!plan.wgsl.contains("340282350000000000000000000000000000000"));
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
}
