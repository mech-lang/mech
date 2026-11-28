//! The checked lowering and its immutable preparation inputs travel together.
use super::prepare::PreparedNativeProgram;
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::compilation_inputs::CompilationInputs;
use crate::native_artifact::compiler::backend;
use mech_core::FunctionCatalog;
use mech_engine::ProgramArtifact;
use mech_engine::native_artifact::{
    NativeProgramArtifactPlan, lower_native_program_artifact_with_definition_table_limit,
};

pub(super) struct LoweredProgramCompilation<'a> {
    artifact: &'a ProgramArtifact,
    catalog: &'a FunctionCatalog,
    inputs: &'a CompilationInputs,
    plan: NativeProgramArtifactPlan,
}

impl<'a> LoweredProgramCompilation<'a> {
    pub(super) fn lower(
        artifact: &'a ProgramArtifact,
        catalog: &'a FunctionCatalog,
        max_definition_table_bytes: u64,
        inputs: &'a CompilationInputs,
    ) -> Result<Self, NativeCompileError> {
        let plan = lower_native_program_artifact_with_definition_table_limit(
            artifact,
            catalog,
            max_definition_table_bytes,
        )
        .map_err(backend)?;
        Ok(Self {
            artifact,
            catalog,
            inputs,
            plan,
        })
    }

    pub(super) fn plan(&self) -> &NativeProgramArtifactPlan {
        &self.plan
    }

    pub(super) fn prepare(self) -> Result<PreparedNativeProgram, NativeCompileError> {
        PreparedNativeProgram::prepare_with_inputs(
            self.artifact,
            self.catalog,
            self.plan,
            self.inputs,
        )
        .map_err(backend)
    }
}
