#[cfg(feature = "native")]
use std::sync::OnceLock;
use std::{collections::BTreeMap, num::NonZeroU32, sync::Arc};

use web_time::Instant;

#[cfg(feature = "jit")]
use mech_compute::CPU_JIT_BACKEND;
use mech_compute::{
    BackendClass, BackendId, CPU_SCALAR_BACKEND, CPU_SIMD_BACKEND, ComputeBackendCapabilities,
    ComputeBackendDescriptor, ComputeBackendError, ComputeBackendFactory, ComputeBackendRejection,
    ComputeDispatchDisposition, ComputeDispatchReport, ComputeDispatchRequest, ComputeExecutable,
    ComputeExecutionError, ComputeFaultEvidence, ComputeInitializerSet, ComputeInputUpdate,
    ComputeKernel, ComputeOutputSelection, ComputeOutputSnapshot, ComputePort, ComputeProgram,
    ComputeSession, ComputeValue, TensorLayout,
};
#[cfg(feature = "native")]
use mech_compute::{ComputeBackendRegistry, WGPU_BACKEND};

#[cfg(feature = "jit")]
use crate::BatchedJitCpuSession;
#[cfg(feature = "native")]
use crate::BatchedResidentGpuSession;
#[cfg(feature = "native")]
use crate::ResidentGpuSession;
use crate::{
    BatchedCpuSession, BatchedExecutionError, BatchedIntegrityFault, BatchedSimdCpuSession,
    ElementwiseKernel, FixedShapeKernel, OwnedResidentCpuSession,
};

#[cfg(feature = "native")]
pub fn native_compute_backend_registry() -> Arc<ComputeBackendRegistry> {
    let mut registry = ComputeBackendRegistry::default();
    registry
        .register(Arc::new(CpuScalarBackendFactory::new()))
        .expect("static CPU backend ID is unique");
    registry
        .register(Arc::new(CpuSimdBackendFactory::new()))
        .expect("static SIMD backend ID is unique");
    #[cfg(feature = "jit")]
    registry
        .register(Arc::new(CpuJitBackendFactory::new()))
        .expect("static JIT backend ID is unique");
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
            descriptor: descriptor(
                CPU_SCALAR_BACKEND,
                BackendClass::Cpu,
                100,
                ComputeBackendCapabilities {
                    elementwise: true,
                    fixed_shape: true,
                    integrity_rejection: true,
                    native: true,
                    browser: true,
                },
            ),
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
        supports_common_program(&self.descriptor.id, program)
    }

    fn compile(
        &self,
        program: &ComputeProgram,
    ) -> Result<Box<dyn ComputeExecutable>, ComputeBackendError> {
        let program = match program.kernel() {
            ComputeKernel::Elementwise(_) => CpuScalarProgram::Elementwise(
                ElementwiseKernel::from_compute_program(program).map_err(|error| {
                    backend_error(
                        &self.descriptor.id,
                        "compile",
                        format!("compute lowering failed: {error}"),
                    )
                })?,
            ),
            ComputeKernel::FixedShape(_) => CpuScalarProgram::Fixed(
                FixedShapeKernel::from_compute_program(program).map_err(|error| {
                    backend_error(
                        &self.descriptor.id,
                        "compile",
                        format!("fixed-shape lowering failed: {error}"),
                    )
                })?,
            ),
        };
        Ok(Box::new(CpuScalarExecutable {
            backend: self.descriptor.id.clone(),
            program,
        }))
    }
}

struct CpuScalarExecutable {
    backend: BackendId,
    program: CpuScalarProgram,
}

enum CpuScalarProgram {
    Elementwise(ElementwiseKernel),
    Fixed(FixedShapeKernel),
}

