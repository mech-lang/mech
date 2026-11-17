//! Exercise the production ownership proof without the full catalog of machines.
#![allow(
    dead_code,
    reason = "The source includes the unrelated private-function inliner"
)]

#[path = "../../../hosts/gpu/src/native_artifact/program/compact/inlining.rs"]
mod inlining;
#[path = "../../../hosts/gpu/src/native_artifact/program/compact/owner_counts/mod.rs"]
mod owner_counts;
