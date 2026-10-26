//! Same-artifact metadata for generated ABI16, not an evaluator.
use super::iteration_abi::NativeIterationVisit;
use mech_core::SchemaTable;
use mech_engine::native_artifact::{
    NativeIterationAction, NativePreparedLibraryCall, NativeProgramArtifactPlan,
};

#[path = "program_descriptor_image.rs"]
pub(super) mod image;

#[path = "program_retirement.rs"]
mod retirement;
pub(super) use retirement::{RetirementKind, RetirementTables};

#[derive(Debug)]
pub struct NativeProgramDescriptor {
    pub(super) metadata: super::program::prepare::ProgramMetadata,
    pub(super) footprint: NativeCodeFootprint,
    pub(super) identity: [u8; 32],
}
#[derive(Clone, Copy, Debug)]
pub(super) struct NativeCodeFootprint {
    pub stack_bytes: u64,
    pub code_bytes: u64,
}
// Public Serialize is a compatibility boundary, independent of Rust nesting.
impl serde::Serialize for NativeProgramDescriptor {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut fields = serializer.serialize_struct("NativeProgramDescriptor", 25)?;
        fields.serialize_field("plan", &self.metadata.plan)?;
        #[derive(serde::Serialize)]
        struct Schemas<'a>(#[serde(serialize_with = "image::serialize_schemas")] &'a SchemaTable);
        fields.serialize_field("schemas", &Schemas(&self.metadata.schemas))?;
        fields.serialize_field("calls", &self.metadata.calls)?;
        fields.serialize_field("types", &self.metadata.types)?;
        fields.serialize_field("visits", &self.metadata.visits)?;
        fields.serialize_field("retirements", &self.metadata.retirements)?;
        fields.serialize_field("constant_bytes", &self.metadata.constant_bytes)?;
        fields.serialize_field("scalar_bytes", &self.metadata.scalar_bytes)?;
        fields.serialize_field(
            "observation_schema_bytes",
            &self.metadata.observation_schema_bytes,
        )?;
        fields.serialize_field("max_library_inputs", &self.metadata.max_library_inputs)?;
        fields.serialize_field("window_regions", &self.metadata.window_regions)?;
        fields.serialize_field("string_map_regions", &self.metadata.string_map_regions)?;
        fields.serialize_field("scalar_aggregates", &self.metadata.scalar_aggregates)?;
        fields.serialize_field("map_owner_words", &self.metadata.map_owner_words)?;
        fields.serialize_field("frozen_maps", &self.metadata.frozen_maps)?;
        fields.serialize_field("map_admission", &self.metadata.map_admission)?;
        fields.serialize_field("parallel", &self.metadata.parallel)?;
        fields.serialize_field("borrowed_strings", &self.metadata.borrowed_strings)?;
        fields.serialize_field("closed_tails", &self.metadata.closed_tails)?;
        fields.serialize_field("selected_joins", &self.metadata.selected_joins)?;
        fields.serialize_field("line_matrices", &self.metadata.line_matrices)?;
        fields.serialize_field("prefix", &self.metadata.prefix)?;
        fields.serialize_field("stack", &self.footprint.stack_bytes)?;
        fields.serialize_field("code", &self.footprint.code_bytes)?;
        fields.serialize_field("identity", &self.identity)?;
        fields.end()
    }
}
pub(super) fn constant_admission_total(bounds: &[u64]) -> Result<u64, String> {
    bounds
        .iter()
        .try_fold(0u64, |sum, &bound| sum.checked_add(bound))
        .ok_or_else(|| "constant admission total overflows".to_owned())
}
impl NativeProgramDescriptor {
    pub(super) fn from_prepared(
        prepared: super::program::prepare::PreparedNativeProgram,
        footprint: NativeCodeFootprint,
    ) -> Self {
        let metadata = prepared.metadata();
        let identity = super::program_identity::typed_ir_sha256(
            prepared.plan(),
            prepared.complete_frontiers(),
            &metadata.selected_joins.regions,
            &metadata.line_matrices.regions,
            metadata.map_owner_words.as_ref(),
            &metadata.frozen_maps,
            &metadata.string_map_regions,
        );
        Self {
            metadata: prepared.into_metadata(),
            footprint,
            identity,
        }
    }
    pub(super) fn metadata(&self) -> &super::program::prepare::ProgramMetadata {
        &self.metadata
    }
    pub fn plan(&self) -> &NativeProgramArtifactPlan {
        &self.metadata.plan
    }
    /// Immutable canonical types shared by this build's native/resource metadata.
    pub fn schemas(&self) -> &SchemaTable {
        &self.metadata.schemas
    }
    /// Fixed-header association only, not a new semantic or visit authority.
    /// prepare checked full selected-template equality against the actual
    /// artifact/factory once; plan and calls cannot mutate after construction.
    /// The caller funds the existing query-stage header work before lookup.
    pub(super) fn query_stage(&self, collection: u32, ordinal: u32) -> Result<(), &'static str> {
        let step = query_step(&self.metadata.plan, collection, ordinal)?;
        match self
            .metadata
            .plan
            .blocks()
            .get(step.site() as usize)
            .map(|b| b.action())
        {
            Some(NativeIterationAction::HistogramQueryStep {
                collection: id,
                step: index,
            }) if *id == collection && *index == ordinal => {}
            _ => return Err("histogram query stage association differs"),
        }
        if self
            .metadata
            .calls
            .get(step.site() as usize)
            .ok_or("histogram query stage outside calls")?
            .is_some()
            != step.template().is_some()
        {
            return Err("histogram query selected stage presence differs");
        }
        Ok(())
    }
    pub const fn stack_bytes(&self) -> u64 {
        self.footprint.stack_bytes
    }
    pub const fn code_bytes(&self) -> u64 {
        self.footprint.code_bytes
    }
    pub fn typed_ir_sha256(&self) -> &[u8; 32] {
        &self.identity
    }
}
fn query_step(
    plan: &NativeProgramArtifactPlan,
    collection: u32,
    ordinal: u32,
) -> Result<&mech_engine::native_artifact::NativeHistogramQueryStep, &'static str> {
    let Some(mech_engine::native_artifact::NativeCollectionResult::GuardedHistogramQuery(sink)) =
        plan.collections()
            .get(collection as usize)
            .map(|c| c.result())
    else {
        return Err("histogram query stage lacks selected proof");
    };
    sink.steps()
        .get(ordinal as usize)
        .ok_or("histogram query step outside proof")
}
/// Compiler-only full immutable association proof. This runs for JIT, Object
/// preparation and linked reconstruction, after canonical current-visit proof
/// and SAME actual artifact/catalog/factory preparation of every selected call.
pub(super) fn validate_query_stages(
    plan: &NativeProgramArtifactPlan,
    calls: &[Option<NativePreparedLibraryCall>],
) -> Result<(), &'static str> {
    if calls.len() != plan.blocks().len() {
        return Err("histogram query call lane length differs");
    }
    for (site, block) in plan.blocks().iter().enumerate() {
        let NativeIterationAction::HistogramQueryStep {
            collection,
            step: ordinal,
        } = block.action()
        else {
            continue;
        };
        let step = query_step(plan, *collection, *ordinal)?;
        if step.site() as usize != site {
            return Err("histogram query original site differs");
        }
        match (step.template(), calls[site].as_ref()) {
            (Some(template), Some(selected)) if selected.template() == template => {}
            (None, None) => {}
            _ => return Err("histogram query selected template differs"),
        }
    }
    Ok(())
}
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_query_stage_tests.rs"]
mod query_stage_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_string_map_factory_tests.rs"]
mod string_map_factory_tests;
impl super::iteration_runtime::VisitPlan for NativeProgramDescriptor {
    fn entry(&self) -> u32 {
        self.metadata.plan.entry()
    }
    fn visits(&self) -> &[NativeIterationVisit] {
        &self.metadata.visits
    }
    fn iteration_count(&self) -> usize {
        self.metadata.plan.iterations().len()
    }
    fn iteration(&self, index: usize) -> mech_engine::native_artifact::NativeControlLoopRef<'_> {
        self.metadata.plan.iterations()[index].as_ref()
    }
}
