//! Build-produced descriptor metadata. Encoding does not grant code authority.
use super::NativeProgramDescriptor;
use mech_core::SchemaTable;
use serde::{Serialize, Serializer};

pub(super) fn serialize_schemas<S: Serializer>(
    schemas: &SchemaTable,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(schemas.entries().map(|entry| entry.schema()))
}

impl NativeProgramDescriptor {
    /// Export the already-validated plan, layouts, selected-call metadata and
    /// canonical schemas. No pointers or invocation-local cache identities are
    /// persisted. This is not yet a loadable image: the existing checked startup
    /// remains authoritative until a build-owned loader is implemented.
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
}
