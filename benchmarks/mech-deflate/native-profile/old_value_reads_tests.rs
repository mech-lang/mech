//! Run the exact engine graph-query tests with its public same-plan facts API.
use mech_engine::native_artifact::{NativeProgramDefinition, NativeProgramReadRole};
mod native_artifact {
    pub use mech_engine::native_artifact::{
        NativeCollectionResult, NativeCollectionSource, NativeControlLoop, NativeMatchValue,
        NativeProgramArtifactPlan,
    };
}
#[allow(dead_code)]
struct NativeProgramFacts<'p>(mech_engine::native_artifact::NativeProgramFacts<'p>);
#[allow(dead_code)]
impl NativeProgramFacts<'_> {
    fn plan(&self) -> &native_artifact::NativeProgramArtifactPlan {
        self.0.plan()
    }
    fn read_roles(
        &self,
        value: native_artifact::NativeMatchValue,
    ) -> Result<&[NativeProgramReadRole], &'static str> {
        self.0.read_roles(value)
    }
    fn definition_sites(&self, value: u32) -> Result<&[NativeProgramDefinition], &'static str> {
        self.0.definition_sites(value)
    }
    fn successors(&self, site: u32) -> Result<[Option<u32>; 2], &'static str> {
        self.0.successors(site)
    }
}
#[allow(dead_code)]
#[path = "../../../src/engine/src/native_artifact/old_value_reads.rs"]
mod queries;
