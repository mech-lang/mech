//! Compile the production pure graph solver without the full GPU test catalog.
#[path = "../../../hosts/gpu/src/native_artifact/program/packed_liveness.rs"]
pub(crate) mod packed_liveness;
mod native_artifact {
    pub(crate) mod program {
        pub(crate) use crate::packed_liveness;
    }
}
#[path = "../../../hosts/gpu/src/native_artifact/program/prepare/numeric_liveness.rs"]
mod liveness;
