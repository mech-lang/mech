//! Observe the SAME selected Project return, not whole-turn success. A retained
//! index may truthfully make a later sink allocation need a higher allowance.
//! The callback observer stores only fixed Copy headers; it never changes an
//! account, replaces a helper, allocates, formats, or asserts across C.
use super::super::super::tests::{artifact_source, limits};
use super::{Error, Visit};
use crate::native_artifact::{
    JitProgramArtifact, NativeExecutionError, NativeProgramExecutionLimits,
    NativeProgramExecutionReport,
};
use mech_core::{
    ManagedMemoryBudget, MemoryBudgetDimension, MemoryRuntimeError, ResidentKernelError, Value,
    snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft},
};
use mech_engine::{ProgramArtifact, native_artifact::NativeCollectionInvocationError};

#[derive(Clone, Copy, Debug)]
struct Record {
    site: u32,
    visit: Visit,
    ok: bool,
    cache_bytes: u64,
    cache_nodes: u64,
    calls: u64,
}
thread_local! {
    static RECORD: core::cell::Cell<Option<Record>> = const { core::cell::Cell::new(None) };
}
pub(super) fn observe(site: u32, visit: Visit, ok: bool, cache_bytes: u64, cache_nodes: u64) {
    RECORD.with(|cell| {
        let calls = cell
            .get()
            .map_or(1, |previous| previous.calls.saturating_add(1));
        cell.set(Some(Record {
            site,
            visit,
            ok,
            cache_bytes,
            cache_nodes,
            calls,
        }));
    });
}

const SOURCE: &str =
    "+> text\ntext/join([(alphabet<string>)[1u64..=1u64] | attempt <- 1u64..=1u64], \"\")\n";
fn artifacts() -> [ProgramArtifact; 2] {
    let original = artifact_source(SOURCE);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}
