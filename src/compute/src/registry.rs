use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    num::NonZeroU32,
    sync::Arc,
};

use mech_core::ComputePlacement;

use crate::{ComputeInputUpdate, ComputeKernel, ComputePortId, ComputeProgram, ComputeValue};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BackendId(Box<str>);

impl BackendId {
    pub fn new(value: impl Into<Box<str>>) -> Result<Self, BackendRegistryError> {
        let value = value.into();
        if value.is_empty()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(BackendRegistryError::InvalidBackendId(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BackendId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

pub const CPU_SCALAR_BACKEND: &str = "cpu-scalar";
pub const CPU_SIMD_BACKEND: &str = "cpu-simd";
pub const CPU_JIT_BACKEND: &str = "cpu-jit";
pub const WGPU_BACKEND: &str = "wgpu";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendClass {
    Cpu,
    Gpu,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComputeBackendCapabilities {
    pub elementwise: bool,
    pub fixed_shape: bool,
    pub integrity_rejection: bool,
    pub native: bool,
    pub browser: bool,
}

impl ComputeBackendCapabilities {
    pub fn supports_kernel(self, kernel: &ComputeKernel) -> bool {
        match kernel {
            ComputeKernel::Elementwise(_) => self.elementwise,
            ComputeKernel::FixedShape(_) => self.fixed_shape,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeBackendDescriptor {
    pub id: BackendId,
    pub class: BackendClass,
    pub priority: u16,
    pub capabilities: ComputeBackendCapabilities,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComputePlatform {
    Native,
    Browser,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendRequest {
    Auto,
    Cpu,
    Gpu,
    Exact(BackendId),
}

impl BackendRequest {
    pub fn parse(value: &str) -> Result<Self, BackendRegistryError> {
        match value {
            "auto" => Ok(Self::Auto),
            "cpu" => Ok(Self::Cpu),
            "gpu" => Ok(Self::Gpu),
            "cpu-scalar" | "cpu-simd" | "cpu-jit" | "wgpu" => {
                Ok(Self::Exact(BackendId::new(value)?))
            }
            _ => Err(BackendRegistryError::UnknownBackendRequest(
                value.to_owned().into_boxed_str(),
            )),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComputeInitializerSet {
    values: BTreeMap<ComputePortId, ComputeValue>,
}

impl ComputeInitializerSet {
    pub fn new(values: BTreeMap<ComputePortId, ComputeValue>) -> Self {
        Self { values }
    }

    pub fn get(&self, port: ComputePortId) -> Option<&ComputeValue> {
        self.values.get(&port)
    }

    pub fn iter(&self) -> impl Iterator<Item = (ComputePortId, &ComputeValue)> {
        self.values.iter().map(|(port, value)| (*port, value))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ComputeOutputSelection {
    #[default]
    All,
    Ports(BTreeSet<ComputePortId>),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComputeOutputSnapshot {
    pub values: BTreeMap<ComputePortId, ComputeValue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeFaultEvidence {
    pub attempted_turn: u64,
    pub constraint: Box<str>,
    pub detail: Box<str>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComputeDispatchReport {
    pub completed_turns: u32,
    pub dispatch_milliseconds: f64,
    pub fault_count: u64,
    pub last_fault: Option<ComputeFaultEvidence>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeBackendRejection {
    pub backend: BackendId,
    pub reason: Box<str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeBackendError {
    pub backend: BackendId,
    pub operation: &'static str,
    pub detail: Box<str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeExecutionError {
    pub backend: BackendId,
    pub operation: &'static str,
    pub detail: Box<str>,
}

pub trait ComputeBackendFactory {
    fn descriptor(&self) -> &ComputeBackendDescriptor;

    fn supports(&self, program: &ComputeProgram) -> Result<(), ComputeBackendRejection>;

    fn compile(
        &self,
        program: &ComputeProgram,
    ) -> Result<Box<dyn ComputeExecutable>, ComputeBackendError>;
}

pub trait ComputeExecutable {
    fn create_session(
        &self,
        initializers: &ComputeInitializerSet,
    ) -> Result<Box<dyn ComputeSession>, ComputeBackendError>;
}

pub trait ComputeSession {
    fn update_inputs(
        &mut self,
        updates: &[ComputeInputUpdate],
    ) -> Result<(), ComputeExecutionError>;

    fn dispatch(
        &mut self,
        turns: NonZeroU32,
    ) -> Result<ComputeDispatchReport, ComputeExecutionError>;

    fn read_outputs(
        &mut self,
        selection: &ComputeOutputSelection,
    ) -> Result<ComputeOutputSnapshot, ComputeExecutionError>;
}

#[derive(Default)]
pub struct ComputeBackendRegistry {
    factories: BTreeMap<BackendId, Arc<dyn ComputeBackendFactory>>,
}

impl fmt::Debug for ComputeBackendRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputeBackendRegistry")
            .field("backends", &self.factories.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl ComputeBackendRegistry {
    pub fn register(
        &mut self,
        factory: Arc<dyn ComputeBackendFactory>,
    ) -> Result<(), BackendRegistryError> {
        let id = factory.descriptor().id.clone();
        if self.factories.contains_key(&id) {
            return Err(BackendRegistryError::DuplicateBackend(id));
        }
        self.factories.insert(id, factory);
        Ok(())
    }

    pub fn descriptor(&self, id: &BackendId) -> Option<&ComputeBackendDescriptor> {
        self.factories.get(id).map(|factory| factory.descriptor())
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &ComputeBackendDescriptor> {
        self.factories.values().map(|factory| factory.descriptor())
    }

    pub fn resolve(
        &self,
        request: &BackendRequest,
        platform: ComputePlatform,
        placement: ComputePlacement,
        program: &ComputeProgram,
    ) -> Result<Arc<dyn ComputeBackendFactory>, BackendRegistryError> {
        let candidates = preference_order(request, platform)?;
        let exact = matches!(request, BackendRequest::Exact(_));
        let mut rejections = Vec::new();
        for id in candidates {
            let Some(factory) = self.factories.get(&id) else {
                rejections.push(ComputeBackendRejection {
                    backend: id,
                    reason: "backend is not registered on this platform".into(),
                });
                continue;
            };
            let descriptor = factory.descriptor();
            if !platform_compatible(descriptor, platform) {
                rejections.push(ComputeBackendRejection {
                    backend: descriptor.id.clone(),
                    reason: "backend is not available on the selected platform".into(),
                });
                continue;
            }
            if !placement_compatible(descriptor.class, placement) {
                let error = ComputeBackendRejection {
                    backend: descriptor.id.clone(),
                    reason: format!(
                        "backend class {:?} is incompatible with region placement {placement:?}",
                        descriptor.class
                    )
                    .into_boxed_str(),
                };
                if exact {
                    return Err(BackendRegistryError::ExplicitBackendRejected(error));
                }
                rejections.push(error);
                continue;
            }
            if !descriptor.capabilities.supports_kernel(program.kernel()) {
                let error = ComputeBackendRejection {
                    backend: descriptor.id.clone(),
                    reason: "backend descriptor does not support this compute kernel".into(),
                };
                if exact {
                    return Err(BackendRegistryError::ExplicitBackendRejected(error));
                }
                rejections.push(error);
                continue;
            }
            match factory.supports(program) {
                Ok(()) => return Ok(Arc::clone(factory)),
                Err(error) if exact => {
                    return Err(BackendRegistryError::ExplicitBackendRejected(error));
                }
                Err(error) => rejections.push(error),
            }
        }
        Err(BackendRegistryError::NoCompatibleBackend { rejections })
    }
}

fn preference_order(
    request: &BackendRequest,
    platform: ComputePlatform,
) -> Result<Vec<BackendId>, BackendRegistryError> {
    let ids = match (request, platform) {
        (BackendRequest::Auto, ComputePlatform::Native) => vec![
            backend_id(WGPU_BACKEND),
            backend_id(CPU_JIT_BACKEND),
            backend_id(CPU_SIMD_BACKEND),
            backend_id(CPU_SCALAR_BACKEND),
        ],
        (BackendRequest::Auto, ComputePlatform::Browser) => {
            vec![
                backend_id(WGPU_BACKEND),
                backend_id(CPU_SIMD_BACKEND),
                backend_id(CPU_SCALAR_BACKEND),
            ]
        }
        (BackendRequest::Cpu, ComputePlatform::Native) => {
            vec![
                backend_id(CPU_JIT_BACKEND),
                backend_id(CPU_SIMD_BACKEND),
                backend_id(CPU_SCALAR_BACKEND),
            ]
        }
        (BackendRequest::Cpu, ComputePlatform::Browser) => {
            vec![backend_id(CPU_SIMD_BACKEND), backend_id(CPU_SCALAR_BACKEND)]
        }
        (BackendRequest::Gpu, _) => vec![backend_id(WGPU_BACKEND)],
        (BackendRequest::Exact(id), _) => vec![id.clone()],
    };
    Ok(ids)
}

fn backend_id(value: &'static str) -> BackendId {
    BackendId(value.into())
}

fn platform_compatible(descriptor: &ComputeBackendDescriptor, platform: ComputePlatform) -> bool {
    match platform {
        ComputePlatform::Native => descriptor.capabilities.native,
        ComputePlatform::Browser => descriptor.capabilities.browser,
    }
}

fn placement_compatible(class: BackendClass, placement: ComputePlacement) -> bool {
    match placement {
        ComputePlacement::Compute => true,
        ComputePlacement::Cpu => class == BackendClass::Cpu,
        ComputePlacement::Gpu => class == BackendClass::Gpu,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendRegistryError {
    InvalidBackendId(Box<str>),
    UnknownBackendRequest(Box<str>),
    DuplicateBackend(BackendId),
    ExplicitBackendRejected(ComputeBackendRejection),
    NoCompatibleBackend {
        rejections: Vec<ComputeBackendRejection>,
    },
}

impl fmt::Display for BackendRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for BackendRegistryError {}

impl fmt::Display for ComputeBackendRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "backend `{}` rejected the program: {}",
            self.backend, self.reason
        )
    }
}

impl Error for ComputeBackendRejection {}

impl fmt::Display for ComputeBackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "backend `{}` failed to {}: {}",
            self.backend, self.operation, self.detail
        )
    }
}

impl Error for ComputeBackendError {}

impl fmt::Display for ComputeExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "backend `{}` failed to {}: {}",
            self.backend, self.operation, self.detail
        )
    }
}

impl Error for ComputeExecutionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ComputeKernel, ComputePhysicalPlan, ComputeRegionInterface, ElementwiseIr};

    struct TestFactory {
        descriptor: ComputeBackendDescriptor,
        rejection: Option<&'static str>,
    }

    impl ComputeBackendFactory for TestFactory {
        fn descriptor(&self) -> &ComputeBackendDescriptor {
            &self.descriptor
        }

        fn supports(&self, _program: &ComputeProgram) -> Result<(), ComputeBackendRejection> {
            match self.rejection {
                Some(reason) => Err(ComputeBackendRejection {
                    backend: self.descriptor.id.clone(),
                    reason: reason.into(),
                }),
                None => Ok(()),
            }
        }

        fn compile(
            &self,
            _program: &ComputeProgram,
        ) -> Result<Box<dyn ComputeExecutable>, ComputeBackendError> {
            unreachable!("resolution tests do not compile")
        }
    }

    fn factory(
        id: &str,
        class: BackendClass,
        native: bool,
        browser: bool,
        rejection: Option<&'static str>,
    ) -> Arc<dyn ComputeBackendFactory> {
        Arc::new(TestFactory {
            descriptor: ComputeBackendDescriptor {
                id: BackendId::new(id).unwrap(),
                class,
                priority: 0,
                capabilities: ComputeBackendCapabilities {
                    elementwise: true,
                    fixed_shape: true,
                    integrity_rejection: true,
                    native,
                    browser,
                },
            },
            rejection,
        })
    }

    fn program() -> ComputeProgram {
        ComputeProgram::new(
            ComputeRegionInterface::default(),
            ComputePhysicalPlan::default(),
            ComputeKernel::Elementwise(ElementwiseIr::default()),
        )
    }

    fn registry() -> ComputeBackendRegistry {
        let mut registry = ComputeBackendRegistry::default();
        registry
            .register(factory("cpu-scalar", BackendClass::Cpu, true, true, None))
            .unwrap();
        registry
            .register(factory("cpu-simd", BackendClass::Cpu, true, true, None))
            .unwrap();
        registry
            .register(factory("cpu-jit", BackendClass::Cpu, true, false, None))
            .unwrap();
        registry
            .register(factory("wgpu", BackendClass::Gpu, true, true, None))
            .unwrap();
        registry
    }

    #[test]
    fn selector_policy_is_deterministic_across_native_and_browser() {
        let registry = registry();
        assert_eq!(
            registry
                .resolve(
                    &BackendRequest::Auto,
                    ComputePlatform::Native,
                    ComputePlacement::Compute,
                    &program(),
                )
                .unwrap()
                .descriptor()
                .id
                .as_str(),
            WGPU_BACKEND
        );
        assert_eq!(
            registry
                .resolve(
                    &BackendRequest::Cpu,
                    ComputePlatform::Native,
                    ComputePlacement::Compute,
                    &program(),
                )
                .unwrap()
                .descriptor()
                .id
                .as_str(),
            CPU_JIT_BACKEND
        );
        assert_eq!(
            registry
                .resolve(
                    &BackendRequest::Cpu,
                    ComputePlatform::Browser,
                    ComputePlacement::Compute,
                    &program(),
                )
                .unwrap()
                .descriptor()
                .id
                .as_str(),
            CPU_SIMD_BACKEND
        );
    }

    #[test]
    fn hard_placement_never_silently_crosses_backend_class() {
        let registry = registry();
        let error = match registry.resolve(
            &BackendRequest::Exact(backend_id(WGPU_BACKEND)),
            ComputePlatform::Native,
            ComputePlacement::Cpu,
            &program(),
        ) {
            Ok(_) => panic!("a GPU backend must not satisfy a hard CPU placement"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            BackendRegistryError::ExplicitBackendRejected(_)
        ));

        assert_eq!(
            registry
                .resolve(
                    &BackendRequest::Auto,
                    ComputePlatform::Native,
                    ComputePlacement::Cpu,
                    &program(),
                )
                .unwrap()
                .descriptor()
                .id
                .as_str(),
            CPU_JIT_BACKEND
        );
    }

    #[test]
    fn auto_falls_back_only_after_backend_admission_rejection() {
        let mut registry = registry();
        registry.factories.insert(
            backend_id(WGPU_BACKEND),
            factory(
                "wgpu",
                BackendClass::Gpu,
                true,
                true,
                Some("adapter unavailable"),
            ),
        );
        assert_eq!(
            registry
                .resolve(
                    &BackendRequest::Auto,
                    ComputePlatform::Native,
                    ComputePlacement::Compute,
                    &program(),
                )
                .unwrap()
                .descriptor()
                .id
                .as_str(),
            CPU_JIT_BACKEND
        );
    }

    #[test]
    fn exact_backend_rejection_never_falls_back() {
        let mut registry = registry();
        registry.factories.insert(
            backend_id(WGPU_BACKEND),
            factory(
                "wgpu",
                BackendClass::Gpu,
                true,
                true,
                Some("adapter unavailable"),
            ),
        );
        assert!(matches!(
            registry.resolve(
                &BackendRequest::Exact(backend_id(WGPU_BACKEND)),
                ComputePlatform::Native,
                ComputePlacement::Compute,
                &program(),
            ),
            Err(BackendRegistryError::ExplicitBackendRejected(_))
        ));
    }
}
