//! Safe protocol faults and fixed observations around the SAME actual JIT helper.
//! No replacement leaf, prices, quotas, allocator probes, assertions or unwind in C.
use crate::native_artifact::program::runtime::collections::string_map_entry::*;
use crate::native_artifact::program::runtime::test_support::snapshot_outputs as bytes;
use crate::native_artifact::program::runtime::tests::{artifact_source, limits};
use crate::native_artifact::{JitProgramArtifact, NativeValueError};
use mech_core::snapshot::{MapEntryDraft, SnapshotValidationContext, ValueDraft};
use mech_core::{ManagedMemoryBudget, ResidentKernelError, ValueDataDraft as Data};
use mech_engine::native_artifact::{
    NativeCollectionInvocationError, NativeIterationAction as Action,
};
use std::cell::Cell;

const SOURCE: &str = "+> text\n+> map\ntext/join([(entries<{string:string}>){piece} | piece <- seed<string>], \"|\")\n";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fault {
    KeyReplay,
    MissingKey,
    WrongKeyVisit,
    WrongEntryVisit,
    OldKeyGeneration,
    OldMapGeneration,
    ForeignKeyHandle,
    ForeignMapHandle,
    ForeignSite,
    EntryReplay,
    ConsumedReplay,
    LeakedAtLatch,
    ObserveAbsent,
}
#[derive(Clone, Copy, Default)]
struct Observation {
    calls: u64,
    site: usize,
    ok: bool,
    bytes: u64,
    nodes: u64,
}
thread_local! {
    static FAULT: Cell<Option<Fault>> = const { Cell::new(None) };
    static FIRED: Cell<bool> = const { Cell::new(false) };
    static ABSENT: Cell<bool> = const { Cell::new(false) };
    static GENERATED: Cell<bool> = const { Cell::new(false) };
    static LAST_SOURCE_PIN: Cell<bool> = const { Cell::new(false) };
    static OBSERVE: Cell<bool> = const { Cell::new(false) };
    static OBSERVATION: Cell<Observation> = const { Cell::new(Observation { calls: 0, site: 0, ok: false, bytes: 0, nodes: 0 }) };
}
pub(in crate::native_artifact::program::runtime::collections::string_map_entry) fn after_project(
    site: usize,
    ok: bool,
    bytes: u64,
    nodes: u64,
) {
    if !OBSERVE.with(Cell::get) {
        return;
    }
    OBSERVATION.with(|cell| {
        let old = cell.get();
        cell.set(Observation {
            calls: old.calls.saturating_add(1),
            site,
            ok,
            bytes,
            nodes,
        });
    });
}

