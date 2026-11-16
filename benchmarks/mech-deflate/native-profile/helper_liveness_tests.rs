//! Exercise the exact helper dataflow and shared packed kernel.
#[path = "../../../hosts/gpu/src/native_artifact/program/packed_liveness.rs"]
pub(crate) mod packed_liveness;
mod native_artifact {
    pub(crate) mod program {
        pub(crate) use crate::packed_liveness;
    }
}
#[path = "../../../hosts/gpu/src/native_artifact/program/codegen/minimal_liveness_dataflow.rs"]
mod helper_liveness;
