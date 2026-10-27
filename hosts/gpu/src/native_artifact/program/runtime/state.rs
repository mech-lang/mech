//! One persistent execution owner and its cumulative report counters.
use super::*;
#[derive(Default)]
pub(in crate::native_artifact::program::runtime) struct RuntimeStats {
    pub(in crate::native_artifact::program::runtime) hash_table_regions: u64,
    pub(in crate::native_artifact::program::runtime) hash_table_updates: u64,
    pub(in crate::native_artifact::program::runtime) packed_map_regions: u64,
    pub(in crate::native_artifact::program::runtime) direct_lookup_regions: u64,
    pub(in crate::native_artifact::program::runtime) packed_map_updates: u64,
    pub(in crate::native_artifact::program::runtime) packed_ssa_regions: u64,
    pub(in crate::native_artifact::program::runtime) guarded_loop_regions: u64,
    pub(in crate::native_artifact::program::runtime) guarded_call_regions: u64,
    pub(in crate::native_artifact::program::runtime) frozen_map_regions: u64,
    pub(in crate::native_artifact::program::runtime) frozen_map_reads: u64,
    pub(in crate::native_artifact::program::runtime) worker_credit_claims: u64,
    pub(in crate::native_artifact::program::runtime) worker_credit_batches: u64,
    pub(in crate::native_artifact::program::runtime) worker_credit_reclaims: u64,
    pub(in crate::native_artifact::program::runtime) fused_string_map_bytes: u64,
    pub(in crate::native_artifact::program::runtime) parallel_regions: u64,
    pub(in crate::native_artifact::program::runtime) parallel_batches: u64,
    pub(in crate::native_artifact::program::runtime) parallel_tasks: u64,
    pub(in crate::native_artifact::program::runtime) parallel_peak_workers: u64,
}
pub(in crate::native_artifact::program::runtime) struct ExecutionState<'plan> {
    // Owners retire before the arena; backing storage retires before claims.
    pub(in crate::native_artifact::program::runtime) map_owners:
        crate::native_artifact::program::runtime::maps::owner::Session<'plan>,
    pub(in crate::native_artifact::program::runtime) collections:
        Option<crate::native_artifact::program::runtime::collections::State>,
    pub(in crate::native_artifact::program::runtime) line_matrices:
        crate::native_artifact::program::runtime::lines::State,
    pub(in crate::native_artifact::program::runtime) buffer_frames:
        super::buffer_frames::BufferFrames,
    pub(in crate::native_artifact::program::runtime) arena: NativeValueArena<'plan>,
    pub(in crate::native_artifact::program::runtime) call_inputs: NativeTypedStorage<Option<Value>>,
    pub(in crate::native_artifact::program::runtime) call_caches: session::CallCacheLane,
    pub(in crate::native_artifact::program::runtime) library: NativeLibraryWorkBudget,
    pub(in crate::native_artifact::program::runtime) control: ControlState,
    pub(in crate::native_artifact::program::runtime) stats: RuntimeStats,
    pub(in crate::native_artifact::program::runtime) consumed: bool,
    pub(in crate::native_artifact::program::runtime) output_cursor: usize,
}
