//! Rust embedding for self-contained, fixed-shape numerical Mech kernels.
//!
//! Source contains one `@compute` section, typed input defaults, persistent
//! state, and optional integrity predicates. The builder explicitly selects
//! live inputs and exported state. It performs source compilation and backend
//! preparation once; each session has independent input and state buffers.
//!
//! This API executes numerical turns. Host drivers, ROS transport, document
//! rendering, and coordinator effects remain separate host facilities.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use mech_core::{CellSlotId, ComputePlacement, MechCode, Program, SectionElement};
use mech_runtime::RuntimeBuilder;

#[cfg(feature = "aot")]
use crate::{
    BatchedAotCpuArtifact, BatchedAotCpuSession, BatchedAotSimdCpuArtifact,
    BatchedAotSimdCpuSession,
};
use crate::{
    BatchedCpuSession, BatchedExecutionError, BatchedSimdCpuSession, ComputeLowerer,
    FixedShapeKernel, GpuAdmissionError,
};

/// Execution strategy for the same fixed-shape numerical source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Backend {
    /// Evaluate lowered scalar instructions on one CPU thread.
    Scalar,
    /// Evaluate lowered instructions in four-instance SIMD groups on one thread.
    Simd,
    /// Emit and load a scalar Cranelift native library.
    #[cfg(feature = "aot")]
    Aot,
    /// Emit and load a four-instance SIMD Cranelift native library.
    /// The inferred instance count must be divisible by four.
    #[cfg(feature = "aot")]
    AotSimd,
}

#[derive(Debug)]
pub enum Error {
    Source(String),
    Admission(GpuAdmissionError),
    Execution(BatchedExecutionError),
    DuplicateInput(String),
    EmptyInput(String),
    UnknownInput(String),
    DuplicateExport(String),
    InvalidExport(String),
    UnknownState(String),
    ArtifactDirectoryRequiresAot,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(message) => write!(f, "Mech source compilation failed: {message}"),
            Self::Admission(error) => write!(f, "kernel admission failed: {error:?}"),
            Self::Execution(error) => write!(f, "{error}"),
            Self::DuplicateInput(name) => write!(f, "input `{name}` was supplied more than once"),
            Self::EmptyInput(name) => write!(f, "input `{name}` has no values"),
            Self::UnknownInput(name) => write!(f, "`{name}` is not a live input of this kernel"),
            Self::DuplicateExport(name) => write!(f, "state `{name}` was exported more than once"),
            Self::InvalidExport(name) => {
                write!(f, "export `{name}` must name persistent kernel state")
            }
            Self::UnknownState(name) => write!(f, "state `{name}` is not exported by this kernel"),
            Self::ArtifactDirectoryRequiresAot => {
                write!(f, "an artifact directory requires an AOT backend")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Execution(error) => Some(error),
            _ => None,
        }
    }
}

impl From<BatchedExecutionError> for Error {
    fn from(error: BatchedExecutionError) -> Self {
        Self::Execution(error)
    }
}

/// Configuration collected before source compilation.
pub struct KernelBuilder {
    source: String,
    inputs: Vec<(String, Vec<f32>)>,
    exports: Vec<String>,
    artifact_directory: Option<PathBuf>,
}

impl KernelBuilder {
    /// Make a source binding live and supply its initial values.
    ///
    /// Its source declaration determines the per-instance type and shape.
    /// Supply one value to broadcast, or one value per instance. For matrices,
    /// flatten each instance in column-major order, then concatenate instances.
    /// Non-singleton input extents must agree; the batch size is fixed at compile.
    pub fn input(mut self, name: impl Into<String>, values: impl Into<Vec<f32>>) -> Self {
        self.inputs.push((name.into(), values.into()));
        self
    }

    /// Expose a persistent (`~`) source binding for named, read-only state access.
    pub fn export(mut self, name: impl Into<String>) -> Self {
        self.exports.push(name.into());
        self
    }

