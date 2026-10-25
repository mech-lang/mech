//! Managed ABI16 invocation. This boundary never evaluates a graph: generated
//! blocks choose every successor and run every scalar/counter instruction.
use crate::native_artifact::abi::{
    NO_NODE, NativeEntryFrame, STATUS_ABI, STATUS_ADMISSION, STATUS_ARITHMETIC, STATUS_OK,
};
use crate::native_artifact::iteration_abi::STATUS_REJECT;
use crate::native_artifact::iteration_runtime::{
    ControlState, ControlView as Control, NativeIterationExecutionError,
};
use crate::native_artifact::memory::{NativeTypedStorage, NativeWordStorage, transport_bytes};
use crate::native_artifact::program::prepare::retirement::RetirementKind;
use crate::native_artifact::program::protocol::GlobalHelper;
#[cfg(all(test, feature = "canonical-native-program"))]
use crate::native_artifact::program::protocol::abi::COLLECTION;
#[cfg(all(test, feature = "canonical-native-program"))]
use crate::native_artifact::program::protocol::abi::LIBRARY;
#[cfg(all(test, feature = "canonical-native-program"))]
use crate::native_artifact::program::protocol::abi::MOVE;
use crate::native_artifact::program::protocol::abi::{Frame, OUTPUT_BASE, VERSION};
use crate::native_artifact::{
    NativeExecutionError, NativeExecutionLimits, NativeExecutionPhase, NativeProgramDescriptor,
    NativeValueArena, NativeValueError, NativeWorkMeter,
};
use mech_core::snapshot::{F64Bits, SnapshotValidationContext, ValueDataDraft, ValueDraft};
use mech_core::{ManagedMemoryBudget, MemoryBudgetLimits, Value, ValueData};
use mech_engine::native_artifact::{
    NativeCollectionInvocationError, NativeIterationAction, NativeIterationTerminator,
    NativeLibraryWorkBudget, NativeMatchAction, NativeMatchOrigin, NativeMatchTerminator,
    NativePreparedLibraryCallCache, NativeScalarKind,
};
use std::ffi::c_void;

#[path = "collections/mod.rs"]
pub(in crate::native_artifact::program::runtime) mod collections;

#[path = "lines/mod.rs"]
pub(in crate::native_artifact::program::runtime) mod lines;

#[path = "session/mod.rs"]
pub(in crate::native_artifact::program::runtime) mod session;
#[cfg(feature = "canonical-native-program-host-runtime")]
pub(in crate::native_artifact) use crate::native_artifact::program::runtime::session::CheckedEntry;
#[cfg(feature = "canonical-native-program-host-runtime")]
pub use crate::native_artifact::program::runtime::session::NativeProgramExecutionSession;