impl ComputeExecutable for CpuScalarExecutable {
    fn create_session(
        &self,
        initializers: &ComputeInitializerSet,
    ) -> Result<Box<dyn ComputeSession>, ComputeBackendError> {
        match &self.program {
            CpuScalarProgram::Elementwise(program) => {
                let inputs =
                    initializer_inputs(&self.backend, program.compute_program(), initializers)?;
                let session = program.clone().into_cpu(&inputs).map_err(|error| {
                    backend_error(&self.backend, "create session", error.to_string())
                })?;
                Ok(Box::new(CpuScalarSession {
                    backend: self.backend.clone(),
                    session,
                }))
            }
            CpuScalarProgram::Fixed(program) => {
                let inputs =
                    initializer_inputs(&self.backend, program.compute_program(), initializers)?;
                let session = program.prepare_cpu(&inputs).map_err(|error| {
                    backend_error(&self.backend, "create session", error.to_string())
                })?;
                Ok(Box::new(FixedScalarSession {
                    backend: self.backend.clone(),
                    program: program.clone(),
                    session,
                }))
            }
        }
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
        let inputs = normalized_update_inputs(
            &self.backend,
            self.session.program_ref().compute_program(),
            updates,
        )?;
        self.session
            .update_inputs(&inputs)
            .map_err(|error| execution_error(&self.backend, "update inputs", error.to_string()))
    }

    fn dispatch(
        &mut self,
        request: &ComputeDispatchRequest,
    ) -> Result<ComputeDispatchReport, ComputeExecutionError> {
        let turns = request.turns;
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

struct FixedScalarSession {
    backend: BackendId,
    program: FixedShapeKernel,
    session: BatchedCpuSession,
}

impl ComputeSession for FixedScalarSession {
    fn update_inputs(
        &mut self,
        updates: &[ComputeInputUpdate],
    ) -> Result<(), ComputeExecutionError> {
        let inputs =
            normalized_update_inputs(&self.backend, self.program.compute_program(), updates)?;
        self.session
            .update_inputs(&inputs)
            .map_err(|error| execution_error(&self.backend, "update inputs", error.to_string()))
    }

    fn dispatch(
        &mut self,
        request: &ComputeDispatchRequest,
    ) -> Result<ComputeDispatchReport, ComputeExecutionError> {
        let turns = request.turns;
        let started = Instant::now();
        let attempted_before = self.session.attempted_turns();
        let result = self.session.dispatch_turns(turns.get());
        fixed_dispatch_report(
            &self.backend,
            turns,
            started,
            result,
            self.session
                .attempted_turns()
                .saturating_sub(attempted_before),
            self.session.fault_count(),
            self.session.last_fault(),
        )
    }

    fn read_outputs(
        &mut self,
        selection: &ComputeOutputSelection,
    ) -> Result<ComputeOutputSnapshot, ComputeExecutionError> {
        fixed_output_snapshot(&self.program, selection, self.session.state())
            .map_err(|detail| execution_error(&self.backend, "read outputs", detail))
    }
}

#[derive(Debug)]
pub struct CpuSimdBackendFactory {
    descriptor: ComputeBackendDescriptor,
}

impl CpuSimdBackendFactory {
    pub fn new() -> Self {
        Self {
            descriptor: fixed_cpu_descriptor(CPU_SIMD_BACKEND, 200),
        }
    }
}

impl Default for CpuSimdBackendFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputeBackendFactory for CpuSimdBackendFactory {
    fn descriptor(&self) -> &ComputeBackendDescriptor {
        &self.descriptor
    }

    fn supports(&self, program: &ComputeProgram) -> Result<(), ComputeBackendRejection> {
        supports_fixed_shape(&self.descriptor.id, program)
    }

    fn compile(
        &self,
        program: &ComputeProgram,
    ) -> Result<Box<dyn ComputeExecutable>, ComputeBackendError> {
        Ok(Box::new(FixedExecutable {
            backend: self.descriptor.id.clone(),
            program: FixedShapeKernel::from_compute_program(program).map_err(|error| {
                backend_error(
                    &self.descriptor.id,
                    "compile",
                    format!("fixed-shape lowering failed: {error}"),
                )
            })?,
            implementation: FixedCpuImplementation::Simd,
        }))
    }
}

#[cfg(feature = "jit")]
#[derive(Debug)]
pub struct CpuJitBackendFactory {
    descriptor: ComputeBackendDescriptor,
}

#[cfg(feature = "jit")]
impl CpuJitBackendFactory {
    pub fn new() -> Self {
        Self {
            descriptor: fixed_cpu_descriptor(CPU_JIT_BACKEND, 300),
        }
    }
}

#[cfg(feature = "jit")]
impl Default for CpuJitBackendFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "jit")]
impl ComputeBackendFactory for CpuJitBackendFactory {
    fn descriptor(&self) -> &ComputeBackendDescriptor {
        &self.descriptor
    }

