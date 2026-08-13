use std::collections::BTreeMap;

use js_sys::{Array, Float32Array, Object, Reflect};
use mech_core::hash_str;
use mech_engine::{MechProgram, MechProgramConfig};
use mech_gpu::{GpuBindingAccess, GpuBindingRole, GpuHost, GpuProgram, WORKGROUP_SIZE};
use mech_runtime::{ConfigProfileOptions, parse_config_document};
use wasm_bindgen::prelude::*;
use web_time::Instant;

#[derive(Clone, Copy, Debug, Default)]
struct CompileTimings {
    catalog_setup: f64,
    parsing: f64,
    source_execution: f64,
    artifact_compilation: f64,
    gpu_lowering: f64,
    input_capture: f64,
}

#[wasm_bindgen(js_name = requiredGpuPaths)]
pub fn required_gpu_paths(config_source: &str) -> Result<Array, JsValue> {
    let document = parse_config_document(
        "browser-project/mech.mcfg",
        config_source,
        ConfigProfileOptions::default(),
    )
    .map_err(|failure| error(format!("Mech config rejected: {failure:?}")))?;
    let run = document
        .run
        .ok_or_else(|| error("GPU project config must contain run settings"))?;
    if run.paths.is_empty() {
        return Err(error("GPU project config must contain one run path"));
    }
    let paths = Array::new();
    for path in run.paths {
        paths.push(&JsValue::from_str(&path.to_string_lossy()));
    }
    Ok(paths)
}

#[wasm_bindgen(js_name = configuredExecutor)]
pub fn configured_executor(config_source: &str) -> Result<JsValue, JsValue> {
    let document = parse_config_document(
        "browser-project/mech.mcfg",
        config_source,
        ConfigProfileOptions::default(),
    )
    .map_err(|failure| error(format!("Mech config rejected: {failure:?}")))?;
    let Some(executor) = document.run.and_then(|run| run.executor) else {
        return Ok(JsValue::NULL);
    };
    let value = Object::new();
    set(&value, "provider", executor.provider)?;
    set(&value, "turns", executor.turns)?;
    Ok(value.into())
}