#[derive(Clone, Debug)]
pub enum NativeProgramExecutionError {
    Base(NativeExecutionError),
    Control(NativeIterationExecutionError),
    Value(NativeValueError),
    Library {
        origin: NativeMatchOrigin,
        error: NativeCollectionInvocationError,
    },
    Collection {
        origin: NativeMatchOrigin,
        error: mech_core::ResidentKernelError,
    },
    Snapshot(mech_core::snapshot::SnapshotValueError),
}
impl From<NativeExecutionError> for NativeProgramExecutionError {
    fn from(e: NativeExecutionError) -> Self {
        Self::Base(e)
    }
}
impl From<NativeIterationExecutionError> for NativeProgramExecutionError {
    fn from(e: NativeIterationExecutionError) -> Self {
        Self::Control(e)
    }
}
impl From<NativeValueError> for NativeProgramExecutionError {
    fn from(e: NativeValueError) -> Self {
        Self::Value(e)
    }
}
impl From<mech_core::MemoryRuntimeError> for NativeProgramExecutionError {
    fn from(e: mech_core::MemoryRuntimeError) -> Self {
        Self::Base(e.into())
    }
}
impl std::fmt::Display for NativeProgramExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canonical native program: {self:?}")
    }
}
impl std::error::Error for NativeProgramExecutionError {}
// Preserve the complete typed native failure when crossing the ordinary
// runtime transaction bridge; callers can inspect this exact enum in MechError.
impl mech_core::MechErrorKind for NativeProgramExecutionError {
    fn name(&self) -> &str {
        "NativeProgramExecutionError"
    }
    fn message(&self) -> String {
        self.to_string()
    }
}
#[derive(Clone, Copy, Debug)]
pub struct NativeProgramExecutionLimits {
    pub execution: NativeExecutionLimits,
    pub library: MemoryBudgetLimits,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeProgramExecutionReport {
    pub work: u64,
    pub comparison_work: u64,
    pub visits: u64,
    pub transport_bytes: u64,
    pub hash_table_regions: u64,
    pub hash_table_updates: u64,
    /// Regions that actually selected packed storage at runtime.
    pub packed_map_regions: u64,
    /// Packed owners that admitted the bounded complete-key lookup cache.
    pub direct_lookup_regions: u64,
    /// Updates completed by those selected packed regions.
    pub packed_map_updates: u64,
    /// Completed packed regions that entered the versioned SSA CFG.
    pub packed_ssa_regions: u64,
    /// Completed regions whose actual guarded-loop selector chose the fast CFG.
    pub guarded_loop_regions: u64,
    /// Completed regions whose initial selector chose proved call-limit checks.
    pub guarded_call_regions: u64,
    /// Complete packed maps actually retained at their original output boundary.
    pub frozen_map_regions: u64,
    /// Successful original source queries against those complete private maps.
    pub frozen_map_reads: u64,
    /// Bounded paired reservations from the original worker-group escrow.
    pub worker_credit_claims: u64,
    /// Original full funded batches actually debited through those credits.
    pub worker_credit_batches: u64,
    /// Credit settlements that release an unused portion, never spent work.
    pub worker_credit_reclaims: u64,
    pub fused_string_map_bytes: u64,
    /// Source comprehensions whose original child bodies completed in workers.
    pub parallel_regions: u64,
    /// Completed dispatches; one dispatch may cover several source regions.
    pub parallel_batches: u64,
    /// Original source elements completed by those worker batches.
    pub parallel_tasks: u64,
    /// Observed peak simultaneously executing workers, not admitted capacity.
    pub parallel_peak_workers: u64,
}
fn invalid(reason: &'static str) -> NativeProgramExecutionError {
    NativeExecutionError::Descriptor { reason }.into()
}
fn amount(count: usize, scale: u64, base: u64) -> Result<u64, NativeProgramExecutionError> {
    (count as u64)
        .checked_mul(scale)
        .and_then(|n| n.checked_add(base))
        .ok_or_else(|| invalid("program size/work overflow"))
}
fn bits(value: &Value, kind: NativeScalarKind) -> Result<u64, NativeProgramExecutionError> {
    match (kind, value.data()) {
        (NativeScalarKind::Bool, ValueData::Bool(v)) => Ok(u64::from(*v)),
        (NativeScalarKind::U8, ValueData::U8(v)) => Ok(u64::from(*v)),
        (NativeScalarKind::U16, ValueData::U16(v)) => Ok(u64::from(*v)),
        (NativeScalarKind::U32, ValueData::U32(v)) => Ok(u64::from(*v)),
        (NativeScalarKind::U64, ValueData::U64(v)) => Ok(*v),
        (NativeScalarKind::I64, ValueData::I64(v)) => Ok(*v as u64),
        (NativeScalarKind::F64, ValueData::F64(v)) => Ok(v.bits()),
        _ => Err(invalid("canonical scalar payload/type mismatch")),
    }
}
pub(in crate::native_artifact) fn execute<F: FnMut(*mut Frame) -> u64>(
    descriptor: &NativeProgramDescriptor,
    entry: F,
    inputs: &[Value],
    outputs: &mut [Option<Value>],
    limits: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
    // A raw immutable Value slice is not resource authority, even when every
    // resource result could be supplied as a lookalike typed input.
    if !descriptor
        .metadata()
        .plan
        .resource_requirements()
        .is_empty()
    {
        return Err(invalid(
            "hosted native plan requires a runtime-issued turn token",
        ));
    }
    execute_prepared(descriptor, entry, inputs, &[], outputs, limits, memory)
}

#[cfg(feature = "canonical-native-program-host-runtime")]
fn verify_hosted_plan(
    descriptor: &NativeProgramDescriptor,
    revision: mech_core::ProgramRevision,
    bound: &mech_runtime::CanonicalNativeResourcePlan,
) -> Result<(), NativeProgramExecutionError> {
    use mech_runtime::CanonicalNativeResourceAction as Action;
    let selected = &descriptor.metadata().plan;
    if revision != selected.program_revision()
        || bound.requirements() != selected.resource_requirements()
        || bound.observations().len() != selected.observations().len()
        || bound.effects().len() != selected.effects().len()
    {
        return Err(invalid(
            "runtime turn differs from canonical native resource plan",
        ));
    }
    for (bound, native) in bound.observations().iter().zip(selected.observations()) {
        if bound.node() != native.node()
            || bound.requirement() != native.requirement()
            || bound.request() != native.request()
            || bound.schema() != native.ty().schema()
            || bound.schema_key() != native.ty().schema_key()
            || bound.action()
                != (Action::Observation {
                    output: native.slot(),
                })
        {
            return Err(invalid(
                "runtime observation differs from exact native site",
            ));
        }
    }
    for (bound, native) in bound.effects().iter().zip(selected.effects()) {
        if bound.node() != native.node()
            || bound.requirement() != native.requirement()
            || bound.request() != native.request()
            || bound.schema() != native.ty().schema()
            || bound.schema_key() != native.ty().schema_key()
            || bound.action()
                != (Action::Effect {
                    payload: native.payload(),
                    ordinal: native.ordinal(),
                })
        {
            return Err(invalid("runtime effect differs from exact native site"));
        }
    }
    Ok(())
}

#[cfg(feature = "canonical-native-program-host-runtime")]
fn verify_hosted_turn(
    descriptor: &NativeProgramDescriptor,
    turn: &mech_runtime::CanonicalNativeResourceTurn<'_>,
) -> Result<(), NativeProgramExecutionError> {
    verify_hosted_plan(descriptor, turn.program_revision(), turn.plan())?;
    let expected = if turn.text_capture().is_some() {
        0
    } else {
        descriptor.metadata().plan.observations().len()
    };
    if turn.observations().len() != expected {
        return Err(invalid(
            "captured observation count differs from exact native plan",
        ));
    }
    Ok(())
}

#[cfg(feature = "canonical-native-program-host-runtime")]
pub(in crate::native_artifact) fn prepare_hosted<'a>(
    descriptor: &'a NativeProgramDescriptor,
    entry: CheckedEntry,
    admission: mech_runtime::CanonicalNativeResourceAdmission<'_>,
    limits: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
) -> Result<NativeProgramExecutionSession<'a>, NativeProgramExecutionError> {
    NativeProgramExecutionSession::prepare(descriptor, entry, admission, limits, memory)
}