    fn supports(&self, program: &ComputeProgram) -> Result<(), ComputeBackendRejection> {
        supports_fixed_shape(&self.descriptor.id, program)
    }

    fn compile(
        &self,
        program: &ComputeProgram,
    ) -> Result<Box<dyn ComputeExecutable>, ComputeBackendError> {
        Ok(Box::new(FixedExecutable {
            backend: self.descriptor.id.clone(),
            program: FixedShapeKernel::from_compute_program(program).map_err(|error| {
                backend_error(
                    &self.descriptor.id,
                    "compile",
                    format!("fixed-shape lowering failed: {error}"),
                )
            })?,
            implementation: FixedCpuImplementation::Jit,
        }))
    }
}

enum FixedCpuImplementation {
    Simd,
    #[cfg(feature = "jit")]
    Jit,
}

struct FixedExecutable {
    backend: BackendId,
    program: FixedShapeKernel,
    implementation: FixedCpuImplementation,
}

impl ComputeExecutable for FixedExecutable {
    fn create_session(
        &self,
        initializers: &ComputeInitializerSet,
    ) -> Result<Box<dyn ComputeSession>, ComputeBackendError> {
        let inputs =
            initializer_inputs(&self.backend, self.program.compute_program(), initializers)?;
        match self.implementation {
            FixedCpuImplementation::Simd => Ok(Box::new(FixedSimdSession {
                backend: self.backend.clone(),
                program: self.program.clone(),
                session: self.program.prepare_simd_cpu(&inputs).map_err(|error| {
                    backend_error(&self.backend, "create session", error.to_string())
                })?,
            })),
            #[cfg(feature = "jit")]
            FixedCpuImplementation::Jit => Ok(Box::new(FixedJitSession {
                backend: self.backend.clone(),
                program: self.program.clone(),
                session: self.program.prepare_jit_cpu(&inputs).map_err(|error| {
                    backend_error(&self.backend, "create session", error.to_string())
                })?,
            })),
        }
    }
}

struct FixedSimdSession {
    backend: BackendId,
    program: FixedShapeKernel,
    session: BatchedSimdCpuSession,
}

impl ComputeSession for FixedSimdSession {
    fn update_inputs(
        &mut self,
        updates: &[ComputeInputUpdate],
    ) -> Result<(), ComputeExecutionError> {
        let inputs =
            normalized_update_inputs(&self.backend, self.program.compute_program(), updates)?;
        self.session
            .update_inputs(&inputs)
            .map_err(|error| execution_error(&self.backend, "update inputs", error.to_string()))
    }

    fn dispatch(
        &mut self,
        request: &ComputeDispatchRequest,
    ) -> Result<ComputeDispatchReport, ComputeExecutionError> {
        let turns = request.turns;
        let started = Instant::now();
        let attempted_before = self.session.attempted_turns();
        let result = self.session.dispatch_turns(turns.get());
        fixed_dispatch_report(
            &self.backend,
            turns,
            started,
            result,
            self.session
                .attempted_turns()
                .saturating_sub(attempted_before),
            self.session.fault_count(),
            self.session.last_fault(),
        )
    }

    fn read_outputs(
        &mut self,
        selection: &ComputeOutputSelection,
    ) -> Result<ComputeOutputSnapshot, ComputeExecutionError> {
        fixed_output_snapshot(&self.program, selection, self.session.state())
            .map_err(|detail| execution_error(&self.backend, "read outputs", detail))
    }
}

#[cfg(feature = "jit")]
struct FixedJitSession {
    backend: BackendId,
    program: FixedShapeKernel,
    session: BatchedJitCpuSession,
}

#[cfg(feature = "jit")]
impl ComputeSession for FixedJitSession {
    fn update_inputs(
        &mut self,
        updates: &[ComputeInputUpdate],
    ) -> Result<(), ComputeExecutionError> {
        let inputs =
            normalized_update_inputs(&self.backend, self.program.compute_program(), updates)?;
        self.session
            .update_inputs(&inputs)
            .map_err(|error| execution_error(&self.backend, "update inputs", error.to_string()))
    }

