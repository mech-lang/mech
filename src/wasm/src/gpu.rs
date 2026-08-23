use std::collections::BTreeMap;

use js_sys::{Array, Float32Array, Object, Reflect, Uint32Array};
use mech_gpu::{
    ElementwiseKernel, FixedShapeKernel, GpuBindingAccess, GpuBindingRole, WORKGROUP_SIZE,
};
use wasm_bindgen::JsValue;
use web_time::Instant;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CompileTimings {
    pub(crate) catalog_setup: f64,
    pub(crate) parsing: f64,
    pub(crate) artifact_compilation: f64,
    pub(crate) gpu_lowering: f64,
    pub(crate) input_capture: f64,
}

#[derive(Clone, Copy)]
pub(crate) enum BrowserGpuProgram<'a> {
    Elementwise(&'a ElementwiseKernel),
    FixedShape(&'a FixedShapeKernel),
}

pub(crate) fn gpu_program_manifest(
    program: BrowserGpuProgram<'_>,
    input_values: &BTreeMap<String, Vec<f32>>,
    timings: CompileTimings,
) -> Result<JsValue, JsValue> {
    let manifest_started = Instant::now();

    let manifest = Object::new();
    let (wgsl, dispatch_elements, kernel_kind) = match program {
        BrowserGpuProgram::Elementwise(program) => {
            (program.wgsl(), program.dispatch_elements(), "elementwise")
        }
        BrowserGpuProgram::FixedShape(program) => (
            program.wgsl(),
            u64::from(program.instances()),
            "fixed-shape",
        ),
    };
    set(&manifest, "kernelKind", kernel_kind)?;
    set(&manifest, "wgsl", wgsl)?;
    set(&manifest, "workgroupSize", WORKGROUP_SIZE)?;
    set(
        &manifest,
        "dispatchElements",
        u32::try_from(dispatch_elements)
            .map_err(|_| error("GPU dispatch size exceeds the browser limit"))?,
    )?;
    let fixed_inputs = match program {
        BrowserGpuProgram::FixedShape(program) => Some(
            program
                .physical_inputs(input_values)
                .map_err(|failure| error(failure.to_string()))?,
        ),
        BrowserGpuProgram::Elementwise(_) => None,
    };
    let fixed_states = match program {
        BrowserGpuProgram::FixedShape(program) => Some(program.physical_states()),
        BrowserGpuProgram::Elementwise(_) => None,
    };
    if let BrowserGpuProgram::FixedShape(program) = program
        && program.integrity_buffer().is_some()
        && program.instances() >= (1 << 24)
    {
        return Err(error(
            "checked WebGPU fault records support fewer than 2^24 instances",
        ));
    }

    let bindings = Array::new();
    match program {
        BrowserGpuProgram::Elementwise(program) => {
            for binding in program.bindings() {
                let value = binding_value(
                    binding.binding,
                    &binding.name,
                    match binding.access {
                        GpuBindingAccess::Read => "read",
                        GpuBindingAccess::ReadWrite => "read-write",
                    },
                    match binding.role() {
                        GpuBindingRole::Input => "input",
                        GpuBindingRole::StateRead => "state-read",
                        GpuBindingRole::StateWrite => "state-write",
                        GpuBindingRole::Output => "output",
                    },
                    binding.slot().get(),
                    usize::try_from(binding.elements)
                        .map_err(|_| error("GPU binding size exceeds the browser limit"))?,
                    "f32",
                )?;
                if let Some(initial) = input_values.get(&binding.name) {
                    set(
                        &value,
                        "initialValues",
                        Float32Array::from(initial.as_slice()),
                    )?;
                }
                bindings.push(&value);
            }
        }
        BrowserGpuProgram::FixedShape(program) => {
            for input in fixed_inputs.as_ref().expect("fixed input plan exists") {
                let value = binding_value(
                    input.binding,
                    &input.name,
                    "read",
                    "input",
                    input.slot.get(),
                    input.elements,
                    "f32",
                )?;
                set(
                    &value,
                    "initialValues",
                    Float32Array::from(input.initial_values.as_slice()),
                )?;
                bindings.push(&value);
            }
            for state in fixed_states.as_ref().expect("fixed state plan exists") {
                let read = binding_value(
                    state.read_binding,
                    &format!("state.{}.read", state.slot.get()),
                    "read",
                    "state-read",
                    state.slot.get(),
                    state.elements,
                    "f32",
                )?;
                bindings.push(&read);
                let write = binding_value(
                    state.write_binding,
                    &format!("state.{}.write", state.slot.get()),
                    "read-write",
                    "state-write",
                    state.slot.get(),
                    state.elements,
                    "f32",
                )?;
                bindings.push(&write);
            }
            if let Some(fault) = program.integrity_buffer() {
                let value = binding_value(
                    fault.binding,
                    "integrity-fault",
                    "read-write",
                    "integrity-fault",
                    0,
                    fault.words,
                    "u32",
                )?;
                set(
                    &value,
                    "initialValues",
                    Uint32Array::from([0_u32, u32::MAX].as_slice()),
                )?;
                bindings.push(&value);
            }
        }
    }
    set(&manifest, "bindings", bindings)?;

    let constraints = Array::new();
    if let BrowserGpuProgram::FixedShape(program) = program {
        for (index, (_id, name)) in program.named_integrity_constraints().enumerate() {
            let value = Object::new();
            set(
                &value,
                "code",
                u32::try_from(index + 1).map_err(|_| {
                    error("GPU integrity constraint count exceeds the browser limit")
                })?,
            )?;
            set(&value, "name", name)?;
            constraints.push(&value);
        }
    }
    set(&manifest, "constraints", constraints)?;

    let states = Array::new();
    match program {
        BrowserGpuProgram::Elementwise(program) => {
            for (slot, elements, initializer) in program.state_initializers() {
                let value = Object::new();
                set(&value, "slot", slot.get())?;
                set(
                    &value,
                    "elements",
                    u32::try_from(elements)
                        .map_err(|_| error("GPU state size exceeds the browser limit"))?,
                )?;
                set(
                    &value,
                    "elementsPerInstance",
                    u32::try_from(elements)
                        .map_err(|_| error("GPU state shape exceeds the browser limit"))?,
                )?;
                set(&value, "initialValues", Float32Array::from(initializer))?;
                states.push(&value);
            }
        }
        BrowserGpuProgram::FixedShape(program) => {
            for state in fixed_states.as_ref().expect("fixed state plan exists") {
                let value = Object::new();
                set(&value, "slot", state.slot.get())?;
                set(
                    &value,
                    "elements",
                    u32::try_from(state.elements)
                        .map_err(|_| error("GPU state size exceeds the browser limit"))?,
                )?;
                set(
                    &value,
                    "elementsPerInstance",
                    u32::try_from(state.elements_per_instance)
                        .map_err(|_| error("GPU state shape exceeds the browser limit"))?,
                )?;
                set(
                    &value,
                    "initialValues",
                    Float32Array::from(state.initial_values.as_slice()),
                )?;
                states.push(&value);
            }
        }
    }
    set(&manifest, "states", states)?;

    let outputs = Array::new();
    let compute = match program {
        BrowserGpuProgram::Elementwise(program) => program.compute_program(),
        BrowserGpuProgram::FixedShape(program) => program.compute_program(),
    };
    for output in &compute.interface().outputs {
        let value = Object::new();
        set(&value, "name", output.name.as_ref())?;
        set(&value, "slot", output.slot.get())?;
        let elements_per_instance = output
            .elements()
            .map_err(|failure| error(failure.to_string()))?;
        let elements = match program {
            BrowserGpuProgram::Elementwise(_) => elements_per_instance,
            BrowserGpuProgram::FixedShape(program) => elements_per_instance
                .checked_mul(program.instances() as usize)
                .ok_or_else(|| error("GPU output size exceeds the browser limit"))?,
        };
        set(
            &value,
            "elements",
            u32::try_from(elements)
                .map_err(|_| error("GPU output size exceeds the browser limit"))?,
        )?;
        set(
            &value,
            "elementsPerInstance",
            u32::try_from(elements_per_instance)
                .map_err(|_| error("GPU output shape exceeds the browser limit"))?,
        )?;
        let dimensions = Array::new();
        if let BrowserGpuProgram::FixedShape(program) = program {
            dimensions.push(&JsValue::from_f64(f64::from(program.instances())));
        }
        let sample_dimensions = Array::new();
        for dimension in &output.dimensions {
            dimensions.push(&JsValue::from_f64(*dimension as f64));
            sample_dimensions.push(&JsValue::from_f64(*dimension as f64));
        }
        set(&value, "dimensions", dimensions)?;
        set(&value, "sampleDimensions", sample_dimensions)?;
        set(
            &value,
            "physicalLayout",
            match program {
                BrowserGpuProgram::Elementwise(_) => "row-major",
                BrowserGpuProgram::FixedShape(_) => "column-major",
            },
        )?;
        outputs.push(&value);
    }
    set(&manifest, "outputs", outputs)?;

    let compile_timings = Object::new();
    set(&compile_timings, "catalogSetup", timings.catalog_setup)?;
    set(&compile_timings, "parsing", timings.parsing)?;
    set(
        &compile_timings,
        "artifactCompilation",
        timings.artifact_compilation,
    )?;
    set(&compile_timings, "gpuLowering", timings.gpu_lowering)?;
    set(&compile_timings, "inputCapture", timings.input_capture)?;
    set(
        &compile_timings,
        "manifestEncoding",
        milliseconds(manifest_started),
    )?;
    set(&manifest, "compileTimings", compile_timings)?;
    Ok(manifest.into())
}

fn binding_value(
    binding: u32,
    name: &str,
    access: &str,
    role: &str,
    slot: u32,
    elements: usize,
    scalar: &str,
) -> Result<Object, JsValue> {
    let value = Object::new();
    set(&value, "binding", binding)?;
    set(&value, "name", name)?;
    set(&value, "access", access)?;
    set(&value, "role", role)?;
    set(&value, "slot", slot)?;
    set(
        &value,
        "elements",
        u32::try_from(elements).map_err(|_| error("GPU binding size exceeds the browser limit"))?,
    )?;
    set(&value, "scalar", scalar)?;
    Ok(value)
}

fn milliseconds(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1_000.0
}

fn set(target: &Object, name: &str, value: impl Into<JsValue>) -> Result<(), JsValue> {
    Reflect::set(target, &JsValue::from_str(name), &value.into()).map(|_| ())
}

fn error(message: impl AsRef<str>) -> JsValue {
    js_sys::Error::new(message.as_ref()).into()
}
