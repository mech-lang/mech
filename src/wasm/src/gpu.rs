use std::collections::BTreeMap;

use js_sys::{Array, Float32Array, Object, Reflect, Uint32Array};
use mech_gpu::{
    ElementwiseKernel, FixedShapeKernel, GpuExecutionPlan, GpuKernelPlanSource,
    GpuPlanInitialValues, GpuPlanKernelKind, GpuPlanLayout, GpuPlanScalar,
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
    backend: &str,
    timings: CompileTimings,
) -> Result<JsValue, JsValue> {
    let manifest_started = Instant::now();

    let manifest = Object::new();
    let plan = GpuExecutionPlan::build(
        match program {
            BrowserGpuProgram::Elementwise(program) => GpuKernelPlanSource::Elementwise(program),
            BrowserGpuProgram::FixedShape(program) => GpuKernelPlanSource::FixedShape(program),
        },
        input_values,
    )
    .map_err(|failure| error(failure.to_string()))?;
    set(
        &manifest,
        "physicalRevision",
        plan.physical_revision(backend)
            .map_err(|failure| error(failure.to_string()))?,
    )?;
    set(&manifest, "planVersion", plan.version)?;
    set(
        &manifest,
        "kernelKind",
        match plan.kernel_kind {
            GpuPlanKernelKind::Elementwise => "elementwise",
            GpuPlanKernelKind::FixedShape => "fixed-shape",
        },
    )?;
    set(&manifest, "wgsl", plan.wgsl.as_str())?;
    set(&manifest, "workgroupSize", plan.workgroup_size)?;
    set(&manifest, "dispatchElements", plan.dispatch_elements)?;

    let bindings = Array::new();
    for binding in &plan.bindings {
        let encoded = binding_value(
            binding.binding,
            &binding.name,
            match binding.access {
                mech_gpu::GpuPlanBindingAccess::Read => "read",
                mech_gpu::GpuPlanBindingAccess::ReadWrite => "read-write",
            },
            match binding.role {
                mech_gpu::GpuPlanBindingRole::Input => "input",
                mech_gpu::GpuPlanBindingRole::StateRead => "state-read",
                mech_gpu::GpuPlanBindingRole::StateWrite => "state-write",
                mech_gpu::GpuPlanBindingRole::Output => "output",
                mech_gpu::GpuPlanBindingRole::IntegrityFault => "integrity-fault",
            },
            binding.slot,
            usize::try_from(binding.elements)
                .map_err(|_| error("GPU binding size exceeds the browser limit"))?,
            match binding.scalar {
                GpuPlanScalar::F32 => "f32",
                GpuPlanScalar::U32 => "u32",
            },
        )?;
        match &binding.initial_values {
            Some(GpuPlanInitialValues::F32(values)) => set(
                &encoded,
                "initialValues",
                Float32Array::from(values.as_slice()),
            )?,
            Some(GpuPlanInitialValues::U32(values)) => set(
                &encoded,
                "initialValues",
                Uint32Array::from(values.as_slice()),
            )?,
            None => {}
        }
        bindings.push(&encoded);
    }
    set(&manifest, "bindings", bindings)?;

    let constraints = Array::new();
    for constraint in &plan.constraints {
        let value = Object::new();
        set(&value, "code", constraint.code)?;
        set(&value, "id", constraint.id.to_string())?;
        set(&value, "name", constraint.name.as_str())?;
        constraints.push(&value);
    }
    set(&manifest, "constraints", constraints)?;

    let states = Array::new();
    for state in &plan.states {
        let value = Object::new();
        set(&value, "slot", state.slot)?;
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
    set(&manifest, "states", states)?;

    let outputs = Array::new();
    for output in &plan.outputs {
        let value = Object::new();
        set(&value, "name", output.name.as_str())?;
        set(&value, "slot", output.slot)?;
        set(&value, "physicalOutput", output.physical_output)?;
        set(
            &value,
            "elements",
            u32::try_from(output.elements)
                .map_err(|_| error("GPU output size exceeds the browser limit"))?,
        )?;
        set(
            &value,
            "elementsPerInstance",
            u32::try_from(output.elements_per_instance)
                .map_err(|_| error("GPU output shape exceeds the browser limit"))?,
        )?;
        let dimensions = Array::new();
        let sample_dimensions = Array::new();
        for dimension in &output.dimensions {
            dimensions.push(&JsValue::from_f64(*dimension as f64));
        }
        for dimension in &output.sample_dimensions {
            sample_dimensions.push(&JsValue::from_f64(*dimension as f64));
        }
        set(&value, "dimensions", dimensions)?;
        set(&value, "sampleDimensions", sample_dimensions)?;
        set(
            &value,
            "physicalLayout",
            match output.physical_layout {
                GpuPlanLayout::RowMajor => "row-major",
                GpuPlanLayout::ColumnMajor => "column-major",
            },
        )?;
        outputs.push(&value);
    }
    set(&manifest, "outputs", outputs)?;

    let physical_outputs = Array::new();
    for output in &plan.physical_outputs {
        let value = Object::new();
        set(&value, "id", output.id)?;
        set(&value, "slot", output.slot)?;
        if let Some(binding) = output.binding {
            set(&value, "binding", binding)?;
        }
        set(
            &value,
            "sampleElements",
            u32::try_from(output.sample_elements)
                .map_err(|_| error("GPU sampled output size exceeds the browser limit"))?,
        )?;
        let aliases = Array::new();
        for alias in &output.aliases {
            aliases.push(&JsValue::from_str(alias));
        }
        set(&value, "aliases", aliases)?;
        physical_outputs.push(&value);
    }
    set(&manifest, "physicalOutputs", physical_outputs)?;

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