    fn dispatch(
        &mut self,
        request: &ComputeDispatchRequest,
    ) -> Result<ComputeDispatchReport, ComputeExecutionError> {
        let turns = request.turns;
        let started = Instant::now();
        let attempted_before = self.session.attempted_turns();
        let result = self.session.dispatch_turns(turns.get());
        fixed_dispatch_report(
            &self.backend,
            turns,
            started,
            result,
            self.session
                .attempted_turns()
                .saturating_sub(attempted_before),
            self.session.fault_count(),
            self.session.last_fault(),
        )
    }

    fn read_outputs(
        &mut self,
        selection: &ComputeOutputSelection,
    ) -> Result<ComputeOutputSnapshot, ComputeExecutionError> {
        fixed_output_snapshot(&self.program, selection, self.session.state())
            .map_err(|detail| execution_error(&self.backend, "read outputs", detail))
    }
}

#[cfg(feature = "native")]
pub struct WgpuBackendFactory {
    descriptor: ComputeBackendDescriptor,
    availability: OnceLock<Result<(), Box<str>>>,
}

#[cfg(feature = "native")]
impl std::fmt::Debug for WgpuBackendFactory {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WgpuBackendFactory")
            .field("descriptor", &self.descriptor)
            .field("availability", &self.availability.get())
            .finish()
    }
}

#[cfg(feature = "native")]
impl WgpuBackendFactory {
    pub fn new() -> Self {
        Self {
            descriptor: descriptor(
                WGPU_BACKEND,
                BackendClass::Gpu,
                400,
                ComputeBackendCapabilities {
                    elementwise: true,
                    fixed_shape: true,
                    integrity_rejection: true,
                    native: true,
                    browser: false,
                },
            ),
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

#[cfg(feature = "native")]
impl Default for WgpuBackendFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "native")]
impl ComputeBackendFactory for WgpuBackendFactory {
    fn descriptor(&self) -> &ComputeBackendDescriptor {
        &self.descriptor
    }

    fn supports(&self, program: &ComputeProgram) -> Result<(), ComputeBackendRejection> {
        supports_common_program(&self.descriptor.id, program)?;
        self.available().map_err(|reason| ComputeBackendRejection {
            backend: self.descriptor.id.clone(),
            reason,
        })
    }

    fn compile(
        &self,
        program: &ComputeProgram,
    ) -> Result<Box<dyn ComputeExecutable>, ComputeBackendError> {
        let program = match program.kernel() {
            ComputeKernel::Elementwise(_) => WgpuProgram::Elementwise(
                ElementwiseKernel::from_compute_program(program).map_err(|error| {
                    backend_error(
                        &self.descriptor.id,
                        "compile",
                        format!("compute lowering failed: {error}"),
                    )
                })?,
            ),
            ComputeKernel::FixedShape(_) => WgpuProgram::Fixed(
                FixedShapeKernel::from_compute_program(program).map_err(|error| {
                    backend_error(
                        &self.descriptor.id,
                        "compile",
                        format!("fixed-shape lowering failed: {error}"),
                    )
                })?,
            ),
        };
        Ok(Box::new(WgpuExecutable {
            backend: self.descriptor.id.clone(),
            program,
        }))
    }
}

#[cfg(feature = "native")]
struct WgpuExecutable {
    backend: BackendId,
    program: WgpuProgram,
}

#[cfg(feature = "native")]
enum WgpuProgram {
    Elementwise(ElementwiseKernel),
    Fixed(FixedShapeKernel),
}

#[cfg(feature = "native")]
impl ComputeExecutable for WgpuExecutable {
    fn create_session(
        &self,
        initializers: &ComputeInitializerSet,
    ) -> Result<Box<dyn ComputeSession>, ComputeBackendError> {
        match &self.program {
            WgpuProgram::Elementwise(program) => {
                let inputs =
                    initializer_inputs(&self.backend, program.compute_program(), initializers)?;
                let session = program.prepare_resident(&inputs).map_err(|error| {
                    backend_error(&self.backend, "create session", error.to_string())
                })?;
                Ok(Box::new(WgpuSession {
                    backend: self.backend.clone(),
                    program: program.clone(),
                    session,
                }))
            }
            WgpuProgram::Fixed(program) => {
                let inputs =
                    initializer_inputs(&self.backend, program.compute_program(), initializers)?;
                let session = program.prepare_resident(&inputs).map_err(|error| {
                    backend_error(&self.backend, "create session", error.to_string())
                })?;
                Ok(Box::new(FixedWgpuSession {
                    backend: self.backend.clone(),
                    program: program.clone(),
                    session,
                }))
            }
        }
    }
}

#[cfg(feature = "native")]
struct WgpuSession {
    backend: BackendId,
    program: ElementwiseKernel,
    session: ResidentGpuSession,
}

#[cfg(feature = "native")]
impl ComputeSession for WgpuSession {
    fn update_inputs(
        &mut self,
        updates: &[ComputeInputUpdate],
    ) -> Result<(), ComputeExecutionError> {
        let inputs =
            normalized_update_inputs(&self.backend, self.program.compute_program(), updates)?;
        for (name, values) in inputs {
            self.session.update_input(&name, &values).map_err(|error| {
                execution_error(&self.backend, "update inputs", error.to_string())
            })?;
        }
        Ok(())
    }

