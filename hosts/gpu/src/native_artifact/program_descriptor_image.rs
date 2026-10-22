//! Build-produced descriptor metadata. Encoding does not grant code authority.
use super::NativeProgramDescriptor;
use mech_core::SchemaTable;
use serde::{Serialize, Serializer};

#[path = "program_descriptor_wire.rs"]
mod wire;
pub(in crate::native_artifact) use wire::proof;

pub(super) fn serialize_schemas<S: Serializer>(
    schemas: &SchemaTable,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(schemas.entries().map(|entry| entry.schema()))
}

impl NativeProgramDescriptor {
    /// Export the already-validated plan, layouts, selected-call metadata and
    /// canonical schemas. No pointers or invocation-local cache identities are
    /// persisted. Only the unsafe build-owned loader may restore its compiler
    /// proofs, and it must retain the exact checked Object's provenance.
    pub fn encode_build_owned_image(&self) -> Result<Vec<u8>, serde_json::Error> {
        #[derive(Serialize)]
        struct Image<'a> {
            format: &'static str,
            version: u32,
            descriptor: &'a NativeProgramDescriptor,
        }
        serde_json::to_vec(&Image {
            format: "mech.build-owned-native-descriptor",
            version: 1,
            descriptor: self,
        })
    }

    /// Restore build-prepared metadata without artifact lowering, use analysis,
    /// region selection, or resource acquisition. This is not code authority.
    ///
    /// # Safety
    /// Confine the result until the caller establishes that these exact image
    /// bytes and the linked Object came from the SAME unchanged checked build.
    /// Verify source, bytecode, descriptor and Object pins before linking. The
    /// supplied image/digest must not be treated as authentication of arbitrary
    /// code or semantic proofs. Bind the Object's complete ABI/work/target/IR
    /// identity before execution and preserve normal admission, current-visit
    /// checks, and capability enforcement. Only the actual linked maintained
    /// factories may populate `catalog`.
    pub unsafe fn decode_build_owned_image(
        bytes: &[u8],
        catalog: &mech_core::FunctionCatalog,
    ) -> Result<Self, String> {
        // Private wire assembly never publishes a safe deserializer for a
        // descriptor or engine proof. Its result inherits this confinement.
        wire::decode(bytes, catalog)
    }
}
