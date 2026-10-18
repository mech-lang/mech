//! Same-artifact metadata for generated ABI5, not an evaluator.
use super::{NativeValueSlot, iteration_abi::NativeIterationVisit};
use mech_core::{FunctionCatalog, SchemaTable, Value};
use mech_engine::{
    ProgramArtifact,
    native_artifact::{
        NativeIterationAction, NativePreparedLibraryCall, NativeProgramArtifactPlan,
        prepare_native_program_library_call,
    },
};
use std::sync::Arc;

#[derive(Debug)]
pub struct NativeProgramDescriptor {
    pub(super) plan: NativeProgramArtifactPlan,
    pub(super) schemas: Arc<SchemaTable>,
    pub(super) calls: Box<[Option<NativePreparedLibraryCall>]>,
    pub(super) types: Box<[NativeValueSlot]>,
    pub(super) visits: Box<[NativeIterationVisit]>,
    pub(super) constant_bytes: Box<[u64]>,
    pub(super) scalar_bytes: u64,
    pub(super) stack: u64,
    pub(super) code: u64,
    pub(super) identity: [u8; 32],
}
impl NativeProgramDescriptor {
    pub(super) fn prepare(
        artifact: &ProgramArtifact,
        catalog: &FunctionCatalog,
        plan: NativeProgramArtifactPlan,
        stack: u64,
        code: u64,
    ) -> Result<Self, String> {
        if artifact.revision() != plan.program_revision() {
            return Err("mixed canonical revision mismatch".into());
        }
        plan.validate_current_visits().map_err(str::to_owned)?;
        let completion = (plan.outputs().len() as u64)
            .checked_mul(64)
            .and_then(|n| n.checked_add((plan.iterations().len() as u64).checked_mul(8)?))
            .and_then(|n| n.checked_add(256))
            .ok_or("program completion overflow")?;
        let visits = super::iteration_abi::control_visits(
            plan.blocks(),
            plan.iterations().iter().map(Into::into),
            completion,
        )
        .map_err(str::to_owned)?
        .into();
        let calls = plan
            .blocks()
            .iter()
            .map(|block| match block.action() {
                NativeIterationAction::LibraryCall { template, .. } => {
                    prepare_native_program_library_call(artifact, template, catalog)
                        .map(Some)
                        .map_err(|e| e.to_string())
                }
                _ => Ok(None),
            })
            .collect::<Result<Box<[_]>, _>>()?;
        let types = plan
            .values()
            .iter()
            .chain(plan.outputs().iter().map(|o| o.ty()))
            .map(|ty| NativeValueSlot {
                schema: ty.schema(),
                key: ty.schema_key(),
            })
            .collect();
        let constant_bytes = plan
            .constants()
            .iter()
            .map(|c| {
                c.constant()
                    .memory_budget_required_bytes()
                    .map_err(|e| format!("constant ownership: {e:?}"))?
                    .checked_add(Value::memory_budget_claim_metadata_bytes())
                    .and_then(|n| n.checked_add(4096))
                    .ok_or_else(|| "constant admission overflows".to_owned())
            })
            .collect::<Result<Box<[_]>, _>>()?;
        // Shared schema finalization avoids a schema clone, but ownership
        // admission still includes the complete schema context. Reserve a
        // conservative fixed scalar construction/import envelope BEFORE use.
        let scalar_bytes = artifact
            .schemas()
            .clone_allocation_bound_bytes()
            .and_then(|n| n.checked_add(Value::canonical_owner_allocation_bytes()))
            .and_then(|n| n.checked_add(Value::memory_budget_claim_metadata_bytes()))
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| n.checked_add(4096))
            .ok_or("scalar ownership bound overflows")?;
        let identity = super::program_identity::typed_ir_sha256(&plan);
        Ok(Self {
            plan,
            schemas: Arc::new(artifact.schemas().clone()),
            calls,
            types,
            visits,
            constant_bytes,
            scalar_bytes,
            stack,
            code,
            identity,
        })
    }
    pub fn plan(&self) -> &NativeProgramArtifactPlan {
        &self.plan
    }
    pub const fn stack_bytes(&self) -> u64 {
        self.stack
    }
    pub const fn code_bytes(&self) -> u64 {
        self.code
    }
    pub fn typed_ir_sha256(&self) -> &[u8; 32] {
        &self.identity
    }
}
impl super::iteration_runtime::VisitPlan for NativeProgramDescriptor {
    fn entry(&self) -> u32 {
        self.plan.entry()
    }
    fn visits(&self) -> &[NativeIterationVisit] {
        &self.visits
    }
    fn iteration_count(&self) -> usize {
        self.plan.iterations().len()
    }
    fn iteration(&self, index: usize) -> mech_engine::native_artifact::NativeControlLoopRef<'_> {
        self.plan.iterations()[index].as_ref()
    }
}
