use std::{
    collections::BTreeMap,
    num::NonZeroU32,
    sync::{Arc, OnceLock},
    time::Instant,
};

use mech_compute::{
    BackendClass, BackendId, CPU_SCALAR_BACKEND, ComputeBackendCapabilities,
    ComputeBackendDescriptor, ComputeBackendError, ComputeBackendFactory, ComputeBackendRegistry,
    ComputeBackendRejection, ComputeDispatchReport, ComputeExecutable, ComputeExecutionError,
    ComputeInitializerSet, ComputeInputUpdate, ComputeKernel, ComputeOutputSelection,
    ComputeOutputSnapshot, ComputePort, ComputeProgram, ComputeSession, ComputeValue, TensorLayout,
    WGPU_BACKEND,
};

use crate::{GpuProgram, OwnedResidentCpuSession, ResidentGpuSession};

pub fn native_compute_backend_registry() -> Arc<ComputeBackendRegistry> {
    let mut registry = ComputeBackendRegistry::default();
    registry
        .register(Arc::new(CpuScalarBackendFactory::new()))
        .expect("static CPU backend ID is unique");
    registry
        .register(Arc::new(WgpuBackendFactory::new()))
        .expect("static wgpu backend ID is unique");
    Arc::new(registry)
}

#[derive(Debug)]
pub struct CpuScalarBackendFactory {
    descriptor: ComputeBackendDescriptor,
}

impl CpuScalarBackendFactory {
    pub fn new() -> Self {
        Self {
            descriptor: descriptor(CPU_SCALAR_BACKEND, BackendClass::Cpu, 100, true),
        }
    }
}

impl Default for CpuScalarBackendFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputeBackendFactory for CpuScalarBackendFactory {
    fn descriptor(&self) -> &ComputeBackendDescriptor {
        &self.descriptor
    }

    fn supports(&self, program: &ComputeProgram) -> Result<(), ComputeBackendRejection> {
        supports_elementwise(&self.descriptor.id, program)
    }

    fn compile(
        &self,
        program: &ComputeProgram,
    ) -> Result<Box<dyn ComputeExecutable>, ComputeBackendError> {
        let program = GpuProgram::from_compute_program(program).map_err(|error| {
            backend_error(
                &self.descriptor.id,
                "compile",
                format!("compute lowering failed: {error}"),
            )
        })?;
        Ok(Box::new(CpuScalarExecutable {
            backend: self.descriptor.id.clone(),
            program,
        }))
    }
}

struct CpuScalarExecutable {
    backend: BackendId,
    program: GpuProgram,
}

impl ComputeExecutable for CpuScalarExecutable {
    fn create_session(
        &self,
        initializers: &ComputeInitializerSet,
    ) -> Result<Box<dyn ComputeSession>, ComputeBackendError> {
        let inputs = initializer_inputs(&self.backend, &self.program, initializers)?;
        let session =
            self.program.clone().into_cpu(&inputs).map_err(|error| {
                backend_error(&self.backend, "create session", error.to_string())
            })?;
        Ok(Box::new(CpuScalarSession {
            backend: self.backend.clone(),
            session,
        }))
    }
}

struct CpuScalarSession {
    backend: BackendId,
    session: OwnedResidentCpuSession,
}

impl ComputeSession for CpuScalarSession {
    fn update_inputs(
        &mut self,
        updates: &[ComputeInputUpdate],
    ) -> Result<(), ComputeExecutionError> {
        let inputs = normalized_update_inputs(&self.backend, self.session.program_ref(), updates)?;
        self.session
            .update_inputs(&inputs)
            .map_err(|error| execution_error(&self.backend, "update inputs", error.to_string()))
    }

    fn dispatch(
        &mut self,
        turns: NonZeroU32,
    ) -> Result<ComputeDispatchReport, ComputeExecutionError> {
        let started = Instant::now();
        self.session
            .dispatch_turns(turns.get())
            .map_err(|error| execution_error(&self.backend, "dispatch", error.to_string()))?;
        Ok(ComputeDispatchReport {
            completed_turns: turns.get(),
            dispatch_milliseconds: started.elapsed().as_secs_f64() * 1_000.0,
            ..Default::default()
        })
    }

    fn read_outputs(
        &mut self,
        selection: &ComputeOutputSelection,
    ) -> Result<ComputeOutputSnapshot, ComputeExecutionError> {
        let outputs = self
            .session
            .outputs()
            .map_err(|error| execution_error(&self.backend, "read outputs", error.to_string()))?;
        output_snapshot(self.session.program_ref(), selection, &outputs)
            .map_err(|detail| execution_error(&self.backend, "read outputs", detail))
    }
}

pub struct WgpuBackendFactory {
    descriptor: ComputeBackendDescriptor,
    availability: OnceLock<Result<(), Box<str>>>,
}

