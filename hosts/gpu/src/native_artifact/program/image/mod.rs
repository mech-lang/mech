//! Same-artifact metadata for generated ABI16, not an evaluator.
use crate::native_artifact::iteration_abi::NativeIterationVisit;
use mech_core::SchemaTable;
use mech_engine::native_artifact::{NativeIterationAction, NativeProgramArtifactPlan};

#[path = "decode/mod.rs"]
pub(in crate::native_artifact) mod decode;

#[derive(Debug)]
pub struct NativeProgramDescriptor {
    pub(in crate::native_artifact) metadata:
        crate::native_artifact::program::prepare::ProgramMetadata,
    pub(in crate::native_artifact) footprint: NativeCodeFootprint,
    pub(in crate::native_artifact) identity: [u8; 32],
}
#[derive(Clone, Copy, Debug)]
pub(in crate::native_artifact) struct NativeCodeFootprint {
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
        struct Schemas<'a>(
            #[serde(
                serialize_with = "crate::native_artifact::program::image::decode::serialize_schemas"
            )]
            &'a SchemaTable,
        );
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
impl NativeProgramDescriptor {
    pub(in crate::native_artifact) fn from_prepared(
        prepared: crate::native_artifact::program::prepare::PreparedNativeProgram,
        footprint: NativeCodeFootprint,
    ) -> Self {
        let metadata = prepared.metadata();
        let identity = crate::native_artifact::program::image::identity::typed_ir_sha256(
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
    pub(in crate::native_artifact) fn metadata(
        &self,
    ) -> &crate::native_artifact::program::prepare::ProgramMetadata {
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
    pub(in crate::native_artifact) fn query_stage(
        &self,
        collection: u32,
        ordinal: u32,
    ) -> Result<(), &'static str> {
        let step = crate::native_artifact::program::prepare::query_stages::query_step(
            &self.metadata.plan,
            collection,
            ordinal,
        )?;
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
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "string_map_factory_tests.rs"]
pub(in crate::native_artifact::program::image) mod string_map_factory_tests;
impl crate::native_artifact::iteration_runtime::VisitPlan for NativeProgramDescriptor {
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

#[cfg(feature = "canonical-native-program-runtime")]
#[path = "identity.rs"]
pub(in crate::native_artifact) mod identity;
#[cfg(feature = "canonical-native-program-runtime")]
#[path = "packing.rs"]
pub(in crate::native_artifact) mod packing;