    /// Select the directory for emitted AOT objects and dynamic libraries.
    /// Otherwise the backend uses its normal AOT cache directory.
    pub fn artifact_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.artifact_directory = Some(directory.into());
        self
    }

    /// Compile source, validate the interface, and prepare the selected backend.
    /// AOT emits and loads a native library here, before any session starts.
    pub fn compile(self, backend: Backend) -> Result<Kernel, Error> {
        if self.artifact_directory.is_some() && matches!(backend, Backend::Scalar | Backend::Simd) {
            return Err(Error::ArtifactDirectoryRequiresAot);
        }
        let mut inputs = BTreeMap::new();
        for (name, values) in self.inputs {
            if values.is_empty() {
                return Err(Error::EmptyInput(name));
            }
            if inputs.insert(name.clone(), values).is_some() {
                return Err(Error::DuplicateInput(name));
            }
        }
        let mut exports = BTreeSet::new();
        for name in self.exports {
            if !exports.insert(name.clone()) {
                return Err(Error::DuplicateExport(name));
            }
        }
        let tree = mech_syntax::parse(self.source.trim())
            .map_err(|error| Error::Source(format!("{error:?}")))?;
        validate_source_boundary(&tree)?;
        let artifact = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_native_plan_catalog())
            .build_compiler()
            .map_err(|error| Error::Source(format!("{error:?}")))?
            .compile_tree_artifact_with_interface(
                &tree,
                &BTreeMap::new(),
                &inputs.keys().cloned().collect(),
                &exports.iter().map(String::as_str).collect::<Vec<_>>(),
            )
            .map_err(|error| Error::Source(format!("{error:?}")))?
            .into_artifact();
        let program = ComputeLowerer
            .compile_broadcast(&artifact, &inputs)
            .map_err(Error::Admission)?;
        let input_widths = program
            .inputs()
            .map(|(name, width)| (name.to_owned(), width))
            .collect::<BTreeMap<_, _>>();
        for name in inputs.keys() {
            if !input_widths.contains_key(name) {
                return Err(Error::UnknownInput(name.clone()));
            }
        }
        let state_widths = program.state_layout().collect::<BTreeMap<_, _>>();
        let bindings = artifact.interactive_symbol_bindings().collect::<Vec<_>>();
        let mut states = BTreeMap::new();
        for name in exports {
            let binding = bindings
                .iter()
                .find(|binding| binding.lexical_name == name)
                .ok_or_else(|| Error::InvalidExport(name.clone()))?;
            let width = state_widths
                .get(&binding.storage)
                .ok_or_else(|| Error::InvalidExport(name.clone()))?;
            states.insert(name, (binding.storage, *width));
        }
        let compiled = match backend {
            Backend::Scalar => Compiled::Scalar,
            Backend::Simd => Compiled::Simd,
            #[cfg(feature = "aot")]
            Backend::Aot => Compiled::Aot(match self.artifact_directory {
                Some(directory) => program.compile_aot_cpu_to(directory)?,
                None => program.compile_aot_cpu()?,
            }),
            #[cfg(feature = "aot")]
            Backend::AotSimd => Compiled::AotSimd(match self.artifact_directory {
                Some(directory) => program.compile_aot_simd_cpu_to(directory)?,
                None => program.compile_aot_simd_cpu()?,
            }),
        };
        Ok(Kernel {
            program,
            compiled,
            initial_inputs: inputs,
            interface: Arc::new(Interface {
                input_widths,
                states,
            }),
        })
    }
}

// The lowerer operates on the whole artifact. Reject mixed coordinator/kernel
// documents instead of silently executing their ordinary sections as kernel work.
fn validate_source_boundary(tree: &Program) -> Result<(), Error> {
    let mut regions = 0;
    for section in &tree.body.sections {
        match mech_engine::section_compute_placement(section)
            .map_err(|error| Error::Source(format!("{error:?}")))?
        {
            Some(ComputePlacement::Compute) => regions += 1,
            Some(_) => {
                return Err(Error::Source(
                    "use a backend-neutral @compute section and select the backend in Rust".into(),
                ));
            }
            None => {
                if section.elements.iter().any(|element| {
                    !matches!(element,
                        SectionElement::MechCode(code) if code.iter().all(|(code, comment)|
                            matches!(code, MechCode::Import(_)) && comment.is_none())
                    )
                }) {
                    return Err(Error::Source(
                        "kernel source may contain only imports outside its @compute section; put defaults and numerical work inside that section".into(),
                    ));
                }
            }
        }
    }
    if regions != 1 {
        return Err(Error::Source(format!(
            "kernel source requires exactly one @compute section, found {regions}"
        )));
    }
    Ok(())
}

struct Interface {
    input_widths: BTreeMap<String, usize>,
    states: BTreeMap<String, (CellSlotId, usize)>,
}

enum Compiled {
    Scalar,
    Simd,
    #[cfg(feature = "aot")]
    Aot(BatchedAotCpuArtifact),
    #[cfg(feature = "aot")]
    AotSimd(BatchedAotSimdCpuArtifact),
}

/// An immutable compiled kernel. Start any number of independent sessions.
pub struct Kernel {
    program: FixedShapeKernel,
    compiled: Compiled,
    initial_inputs: BTreeMap<String, Vec<f32>>,
    interface: Arc<Interface>,
}

impl Kernel {
    pub fn from_source(source: impl Into<String>) -> KernelBuilder {
        KernelBuilder {
            source: source.into(),
            inputs: Vec::new(),
            exports: Vec::new(),
            artifact_directory: None,
        }
    }

    pub fn instances(&self) -> u32 {
        self.program.instances()
    }

