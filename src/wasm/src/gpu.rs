use js_sys::{Array, Object, Reflect};
use mech_core::{LegacyValue, Ref, hash_str, matrix::Matrix};
use mech_engine::{MechProgram, MechProgramConfig};
use mech_gpu::{GpuBindingAccess, GpuBindingRole, GpuHost, GpuProgram, WORKGROUP_SIZE};
use wasm_bindgen::prelude::*;

const MAX_BROWSER_PARTICLES: u32 = 2_000_000;

#[wasm_bindgen(js_name = compileGpuProgram)]
pub fn compile_gpu_program(source: &str, particle_capacity: u32) -> Result<JsValue, JsValue> {
    if !(1..=MAX_BROWSER_PARTICLES).contains(&particle_capacity) {
        return Err(error(format!(
            "particle capacity must be between 1 and {MAX_BROWSER_PARTICLES}"
        )));
    }
    let program = compile_program(source, particle_capacity).map_err(error)?;

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
        bindings.push(&value);
    }
    set(&manifest, "bindings", bindings)?;

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
        outputs.push(&value);
    }
    set(&manifest, "outputs", outputs)?;
    Ok(manifest.into())
}

fn compile_program(source: &str, particle_capacity: u32) -> Result<GpuProgram, String> {
    let elements = usize::try_from(particle_capacity)
        .ok()
        .and_then(|count| count.checked_mul(2))
        .ok_or_else(|| "particle element count overflow".to_owned())?;
    let zeros = vec![0.0_f32; elements];
    let mut source_program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech_stdlib::source_native_plan_catalog(),
    );
    let values = [
        (
            "host-positions",
            LegacyValue::MatrixF32(Matrix::from_vec(
                zeros.clone(),
                2,
                particle_capacity as usize,
            )),
        ),
        (
            "host-velocities",
            LegacyValue::MatrixF32(Matrix::from_vec(zeros, 2, particle_capacity as usize)),
        ),
        ("host-origin", LegacyValue::F32(Ref::new(0.0))),
        ("host-attraction", LegacyValue::F32(Ref::new(0.34))),
        ("host-drag", LegacyValue::F32(Ref::new(0.997))),
        ("host-dt", LegacyValue::F32(Ref::new(1.0 / 120.0))),
    ];
    let symbols = source_program.interpreter().symbols();
    for (name, value) in values {
        let id = hash_str(name);
        symbols.borrow_mut().insert(id, value, false);
        symbols
            .borrow()
            .dictionary
            .borrow_mut()
            .insert(id, name.to_owned());
    }
    source_program
        .run_string(source.trim())
        .map_err(|failure| format!("Mech source rejected: {failure:?}"))?;
    let artifact = source_program
        .compile_program_artifact()
        .map_err(|failure| format!("Mech artifact compilation failed: {failure:?}"))?;
    GpuHost
        .compile(&artifact)
        .map_err(|failure| format!("GPU host rejected the Mech program: {failure}"))
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

    const PARTICLE_SOURCE: &str =
        include_str!("../../../examples/gpu-particles/particle-kernel.mec");

    #[test]
    fn selected_browser_features_compile_particle_source_to_gpu_program() {
        let program = compile_program(PARTICLE_SOURCE, 1024)
            .expect("browser compiler feature closure must admit the particle source");
        assert_eq!(program.dispatch_elements(), 2048);
        assert!(
            program
                .wgsl()
                .starts_with("// Generated from a typed Mech ProgramArtifact.")
        );
        assert!(program.wgsl().contains("@compute @workgroup_size(64)"));
        assert_eq!(program.bindings().len(), 8);
        assert!(
            program.bindings().iter().all(|binding| {
                binding.role() != GpuBindingRole::Input || binding.elements == 1
            })
        );
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