// Ordinary pure evaluation drives the SAME generated phases internally.
// Fresh input/output transport begins only after closed activation succeeds.
fn execute_prepared<F: FnMut(*mut Frame) -> u64>(
    descriptor: &NativeProgramDescriptor,
    mut entry: F,
    inputs: &[Value],
    observations: &[Value],
    outputs: &mut [Option<Value>],
    limits: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
    let mut prepared = session::Prepared::new(descriptor, limits, memory)?;
    prepared.activate(&mut entry)?;
    prepared.finish(&mut entry, inputs, observations, outputs)
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "histogram_query_observation_tests.rs"]
pub(in crate::native_artifact::program::runtime) mod histogram_query_observation_tests;

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "histogram_observation_tests.rs"]
pub(in crate::native_artifact::program::runtime) mod histogram_observation_tests;

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "text_join_observation_tests.rs"]
pub(in crate::native_artifact::program::runtime) mod text_join_observation_tests;

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "tests.rs"]
pub(in crate::native_artifact::program::runtime) mod tests;

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "structural_projection_tests.rs"]
pub(in crate::native_artifact::program::runtime) mod structural_projection_tests;

#[test]
fn native_runtime_consolidation_preserves_the_charged_layout_envelope() {
    let persistent = core::mem::size_of::<ExecutionState<'_>>();
    let invocation = core::mem::size_of::<Invocation<'_, '_, '_>>();
    eprintln!("native-layout: persistent={persistent} invocation={invocation}");
    let map_view = core::mem::size_of::<invocation::MapExecution<'_, '_, '_, '_>>();
    let collection_view = core::mem::size_of::<invocation::CollectionExecution<'_, '_, '_, '_>>();
    assert!(persistent + invocation + map_view + collection_view <= 2432 + 128);
    assert_eq!(invocation::caller_geometry::fixed_nodes().unwrap(), 2704);
    assert_eq!(core::mem::size_of::<Frame>(), 144);
}

#[path = "maps/mod.rs"]
pub(in crate::native_artifact) mod maps;

mod invocation;

mod hooks;
mod state;
use self::hooks::{admit, helper};
use self::invocation::Invocation;
use self::state::{ExecutionState, RuntimeStats};