    /// Saved library path for AOT backends; absent for instruction evaluators.
    /// This library contains the native turn function. Session allocation,
    /// named bindings, and publication remain in the Rust host library.
    pub fn library_path(&self) -> Option<&Path> {
        match &self.compiled {
            Compiled::Scalar | Compiled::Simd => None,
            #[cfg(feature = "aot")]
            Compiled::Aot(artifact) => Some(artifact.path()),
            #[cfg(feature = "aot")]
            Compiled::AotSimd(artifact) => Some(artifact.path()),
        }
    }

    /// Allocate a fresh session from source state and configured input values.
    /// Does not recompile or emit another library. A session may outlive this kernel.
    pub fn start(&self) -> Result<Session, Error> {
        let backend = match &self.compiled {
            Compiled::Scalar => Execution::Scalar(self.program.prepare_cpu(&self.initial_inputs)?),
            Compiled::Simd => Execution::Simd(self.program.prepare_simd_cpu(&self.initial_inputs)?),
            #[cfg(feature = "aot")]
            Compiled::Aot(artifact) => Execution::Aot(artifact.prepare(&self.initial_inputs)?),
            #[cfg(feature = "aot")]
            Compiled::AotSimd(artifact) => {
                Execution::AotSimd(artifact.prepare(&self.initial_inputs)?)
            }
        };
        Ok(Session {
            backend,
            interface: self.interface.clone(),
            instances: self.instances(),
        })
    }
}

enum Execution {
    Scalar(BatchedCpuSession),
    Simd(BatchedSimdCpuSession),
    #[cfg(feature = "aot")]
    Aot(BatchedAotCpuSession),
    #[cfg(feature = "aot")]
    AotSimd(BatchedAotSimdCpuSession),
}

/// Mutable resident numerical state and inputs for one kernel instance batch.
pub struct Session {
    backend: Execution,
    interface: Arc<Interface>,
    instances: u32,
}

macro_rules! with_session {
    ($execution:expr, $session:ident => $body:expr) => {
        match $execution {
            Execution::Scalar($session) => $body,
            Execution::Simd($session) => $body,
            #[cfg(feature = "aot")]
            Execution::Aot($session) => $body,
            #[cfg(feature = "aot")]
            Execution::AotSimd($session) => $body,
        }
    };
}

impl Session {
    /// Apply a partial packet of named inputs, then attempt one checked turn.
    ///
    /// Unknown/duplicate names or invalid lengths reject the entire packet
    /// before modifying any input or attempting execution. Omitted inputs keep
    /// their previous values. If a source integrity predicate fails, all
    /// published state remains unchanged, but the valid input packet remains
    /// bound. Supply corrected inputs before retrying. Source checks are never
    /// disabled by this API; a source without predicates has no integrity checks.
    pub fn turn<I, N, V>(&mut self, updates: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = (N, V)>,
        N: AsRef<str>,
        V: AsRef<[f32]>,
    {
        let mut packet = BTreeMap::new();
        for (name, values) in updates {
            let name = name.as_ref();
            let values = values.as_ref();
            let width = *self
                .interface
                .input_widths
                .get(name)
                .ok_or_else(|| Error::UnknownInput(name.to_owned()))?;
            let batch = width * self.instances as usize;
            if values.len() != width && values.len() != batch {
                return Err(BatchedExecutionError::InputLength {
                    name: name.to_owned(),
                    expected_single: width,
                    expected_batch: batch,
                    actual: values.len(),
                }
                .into());
            }
            if packet.insert(name.to_owned(), values.to_vec()).is_some() {
                return Err(Error::DuplicateInput(name.to_owned()));
            }
        }
        with_session!(&mut self.backend, session => session.update_inputs(&packet))?;
        self.advance()
    }

    /// Attempt one checked turn using the currently bound inputs.
    pub fn advance(&mut self) -> Result<(), Error> {
        with_session!(&mut self.backend, session => session.dispatch_turns(1))?;
        Ok(())
    }

    /// Borrow published state by its exported source name, with no state copy.
    /// Values are instance-major, with column-major matrix components per instance.
    /// The borrow prevents another mutable turn while the slice is in use.
    pub fn state(&self, name: &str) -> Result<&[f32], Error> {
        let (slot, _) = self
            .interface
            .states
            .get(name)
            .ok_or_else(|| Error::UnknownState(name.to_owned()))?;
        with_session!(&self.backend, session => session.state())
            .get(slot)
            .map(Vec::as_slice)
            .ok_or_else(|| Error::UnknownState(name.to_owned()))
    }

    /// Number of f32 components in one instance of the named state.
    pub fn state_width(&self, name: &str) -> Result<usize, Error> {
        self.interface
            .states
            .get(name)
            .map(|(_, width)| *width)
            .ok_or_else(|| Error::UnknownState(name.to_owned()))
    }

    pub fn instances(&self) -> u32 {
        self.instances
    }
}