impl std::fmt::Debug for WgpuBackendFactory {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WgpuBackendFactory")
            .field("descriptor", &self.descriptor)
            .field("availability", &self.availability.get())
            .finish()
    }
}

impl WgpuBackendFactory {
    pub fn new() -> Self {
        Self {
            descriptor: descriptor(WGPU_BACKEND, BackendClass::Gpu, 400, true),
            availability: OnceLock::new(),
        }
    }

    fn available(&self) -> Result<(), Box<str>> {
        self.availability
            .get_or_init(|| {
                let instance = wgpu::Instance::default();
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                }))
                .map(|_| ())
                .ok_or_else(|| "no compatible native wgpu adapter is available".into())
            })
            .clone()
    }
}

impl Default for WgpuBackendFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputeBackendFactory for WgpuBackendFactory {
    fn descriptor(&self) -> &ComputeBackendDescriptor {
        &self.descriptor
    }

    fn supports(&self, program: &ComputeProgram) -> Result<(), ComputeBackendRejection> {
        supports_elementwise(&self.descriptor.id, program)?;
        self.available().map_err(|reason| ComputeBackendRejection {
            backend: self.descriptor.id.clone(),
            reason,
        })
    }

    fn compile(
        &self,
        program: &ComputeProgram,
    ) -> Result<Box<dyn ComputeExecutable>, ComputeBackendError> {
        let program = GpuProgram::from_compute_program(program).map_err(|error| {
            backend_error(
                &self.descriptor.id,
                "compile",
                format!("compute lowering failed: {error}"),
            )
        })?;
        Ok(Box::new(WgpuExecutable {
            backend: self.descriptor.id.clone(),
            program,
        }))
    }
}

struct WgpuExecutable {
    backend: BackendId,
    program: GpuProgram,
}

impl ComputeExecutable for WgpuExecutable {
    fn create_session(
        &self,
        initializers: &ComputeInitializerSet,
    ) -> Result<Box<dyn ComputeSession>, ComputeBackendError> {
        let inputs = initializer_inputs(&self.backend, &self.program, initializers)?;
        let session = self
            .program
            .prepare_resident(&inputs)
            .map_err(|error| backend_error(&self.backend, "create session", error.to_string()))?;
        Ok(Box::new(WgpuSession {
            backend: self.backend.clone(),
            program: self.program.clone(),
            session,
        }))
    }
}

struct WgpuSession {
    backend: BackendId,
    program: GpuProgram,
    session: ResidentGpuSession,
}

impl ComputeSession for WgpuSession {
    fn update_inputs(
        &mut self,
        updates: &[ComputeInputUpdate],
    ) -> Result<(), ComputeExecutionError> {
        let inputs = normalized_update_inputs(&self.backend, &self.program, updates)?;
        for (name, values) in inputs {
            self.session.update_input(&name, &values).map_err(|error| {
                execution_error(&self.backend, "update inputs", error.to_string())
            })?;
        }
        Ok(())
    }

    fn dispatch(
        &mut self,
        turns: NonZeroU32,
    ) -> Result<ComputeDispatchReport, ComputeExecutionError> {
        let elapsed = self
            .session
            .dispatch_turns(turns.get())
            .map_err(|error| execution_error(&self.backend, "dispatch", error.to_string()))?;
        Ok(ComputeDispatchReport {
            completed_turns: turns.get(),
            dispatch_milliseconds: elapsed.as_secs_f64() * 1_000.0,
            ..Default::default()
        })
    }

    fn read_outputs(
        &mut self,
        selection: &ComputeOutputSelection,
    ) -> Result<ComputeOutputSnapshot, ComputeExecutionError> {
        let (_, outputs) = self
            .session
            .read_outputs()
            .map_err(|error| execution_error(&self.backend, "read outputs", error.to_string()))?;
        output_snapshot(&self.program, selection, &outputs)
            .map_err(|detail| execution_error(&self.backend, "read outputs", detail))
    }
}

fn descriptor(
    id: &'static str,
    class: BackendClass,
    priority: u16,
    native: bool,
) -> ComputeBackendDescriptor {
    ComputeBackendDescriptor {
        id: BackendId::new(id).expect("static backend ID is valid"),
        class,
        priority,
        capabilities: ComputeBackendCapabilities {
            elementwise: true,
            fixed_shape: false,
            integrity_rejection: false,
            native,
            browser: false,
        },
    }
}

