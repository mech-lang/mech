//! Exercise the exact pure compiler state without the GPU package's full-catalog tests.
#[path = "../../../hosts/gpu/src/native_artifact/codegen/common/boolean_predecessors.rs"]
mod boolean_predecessors;
#[path = "../../../hosts/gpu/src/native_artifact/program/compact/owner_reads.rs"]
mod owner_reads;
#[path = "../../../hosts/gpu/src/native_artifact/program/codegen/cfg_cleanup/identity_routes/reach.rs"]
mod route_reach;
#[path = "../../../hosts/gpu/src/native_artifact/program/compact/guard_state.rs"]
mod state;