    fn dispatch(
        &mut self,
        request: &ComputeDispatchRequest,
    ) -> Result<ComputeDispatchReport, ComputeExecutionError> {
        let turns = request.turns;
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

#[cfg(feature = "native")]
struct FixedWgpuSession {
    backend: BackendId,
    program: FixedShapeKernel,
    session: BatchedResidentGpuSession,
}

#[cfg(feature = "native")]
impl ComputeSession for FixedWgpuSession {
    fn update_inputs(
        &mut self,
        updates: &[ComputeInputUpdate],
    ) -> Result<(), ComputeExecutionError> {
        let inputs =
            normalized_update_inputs(&self.backend, self.program.compute_program(), updates)?;
        self.session
            .update_inputs(&self.program, &inputs)
            .map_err(|error| execution_error(&self.backend, "update inputs", error.to_string()))
    }

    fn dispatch(
        &mut self,
        request: &ComputeDispatchRequest,
    ) -> Result<ComputeDispatchReport, ComputeExecutionError> {
        let turns = request.turns;
        let started = Instant::now();
        let attempted_before = self.session.attempted_turns();
        let result = self.session.dispatch_turns(turns.get()).map(|_| ());
        fixed_dispatch_report(
            &self.backend,
            turns,
            started,
            result,
            self.session
                .attempted_turns()
                .saturating_sub(attempted_before),
            self.session.fault_count(),
            self.session.last_fault(),
        )
    }

    fn read_outputs(
        &mut self,
        selection: &ComputeOutputSelection,
    ) -> Result<ComputeOutputSnapshot, ComputeExecutionError> {
        let (state, snapshot_selection) = match selection {
            ComputeOutputSelection::Samples { ports, instance } => {
                let slots = self
                    .program
                    .compute_program()
                    .interface()
                    .outputs
                    .iter()
                    .filter(|port| ports.contains(&port.id))
                    .map(|port| port.slot)
                    .collect();
                (
                    self.session.read_published_sample(&slots, *instance),
                    ComputeOutputSelection::Samples {
                        ports: ports.clone(),
                        instance: 0,
                    },
                )
            }
            ComputeOutputSelection::All | ComputeOutputSelection::Ports(_) => (
                self.session.read_published_state().map(|(_, state)| state),
                selection.clone(),
            ),
        };
        let state = state
            .map_err(|error| execution_error(&self.backend, "read outputs", error.to_string()))?;
        fixed_output_snapshot(&self.program, &snapshot_selection, &state)
            .map_err(|detail| execution_error(&self.backend, "read outputs", detail))
    }
}

fn descriptor(
    id: &'static str,
    class: BackendClass,
    priority: u16,
    capabilities: ComputeBackendCapabilities,
) -> ComputeBackendDescriptor {
    ComputeBackendDescriptor {
        id: BackendId::new(id).expect("static backend ID is valid"),
        class,
        priority,
        capabilities,
    }
}

fn fixed_cpu_descriptor(id: &'static str, priority: u16) -> ComputeBackendDescriptor {
    descriptor(
        id,
        BackendClass::Cpu,
        priority,
        ComputeBackendCapabilities {
            fixed_shape: true,
            integrity_rejection: true,
            native: true,
            ..Default::default()
        },
    )
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

fn supports_fixed_shape(
    backend: &BackendId,
    program: &ComputeProgram,
) -> Result<(), ComputeBackendRejection> {
    if !matches!(program.kernel(), ComputeKernel::FixedShape(_)) {
        return Err(ComputeBackendRejection {
            backend: backend.clone(),
            reason: "backend requires a fixed-shape scalar kernel".into(),
        });
    }
    if program.fixed_shape_storage().is_none() {
        return Err(ComputeBackendRejection {
            backend: backend.clone(),
            reason: "fixed-shape program has no resident storage plan".into(),
        });
    }
    Ok(())
}

fn supports_common_program(
    backend: &BackendId,
    program: &ComputeProgram,
) -> Result<(), ComputeBackendRejection> {
    match program.kernel() {
        ComputeKernel::Elementwise(_) => supports_elementwise(backend, program),
        ComputeKernel::FixedShape(_) => supports_fixed_shape(backend, program),
    }
}

fn initializer_inputs(
    backend: &BackendId,
    program: &ComputeProgram,
    initializers: &ComputeInitializerSet,
) -> Result<BTreeMap<String, Vec<f32>>, ComputeBackendError> {
    program
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
    program: &ComputeProgram,
    updates: &[ComputeInputUpdate],
) -> Result<BTreeMap<String, Vec<f32>>, ComputeExecutionError> {
    updates
        .iter()
        .map(|update| {
            let update = program
                .normalize_input_update(update.clone())
                .map_err(|error| execution_error(backend, "update inputs", error.to_string()))?;
            let port = program
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
    program: &ElementwiseKernel,
    selection: &ComputeOutputSelection,
    outputs: &BTreeMap<String, Vec<f32>>,
) -> Result<ComputeOutputSnapshot, String> {
    let selected = |port: &ComputePort| match selection {
        ComputeOutputSelection::All => true,
        ComputeOutputSelection::Ports(ports) | ComputeOutputSelection::Samples { ports, .. } => {
            ports.contains(&port.id)
        }
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

fn fixed_output_snapshot(
    program: &FixedShapeKernel,
    selection: &ComputeOutputSelection,
    state: &BTreeMap<mech_core::CellSlotId, Vec<f32>>,
) -> Result<ComputeOutputSnapshot, String> {
    let selected = |port: &ComputePort| match selection {
        ComputeOutputSelection::All => true,
        ComputeOutputSelection::Ports(ports) | ComputeOutputSelection::Samples { ports, .. } => {
            ports.contains(&port.id)
        }
    };
    let sample_instance = match selection {
        ComputeOutputSelection::Samples { instance, .. } => Some(*instance as usize),
        ComputeOutputSelection::All | ComputeOutputSelection::Ports(_) => None,
    };
    let values = program
        .compute_program()
        .interface()
        .outputs
        .iter()
        .filter(|port| selected(port))
        .map(|port| {
            let physical = state
                .get(&port.slot)
                .ok_or_else(|| format!("backend did not publish output `{}`", port.name))?;
            let instances = if sample_instance.is_some() {
                1
            } else {
                program.instances() as usize
            };
            let elements = port
                .elements()
                .map_err(|error| format!("output `{}` has an invalid shape: {error}", port.name))?;
            let physical = if let Some(instance) = sample_instance {
                let start = instance.checked_mul(elements).ok_or_else(|| {
                    format!(
                        "sample instance {instance} overflows output `{}`",
                        port.name
                    )
                })?;
                let end = start.checked_add(elements).ok_or_else(|| {
                    format!(
                        "sample instance {instance} overflows output `{}`",
                        port.name
                    )
                })?;
                physical.get(start..end).ok_or_else(|| {
                    format!(
                        "sample instance {instance} exceeds output `{}` with {} elements",
                        port.name,
                        physical.len(),
                    )
                })?
            } else {
                physical.as_slice()
            };
            let logical = fixed_column_major_to_row_major(physical, &port.dimensions, instances)?;
            let value = if port.dimensions.is_empty() && instances == 1 {
                let [value] = logical.as_slice() else {
                    return Err(format!(
                        "scalar output `{}` returned {} elements",
                        port.name,
                        logical.len()
                    ));
                };
                ComputeValue::ScalarF32(*value)
            } else {
                let mut dimensions = Vec::from(port.dimensions.as_ref());
                if instances > 1 {
                    dimensions.insert(0, instances as u64);
                }
                ComputeValue::TensorF32 {
                    dimensions: dimensions.into_boxed_slice(),
                    layout: TensorLayout::RowMajor,
                    values: logical.into(),
                }
            };
            Ok((port.id, value))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    Ok(ComputeOutputSnapshot { values })
}

fn fixed_column_major_to_row_major(
    physical: &[f32],
    dimensions: &[u64],
    instances: usize,
) -> Result<Vec<f32>, String> {
    let rows = dimensions.first().copied().unwrap_or(1) as usize;
    let columns = dimensions.get(1).copied().unwrap_or(1) as usize;
    let elements = rows
        .checked_mul(columns)
        .ok_or_else(|| "fixed-shape output dimensions overflow".to_owned())?;
    if physical.len() != elements.saturating_mul(instances) {
        return Err(format!(
            "fixed-shape output returned {} elements, expected {}",
            physical.len(),
            elements.saturating_mul(instances)
        ));
    }
    if dimensions.len() < 2 {
        return Ok(physical.to_vec());
    }
    let mut logical = vec![0.0; physical.len()];
    for instance in 0..instances {
        let base = instance * elements;
        for row in 0..rows {
            for column in 0..columns {
                logical[base + row * columns + column] = physical[base + row + column * rows];
            }
        }
    }
    Ok(logical)
}

fn fixed_dispatch_report(
    backend: &BackendId,
    turns: NonZeroU32,
    started: Instant,
    result: Result<(), BatchedExecutionError>,
    attempted_turns: u64,
    fault_count: u64,
    last_fault: Option<&BatchedIntegrityFault>,
) -> Result<ComputeDispatchReport, ComputeExecutionError> {
    match result {
        Ok(()) => Ok(ComputeDispatchReport {
            disposition: ComputeDispatchDisposition::Completed,
            completed_turns: turns.get(),
            dispatch_milliseconds: started.elapsed().as_secs_f64() * 1_000.0,
            fault_count,
            last_fault: last_fault.map(compute_fault_evidence),
        }),
        Err(BatchedExecutionError::Integrity(fault)) => Ok(ComputeDispatchReport {
            disposition: ComputeDispatchDisposition::Rejected,
            completed_turns: u32::try_from(attempted_turns.saturating_sub(1))
                .unwrap_or(u32::MAX)
                .min(turns.get()),
            dispatch_milliseconds: started.elapsed().as_secs_f64() * 1_000.0,
            fault_count,
            last_fault: Some(compute_fault_evidence(&fault)),
        }),
        Err(error) => Err(execution_error(backend, "dispatch", error.to_string())),
    }
}

fn compute_fault_evidence(fault: &BatchedIntegrityFault) -> ComputeFaultEvidence {
    ComputeFaultEvidence {
        attempted_turn: u128::from(fault.attempted_turn),
        constraint: fault.constraint_name.clone(),
        detail: format!("candidate rejected at batch instance {}", fault.instance).into_boxed_str(),
    }
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
        state_advanced: false,
    }
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use super::*;
    use mech_compute::{BackendRequest, ComputePlatform};
    use mech_core::{ComputePlacement, IntegrityConstraintId};

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

    #[test]
    fn integrity_report_counts_turns_published_before_a_batched_rejection() {
        let fault = BatchedIntegrityFault {
            attempted_turn: 2,
            instance: 0,
            constraint: IntegrityConstraintId::new(0),
            constraint_name: "finite".into(),
        };
        let report = fixed_dispatch_report(
            &BackendId::new(CPU_SCALAR_BACKEND).unwrap(),
            NonZeroU32::new(3).unwrap(),
            Instant::now(),
            Err(BatchedExecutionError::Integrity(fault.clone())),
            2,
            1,
            Some(&fault),
        )
        .unwrap();

        assert_eq!(report.completed_turns, 1);
        assert_eq!(report.fault_count, 1);
        assert_eq!(report.last_fault.unwrap().attempted_turn, 2);
    }
}