fn supports_elementwise(
    backend: &BackendId,
    program: &ComputeProgram,
) -> Result<(), ComputeBackendRejection> {
    if !matches!(program.kernel(), ComputeKernel::Elementwise(_)) {
        return Err(ComputeBackendRejection {
            backend: backend.clone(),
            reason: "backend currently supports elementwise programs only".into(),
        });
    }
    if program.elementwise_storage().is_none() {
        return Err(ComputeBackendRejection {
            backend: backend.clone(),
            reason: "elementwise program has no physical storage plan".into(),
        });
    }
    Ok(())
}

fn initializer_inputs(
    backend: &BackendId,
    program: &GpuProgram,
    initializers: &ComputeInitializerSet,
) -> Result<BTreeMap<String, Vec<f32>>, ComputeBackendError> {
    program
        .compute_program()
        .interface()
        .inputs
        .iter()
        .map(|port| {
            let value = initializers.get(port.id).ok_or_else(|| {
                backend_error(
                    backend,
                    "create session",
                    format!("input `{}` has no initializer", port.name),
                )
            })?;
            let value = port.normalize_value(value.clone()).map_err(|error| {
                backend_error(
                    backend,
                    "create session",
                    format!("input `{}` initializer is invalid: {error}", port.name),
                )
            })?;
            Ok((port.name.to_string(), compute_values(value)))
        })
        .collect()
}

fn normalized_update_inputs(
    backend: &BackendId,
    program: &GpuProgram,
    updates: &[ComputeInputUpdate],
) -> Result<BTreeMap<String, Vec<f32>>, ComputeExecutionError> {
    updates
        .iter()
        .map(|update| {
            let update = program
                .compute_program()
                .normalize_input_update(update.clone())
                .map_err(|error| execution_error(backend, "update inputs", error.to_string()))?;
            let port = program
                .compute_program()
                .interface()
                .input(update.port)
                .expect("normalized update names a declared input");
            Ok((port.name.to_string(), compute_values(update.value)))
        })
        .collect()
}

fn compute_values(value: ComputeValue) -> Vec<f32> {
    match value {
        ComputeValue::ScalarF32(value) => vec![value],
        ComputeValue::TensorF32 { values, .. } => values.to_vec(),
    }
}

fn output_snapshot(
    program: &GpuProgram,
    selection: &ComputeOutputSelection,
    outputs: &BTreeMap<String, Vec<f32>>,
) -> Result<ComputeOutputSnapshot, String> {
    let selected = |port: &ComputePort| match selection {
        ComputeOutputSelection::All => true,
        ComputeOutputSelection::Ports(ports) => ports.contains(&port.id),
    };
    let values = program
        .compute_program()
        .interface()
        .outputs
        .iter()
        .filter(|port| selected(port))
        .map(|port| {
            let values = outputs
                .get(port.name.as_ref())
                .ok_or_else(|| format!("backend did not return output `{}`", port.name))?;
            let value = if port.dimensions.is_empty() {
                let [value] = values.as_slice() else {
                    return Err(format!(
                        "scalar output `{}` returned {} elements",
                        port.name,
                        values.len()
                    ));
                };
                ComputeValue::ScalarF32(*value)
            } else {
                ComputeValue::TensorF32 {
                    dimensions: port.dimensions.clone(),
                    layout: TensorLayout::RowMajor,
                    values: Arc::from(values.clone()),
                }
            };
            Ok((port.id, value))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    Ok(ComputeOutputSnapshot { values })
}

fn backend_error(
    backend: &BackendId,
    operation: &'static str,
    detail: impl Into<Box<str>>,
) -> ComputeBackendError {
    ComputeBackendError {
        backend: backend.clone(),
        operation,
        detail: detail.into(),
    }
}

fn execution_error(
    backend: &BackendId,
    operation: &'static str,
    detail: impl Into<Box<str>>,
) -> ComputeExecutionError {
    ComputeExecutionError {
        backend: backend.clone(),
        operation,
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_compute::{BackendRequest, ComputePlatform};
    use mech_core::ComputePlacement;

    #[test]
    fn native_registry_has_real_cpu_and_wgpu_factories() {
        let registry = native_compute_backend_registry();
        assert!(
            registry
                .descriptors()
                .any(|descriptor| descriptor.id.as_str() == CPU_SCALAR_BACKEND)
        );
        assert!(
            registry
                .descriptors()
                .any(|descriptor| descriptor.id.as_str() == WGPU_BACKEND)
        );
    }

    #[test]
    fn cpu_request_does_not_select_the_gpu_factory() {
        let registry = native_compute_backend_registry();
        let program = crate::empty_compute_program();
        let Err(error) = registry.resolve(
            &BackendRequest::Cpu,
            ComputePlatform::Native,
            ComputePlacement::Compute,
            &program,
        ) else {
            panic!("an incomplete elementwise program must be rejected")
        };
        assert!(matches!(
            error,
            mech_compute::BackendRegistryError::NoCompatibleBackend { .. }
        ));
    }
}
