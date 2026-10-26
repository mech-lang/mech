//! Immutable execution metadata, never a capability or native-entry proof.
use super::{
    CanonicalNativeResourceAdmission, CanonicalNativeResourceCompletion,
    CanonicalNativeResourceLimits, CanonicalNativeResourcePlan, CanonicalNativeResourcePrepared,
    CanonicalNativeResourceRunError, CanonicalNativeResourceTextChunk, CanonicalNativeResourceTurn,
    ResourceMetadataView,
};
use crate::runtime::MechRuntime;
use mech_core::{MResult, SchemaTable};
use mech_engine::{ApplicationRequirementTable, ProgramArtifact};

#[path = "prepared_metadata_image.rs"]
#[cfg(feature = "canonical-native-images")]
mod image;

/// Resource metadata selected from a validated canonical artifact. This owns
/// only the resource plan, original requirements and schemas, not the program
/// graph. Construction performs no I/O and admits no capability. An unsafe
/// build-owned image loader may restore the same checked metadata; no public
/// Deserialize implementation or safe unchecked constructor is provided.
#[derive(Clone, Debug)]
pub struct CanonicalNativeResourceMetadata {
    plan: CanonicalNativeResourcePlan,
    requirements: ApplicationRequirementTable,
    schemas: SchemaTable,
}

impl CanonicalNativeResourceMetadata {
    pub fn prepare(artifact: &ProgramArtifact, max_ports: usize) -> MResult<Self> {
        Ok(Self {
            plan: CanonicalNativeResourcePlan::select(artifact, max_ports)?,
            requirements: artifact.requirements().clone(),
            schemas: artifact.schemas().clone(),
        })
    }

    pub fn plan(&self) -> &CanonicalNativeResourcePlan {
        &self.plan
    }
}

impl MechRuntime {
    /// Use already-selected resource metadata through the SAME phased resource
    /// pipeline. Every invocation rechecks port caps and fresh runtime authority;
    /// preparing or reusing metadata cannot retain a grant from another runtime.
    /// Native session provenance and matching revision remain the native host's
    /// responsibility, exactly as for the artifact-based phased entry point.
    pub fn with_prepared_canonical_native_resources_phased_text<S, R>(
        &mut self,
        metadata: &CanonicalNativeResourceMetadata,
        limits: CanonicalNativeResourceLimits,
        prepare: impl FnOnce(
            CanonicalNativeResourceAdmission<'_>,
        ) -> MResult<CanonicalNativeResourcePrepared<S>>,
        chunk: impl FnMut(&mut S, CanonicalNativeResourceTextChunk<'_>) -> MResult<()>,
        invoke: impl FnOnce(
            S,
            CanonicalNativeResourceTurn<'_>,
        ) -> MResult<CanonicalNativeResourceCompletion<R>>,
    ) -> Result<R, CanonicalNativeResourceRunError> {
        self.with_selected_native_resources_delivery(
            ResourceMetadataView {
                plan: &metadata.plan,
                requirements: &metadata.requirements,
                schemas: &metadata.schemas,
            },
            limits,
            true,
            prepare,
            chunk,
            invoke,
        )
    }
}