#[wasm_bindgen(js_name = compileGpuProgram)]
pub fn compile_gpu_program(source: &str) -> Result<JsValue, JsValue> {
    let (program, input_values, timings) = compile_program(source).map_err(error)?;
    let manifest_started = Instant::now();

    let manifest = Object::new();
    set(&manifest, "wgsl", program.wgsl())?;
    set(&manifest, "workgroupSize", WORKGROUP_SIZE)?;
    set(
        &manifest,
        "dispatchElements",
        u32::try_from(program.dispatch_elements())
            .map_err(|_| error("GPU dispatch size exceeds the browser limit"))?,
    )?;

    let bindings = Array::new();
    for binding in program.bindings() {
        let value = Object::new();
        set(&value, "binding", binding.binding)?;
        set(&value, "name", binding.name.as_str())?;
        set(
            &value,
            "access",
            match binding.access {
                GpuBindingAccess::Read => "read",
                GpuBindingAccess::ReadWrite => "read-write",
            },
        )?;
        set(
            &value,
            "role",
            match binding.role() {
                GpuBindingRole::Input => "input",
                GpuBindingRole::StateRead => "state-read",
                GpuBindingRole::StateWrite => "state-write",
                GpuBindingRole::Output => "output",
            },
        )?;
        set(&value, "slot", binding.slot().get())?;
        set(
            &value,
            "elements",
            u32::try_from(binding.elements)
                .map_err(|_| error("GPU binding size exceeds the browser limit"))?,
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
    set(&manifest, "bindings", bindings)?;

    let states = Array::new();
    for (slot, elements, initializer) in program.state_initializers() {
        let value = Object::new();
        set(&value, "slot", slot.get())?;
        set(
            &value,
            "elements",
            u32::try_from(elements)
                .map_err(|_| error("GPU state size exceeds the browser limit"))?,
        )?;
        set(&value, "initialValues", Float32Array::from(initializer))?;
        states.push(&value);
    }
    set(&manifest, "states", states)?;

    let outputs = Array::new();
    for (name, slot, elements) in program.outputs() {
        let value = Object::new();
        set(&value, "name", name)?;
        set(&value, "slot", slot.get())?;
        set(
            &value,
            "elements",
            u32::try_from(elements)
                .map_err(|_| error("GPU output size exceeds the browser limit"))?,
        )?;
        let dimensions = Array::new();
        for dimension in program.output_dimensions(slot).unwrap_or_default() {
            dimensions.push(&JsValue::from_f64(*dimension as f64));
        }
        set(&value, "dimensions", dimensions)?;
        outputs.push(&value);
    }
    set(&manifest, "outputs", outputs)?;

    let compile_timings = Object::new();
    set(&compile_timings, "catalogSetup", timings.catalog_setup)?;
    set(&compile_timings, "parsing", timings.parsing)?;
    set(
        &compile_timings,
        "sourceExecution",
        timings.source_execution,
    )?;
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

fn compile_program(
    source: &str,
) -> Result<(GpuProgram, BTreeMap<String, Vec<f32>>, CompileTimings), String> {
    let catalog_started = Instant::now();
    let mut source_program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech_stdlib::source_catalog(),
    );
    let catalog_setup = milliseconds(catalog_started);

    let parse_started = Instant::now();
    let tree = mech_syntax::parse(source.trim())
        .map_err(|failure| format!("Mech source rejected: {failure:?}"))?;
    let parsing = milliseconds(parse_started);

    let source_started = Instant::now();
    source_program
        .run_tree(&tree)
        .map_err(|failure| format!("Mech source rejected: {failure:?}"))?;
    let source_execution = milliseconds(source_started);

    let artifact_started = Instant::now();
    let artifact = source_program
        .compile_program_product()
        .map_err(|failure| format!("Mech artifact compilation failed: {failure:?}"))?
        .into_parts()
        .0;
    let artifact_compilation = milliseconds(artifact_started);

    let gpu_started = Instant::now();
    let program = GpuHost
        .compile(&artifact)
        .map_err(|failure| format!("GPU host rejected the Mech program: {failure}"))?;
    let gpu_lowering = milliseconds(gpu_started);

    let input_started = Instant::now();
    let symbols = source_program.interpreter().symbols();
    let symbols = symbols.borrow();
    let mut input_values = BTreeMap::new();
    for binding in program
        .bindings()
        .iter()
        .filter(|binding| binding.role() == GpuBindingRole::Input)
    {
        let cell = symbols
            .get(hash_str(&binding.name))
            .ok_or_else(|| format!("GPU input `{}` has no source value", binding.name))?;
        let values = cell.borrow().as_vecf32().map_err(|failure| {
            format!("GPU input `{}` is not f32 data: {failure:?}", binding.name)
        })?;
        if values.len() != binding.elements as usize {
            return Err(format!(
                "GPU input `{}` has {} source value(s), expected {}",
                binding.name,
                values.len(),
                binding.elements,
            ));
        }
        input_values.insert(binding.name.clone(), values);
    }
    let input_capture = milliseconds(input_started);
    Ok((
        program,
        input_values,
        CompileTimings {
            catalog_setup,
            parsing,
            source_execution,
            artifact_compilation,
            gpu_lowering,
            input_capture,
        },
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

    const PARTICLE_SOURCE: &str = r#"
~positions := [0f32 0f32; 1f32 1f32]
~velocities := [0f32 0f32; 0f32 0f32]
acceleration := (0f32 - positions) * 0.34<f32>
next-velocities := (velocities + acceleration * 0.008333333<f32>) * 0.997<f32>
next-positions := positions + next-velocities * 0.008333333<f32>
velocities = next-velocities
positions = next-positions
(positions, velocities)
"#;

    #[test]
    fn selected_browser_features_compile_complete_particle_source_to_gpu_program() {
        let (program, inputs, _) =
            compile_program(PARTICLE_SOURCE).expect("browser compiler must admit the source");
        assert_eq!(program.dispatch_elements(), 4);
        assert!(
            program
                .wgsl()
                .starts_with("// Generated from a typed Mech ProgramArtifact.")
        );
        assert!(program.wgsl().contains("@compute @workgroup_size(64)"));
        assert!(inputs.values().all(|values| !values.is_empty()));
        assert_eq!(
            program
                .bindings()
                .iter()
                .filter(|binding| binding.role() == GpuBindingRole::StateRead)
                .count(),
            2
        );
        assert_eq!(program.outputs().count(), 2);
    }
}