mod faults;
pub(in super::super) use crate::native_artifact::program::runtime::collections::string_map_entry::tests::faults::before_stage;
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        FAULT.with(|cell| cell.set(None));
        FIRED.with(|cell| cell.set(false));
        ABSENT.with(|cell| cell.set(false));
        GENERATED.with(|cell| cell.set(false));
        LAST_SOURCE_PIN.with(|cell| cell.set(false));
        OBSERVE.with(|cell| cell.set(false));
        OBSERVATION.with(|cell| cell.set(Observation::default()));
    }
}
fn args(
    artifact: &mech_engine::ProgramArtifact,
    memory: &ManagedMemoryBudget,
    text: &str,
    entries: &[(&str, &str)],
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let data = match name.as_str() {
                "seed" => Data::String(text.into()),
                "entries" => Data::Map(
                    entries
                        .iter()
                        .map(|(key, value)| MapEntryDraft {
                            items: vec![Data::String((*key).into()), Data::String((*value).into())]
                                .into_boxed_slice(),
                        })
                        .collect(),
                ),
                name => panic!("unexpected fixture input {name}"),
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values: Box::new([]),
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect()
}
fn variants(source: &str) -> [mech_engine::ProgramArtifact; 2] {
    let original = artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}

#[test]
fn emitted_borrowed_string_map_phase_foreign_generation_replay_and_escape_are_atomic() {
    for (source, generated) in [
        (SOURCE, false),
        (
            "+> text\n+> map\ngenerated := text/join([part | part <- seed<string>], \"\")\ntext/join([(entries<{string:string}>){piece} | piece <- generated], \"|\")\n",
            true,
        ),
    ] {
        for artifact in variants(source) {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(
                jit.descriptor()
                    .plan()
                    .collections()
                    .iter()
                    .filter(|collection| collection.string_element_map_projection().is_some())
                    .count(),
                1
            );
            let args = args(
                &artifact,
                &memory,
                "abc",
                &[("a", "A"), ("b", "B"), ("c", "C")],
            );
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            let prior = bytes(&artifact, &output);
            let baseline = memory.used_bytes();
            for fault in [
                Fault::KeyReplay,
                Fault::MissingKey,
                Fault::WrongKeyVisit,
                Fault::WrongEntryVisit,
                Fault::OldKeyGeneration,
                Fault::OldMapGeneration,
                Fault::ForeignKeyHandle,
                Fault::ForeignMapHandle,
                Fault::ForeignSite,
                Fault::EntryReplay,
                Fault::ConsumedReplay,
                Fault::LeakedAtLatch,
                Fault::ObserveAbsent,
            ] {
                let _reset = Reset;
                FAULT.with(|cell| cell.set(Some(fault)));
                GENERATED.with(|cell| cell.set(generated));
                let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
                assert!(
                    FIRED.with(Cell::get),
                    "actual emitted projected stage must be reached: {fault:?}"
                );
                if fault == Fault::ObserveAbsent {
                    result.unwrap();
                    assert!(
                        ABSENT.with(Cell::get),
                        "no elided key/result Value may exist in the real arena"
                    );
                } else {
                    let error = result.unwrap_err();
                    if generated && fault == Fault::OldKeyGeneration {
                        assert!(
                            LAST_SOURCE_PIN.with(Cell::get),
                            "generated source must be distinct from external inputs and have only its arena, Source and private lane owners"
                        );
                        assert!(
                            matches!(error, Error::Value(NativeValueError::StaleHandle)),
                            "{error:?}"
                        );
                    }
                    assert!(
                        matches!(
                            error,
                            Error::Value(NativeValueError::StaleHandle)
                                | Error::Base(
                                    crate::native_artifact::NativeExecutionError::Descriptor { .. }
                                )
                        ),
                        "{fault:?}: {error:?}"
                    );
                }
                assert_eq!(bytes(&artifact, &output), prior, "{fault:?}");
                assert_eq!(memory.used_bytes(), baseline, "{fault:?}");
                FAULT.with(|cell| cell.set(None));
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                assert_eq!(bytes(&artifact, &output), prior);
                assert_eq!(memory.used_bytes(), baseline);
            }
            drop((args, output));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn emitted_borrowed_string_map_exact_local_owner_peak_and_one_short_are_actual_stage_boundaries() {
    for artifact in variants(SOURCE) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        // Exceed the fixed pre-lookup binding peak without overrunning the
        // unchanged TextJoin cumulative compute cap during baseline output.
        let text = "x".repeat(5 * 256 * 1024);
        let small = args(&artifact, &memory, "a", &[("a", "old")]);
        let large = args(&artifact, &memory, "a", &[("a", text.as_str())]);
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&small, &mut output, limits(), &memory)
            .unwrap();
        let prior = bytes(&artifact, &output);
        let _reset = Reset;
        OBSERVE.with(|cell| cell.set(true));
        jit.evaluate_into(&large, &mut output, limits(), &memory)
            .unwrap();
        let observation = OBSERVATION.with(Cell::get);
        assert_eq!(observation.calls, 1);
        assert!(observation.ok);
        assert!(
            observation.nodes >= 6,
            "full Map(4) and source String(2) nodes must coexist"
        );
        // The unchanged maintained lookup also claims the one-byte canonical
        // map key before its first comparison. No physical key allocation is invented.
        let exact = observation.bytes.checked_add(1).unwrap();
        OBSERVE.with(|cell| cell.set(false));
        jit.evaluate_into(&small, &mut output, limits(), &memory)
            .unwrap();
        assert_eq!(bytes(&artifact, &output), prior);
        let baseline = memory.used_bytes();
        let proof = jit
            .descriptor()
            .plan()
            .collections()
            .iter()
            .find_map(|collection| collection.string_element_map_projection())
            .unwrap();
        assert_eq!(observation.site, proof.selection_site() as usize);
        for (cap, project_ok) in [(exact, true), (exact - 1, false)] {
            OBSERVATION.with(|cell| cell.set(Observation::default()));
            OBSERVE.with(|cell| cell.set(true));
            let mut policy = limits();
            policy.library.max_temporary_bytes = Some(cap);
            let result = jit.evaluate_into(&large, &mut output, policy, &memory);
            OBSERVE.with(|cell| cell.set(false));
            let actual = OBSERVATION.with(Cell::get);
            assert_eq!(
                actual.calls, 1,
                "must reach the same actual selected projection once"
            );
            assert_eq!(actual.site, proof.selection_site() as usize);
            assert_eq!(actual.ok, project_ok);
            if project_ok {
                let error = result.unwrap_err();
                assert!(
                    matches!(
                        error,
                        Error::Collection {
                            error: ResidentKernelError::ResourceLimit {
                                dimension: Dimension::TemporaryBytes,
                                ..
                            },
                            ..
                        }
                    ),
                    "exact project success may still truthfully deny later append growth: {error:?}"
                );
            } else {
                let error = result.unwrap_err();
                assert!(
                    matches!(error, Error::Library { origin,
                    error: NativeCollectionInvocationError::Kernel(ResidentKernelError::ResourceLimit {
                        dimension: Dimension::TemporaryBytes, required, limit, .. }) }
                    if origin == proof.selection_template().origin() && required == exact && limit == exact - 1),
                    "{error:?}"
                );
            }
            assert_eq!(bytes(&artifact, &output), prior);
            assert_eq!(memory.used_bytes(), baseline);
        }
        jit.evaluate_into(&large, &mut output, limits(), &memory)
            .unwrap();
        assert!(
            matches!(output.last().unwrap().as_ref().unwrap().data(), ValueData::String(value) if value.as_ref() == text)
        );
        drop((small, large, output));
        assert_eq!(memory.used_bytes(), 0);
    }
}
