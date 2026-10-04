//! Canonical source-compilation configuration and immutable artifact products.

use crate::*;

#[derive(Debug, Clone)]
pub struct CompilerPlanningLimits {
    pub max_planning_steps: usize,
}

impl Default for CompilerPlanningLimits {
    fn default() -> Self {
        Self {
            max_planning_steps: 10_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompilerPlanningConfig {
    pub name: String,
    pub limits: CompilerPlanningLimits,
}

impl Default for CompilerPlanningConfig {
    fn default() -> Self {
        Self {
            name: "program".into(),
            limits: CompilerPlanningLimits::default(),
        }
    }
}

#[cfg(feature = "semantic-compiler")]
#[derive(Debug)]
pub struct ProgramCompilationProduct {
    artifact: ProgramArtifact,
    bytecode: Vec<u8>,
    source_dependencies: std::collections::BTreeMap<String, u64>,
    instruction_type_bindings: Vec<Option<mech_core::BoundCall>>,
    instruction_type_binding_requirements: Vec<bool>,
    instruction_memory_plans: Vec<Option<mech_core::CallMemoryPlan>>,
}

/// Immutable source-compilation product for hosts that immediately activate
/// an artifact and do not need a second durable bytecode representation.
#[cfg(feature = "semantic-compiler")]
#[derive(Debug)]
pub struct ProgramArtifactCompilationProduct {
    artifact: ProgramArtifact,
}

#[cfg(feature = "semantic-compiler")]
impl ProgramArtifactCompilationProduct {
    /// Wrap an already compiled canonical artifact for immediate activation.
    /// No planner cells or duplicate durable bytecode are retained.
    pub fn from_artifact(artifact: ProgramArtifact) -> Self {
        Self { artifact }
    }

    pub const fn artifact(&self) -> &ProgramArtifact {
        &self.artifact
    }

    pub fn into_artifact(self) -> ProgramArtifact {
        self.artifact
    }
}

/// Compiler-only instruction for preserving a source-declared custom send
/// operation in the canonical application requirement. It does not grant
/// authority or change interpreter execution.
#[cfg(feature = "semantic-compiler")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledResourceSendOperation {
    pub base_uri: String,
    pub path: Option<String>,
    pub operation: String,
}

#[cfg(feature = "semantic-compiler")]
impl ProgramCompilationProduct {
    /// Build the durable product directly from a canonical ProgramArtifact.
    /// Canonical artifacts already carry their operation contracts, schemas,
    /// memory declarations, inputs, outputs, and requirements in artifact sections.
    /// The decoded instruction stream is empty, as are its native binding sidecars.
    pub fn from_canonical_artifact(artifact: ProgramArtifact) -> MResult<Self> {
        let bytecode = encode_program_artifact_bytecode_v1(&artifact).map_err(|error| {
            MechError::new(
                ProgramArtifactCompilationError {
                    reason: format!("unable to encode canonical ProgramArtifact: {error:?}"),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        Ok(Self {
            artifact,
            bytecode,
            source_dependencies: Default::default(),
            instruction_type_bindings: Vec::new(),
            instruction_type_binding_requirements: Vec::new(),
            instruction_memory_plans: Vec::new(),
        })
    }

    pub const fn artifact(&self) -> &ProgramArtifact {
        &self.artifact
    }

    pub fn bytecode(&self) -> &[u8] {
        &self.bytecode
    }

    pub fn into_parts(self) -> (ProgramArtifact, Vec<u8>) {
        (self.artifact, self.bytecode)
    }

    /// Exact resolved source snapshots whose exports were embedded in this product.
    pub fn source_dependencies(&self) -> &std::collections::BTreeMap<String, u64> {
        &self.source_dependencies
    }

    /// Attach provenance collected by the same source-resolution pass that compiled the root.
    pub fn with_source_dependencies(
        mut self,
        dependencies: std::collections::BTreeMap<String, u64>,
    ) -> Self {
        self.source_dependencies = dependencies;
        self
    }

    pub fn instruction_type_bindings(&self) -> &[Option<mech_core::BoundCall>] {
        &self.instruction_type_bindings
    }

    pub fn instruction_type_binding_requirements(&self) -> &[bool] {
        &self.instruction_type_binding_requirements
    }

    pub fn instruction_memory_plans(&self) -> &[Option<mech_core::CallMemoryPlan>] {
        &self.instruction_memory_plans
    }

    pub fn into_native_parts(
        self,
    ) -> (
        ProgramArtifact,
        Vec<u8>,
        Vec<Option<mech_core::BoundCall>>,
        Vec<bool>,
        Vec<Option<mech_core::CallMemoryPlan>>,
    ) {
        (
            self.artifact,
            self.bytecode,
            self.instruction_type_bindings,
            self.instruction_type_binding_requirements,
            self.instruction_memory_plans,
        )
    }
}

#[cfg(feature = "semantic-compiler")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramArtifactCompilationError {
    pub reason: String,
}

#[cfg(feature = "semantic-compiler")]
impl MechErrorKind for ProgramArtifactCompilationError {
    fn name(&self) -> &str {
        "ProgramArtifactCompilationError"
    }

    fn message(&self) -> String {
        self.reason.clone()
    }
}