fn input(artifact: &ProgramArtifact, memory: &ManagedMemoryBudget) -> Value {
    // Finite genuine Unicode input; the published result is one combining
    // grapheme. Public differential tests separately exercise resident parity.
    let value = ValueDraft {
        schema: artifact.inputs()[0].schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::String("é🧑‍🚀".repeat(50_000)),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}
type Outcome = Result<NativeProgramExecutionReport, Error>;
fn observed(
    jit: &JitProgramArtifact,
    args: &[Value],
    outputs: &mut [Option<Value>],
    caps: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
) -> (Outcome, Option<Record>) {
    RECORD.with(|cell| cell.set(None));
    let result = jit.evaluate_into(args, outputs, caps, memory);
    (result, RECORD.with(core::cell::Cell::get))
}
fn snapshot(artifact: &ProgramArtifact, outputs: &[Option<Value>]) -> Vec<Box<[u8]>> {
    outputs
        .iter()
        .map(|value| {
            value
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap()
        })
        .collect()
}
fn target(jit: &JitProgramArtifact) -> (u32, u32, mech_engine::native_artifact::NativeMatchOrigin) {
    let projections = jit
        .descriptor()
        .plan()
        .collections()
        .iter()
        .filter_map(|collection| collection.string_span_projection())
        .collect::<Vec<_>>();
    assert_eq!(projections.len(), 1);
    let projection = projections[0];
    let site = projection.selection_site();
    let iteration = jit.descriptor().plan().blocks()[site as usize]
        .iteration()
        .unwrap();
    (site, iteration, projection.selection_template().origin())
}
fn exact_record(record: Option<Record>, site: u32, iteration: u32, ok: bool) -> Record {
    let record = record.expect("the actual emitted Project helper must return");
    assert_eq!(
        (
            record.site,
            record.visit.iteration,
            record.visit.remaining,
            record.calls
        ),
        (site, iteration, 1, 1)
    );
    assert_eq!(record.ok, ok);
    assert!(
        record.cache_bytes > 0,
        "a genuine selected binding must exist"
    );
    assert_eq!(
        record.cache_nodes,
        if ok { 4 } else { 1 },
        "exact must retain binding+actual Unicode index; short must fail Slot admission, not binding prefix"
    );
    record
}
fn fully_funded(
    jit: &JitProgramArtifact,
    artifact: &ProgramArtifact,
    args: &[Value],
    outputs: &mut [Option<Value>],
    memory: &ManagedMemoryBudget,
    site: u32,
    iteration: u32,
) {
    let (result, record) = observed(jit, args, outputs, limits(), memory);
    result.unwrap();
    exact_record(record, site, iteration, true);
    assert_eq!(outputs.len(), 1);
    assert_eq!(
        outputs[0].as_ref().unwrap().canonical_data_draft().unwrap(),
        ValueDataDraft::String("é".into())
    );
    assert_eq!(snapshot(artifact, outputs).len(), 1);
}

#[test]
fn actual_string_span_project_temporary_exact_one_short_observes_stage_not_later_sink() {
    for artifact in artifacts() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let (site, iteration, origin) = target(&jit);
        let args = [input(&artifact, &memory)];
        let mut outputs = vec![None; artifact.outputs().len()];
        fully_funded(
            &jit,
            &artifact,
            &args,
            &mut outputs,
            &memory,
            site,
            iteration,
        );
        let prior = snapshot(&artifact, &outputs);
        let baseline = memory.used_bytes();
        let mut low = 0;
        let mut high = limits().library.max_temporary_bytes.unwrap();
        while low < high {
            let candidate = low + (high - low) / 2;
            let mut caps = limits();
            caps.library.max_temporary_bytes = Some(candidate);
            let (_, record) = observed(&jit, &args, &mut outputs, caps, &memory);
            if record.is_some_and(|record| record.ok) {
                high = candidate;
            } else {
                low = candidate + 1;
            }
            assert_eq!(snapshot(&artifact, &outputs), prior);
            assert_eq!(memory.used_bytes(), baseline);
        }
        assert!(high > 0);
        let mut exact = limits();
        exact.library.max_temporary_bytes = Some(high);
        let (result, record) = observed(&jit, &args, &mut outputs, exact, &memory);
        let exact_observation = exact_record(record, site, iteration, true);
        if let Err(error) = result {
            assert!(
                matches!(error, Error::Collection { origin: later, error: ResidentKernelError::ResourceLimit {
                dimension: MemoryBudgetDimension::TemporaryBytes, required, limit, ..
            }} if later != origin && required > limit && limit == high),
                "Project succeeded; any later denial must truthfully name the sink, not be called Project success for the whole turn: {error:?}"
            );
        }
        assert_eq!(snapshot(&artifact, &outputs), prior);
        assert_eq!(memory.used_bytes(), baseline);
        let mut short = exact;
        short.library.max_temporary_bytes = Some(high - 1);
        let (result, record) = observed(&jit, &args, &mut outputs, short, &memory);
        let short_observation = exact_record(record, site, iteration, false);
        assert!(exact_observation.cache_bytes > short_observation.cache_bytes);
        let error = result.unwrap_err();
        assert!(
            matches!(error, Error::Library { origin: actual,
            error: NativeCollectionInvocationError::Kernel(ResidentKernelError::ResourceLimit {
                dimension: MemoryBudgetDimension::TemporaryBytes, required, limit, ..
            })} if actual == origin && required == high && limit == high - 1),
            "one-short must deny the actual selected Project index: {error:?}"
        );
        assert_eq!(snapshot(&artifact, &outputs), prior);
        assert_eq!(memory.used_bytes(), baseline);
        fully_funded(
            &jit,
            &artifact,
            &args,
            &mut outputs,
            &memory,
            site,
            iteration,
        );
        assert_eq!(snapshot(&artifact, &outputs), prior);
        drop((args, outputs));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn actual_string_span_project_global_exact_one_short_observes_stage_not_later_sink() {
    for artifact in artifacts() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let (site, iteration, origin) = target(&jit);
        let args = [input(&artifact, &memory)];
        let mut outputs = vec![None; artifact.outputs().len()];
        fully_funded(
            &jit,
            &artifact,
            &args,
            &mut outputs,
            &memory,
            site,
            iteration,
        );
        let prior = snapshot(&artifact, &outputs);
        let baseline = memory.used_bytes();
        let available = memory.limit_bytes() - baseline;
        let mut low = 0;
        let mut high = available;
        while low < high {
            let candidate = low + (high - low) / 2;
            let ballast = memory.reserve_capacity(available - candidate).unwrap();
            let (_, record) = observed(&jit, &args, &mut outputs, limits(), &memory);
            if record.is_some_and(|record| record.ok) {
                high = candidate;
            } else {
                low = candidate + 1;
            }
            assert_eq!(snapshot(&artifact, &outputs), prior);
            drop(ballast);
            assert_eq!(memory.used_bytes(), baseline);
        }
        assert!(high > 0);
        let mut exact_cache_bytes = None;
        for short in [false, true] {
            let remaining = high - u64::from(short);
            let ballast = memory.reserve_capacity(available - remaining).unwrap();
            let (result, record) = observed(&jit, &args, &mut outputs, limits(), &memory);
            let record = exact_record(record, site, iteration, !short);
            if short {
                assert!(exact_cache_bytes.unwrap() > record.cache_bytes);
                let error = result.unwrap_err();
                assert!(
                    matches!(error, Error::Library { origin: actual,
                    error: NativeCollectionInvocationError::Memory(MemoryRuntimeError::BudgetExceeded {
                        operation: None, requested, limit, ..
                    })} if actual == origin && requested == limit + 1),
                    "one-short must deny the actual selected Project Slot reservation: {error:?}"
                );
            } else {
                exact_cache_bytes = Some(record.cache_bytes);
                if let Err(error) = result {
                    assert!(
                        matches!(error, Error::Base(NativeExecutionError::Memory {
                        error: MemoryRuntimeError::BudgetExceeded { requested, limit, .. }, ..
                    }) if requested > limit),
                        "Project succeeded; a later real sink reservation may still deny without publishing: {error:?}"
                    );
                }
            }
            assert_eq!(snapshot(&artifact, &outputs), prior);
            drop(ballast);
            assert_eq!(memory.used_bytes(), baseline);
        }
        fully_funded(
            &jit,
            &artifact,
            &args,
            &mut outputs,
            &memory,
            site,
            iteration,
        );
        assert_eq!(snapshot(&artifact, &outputs), prior);
        drop((args, outputs));
        assert_eq!(memory.used_bytes(), 0);
    }
}
