//! Private cache-storage choice at real canonical root/loop library sites.
//! These storage tests do not grant a generated current visit.
use super::*;
use mech_engine::native_artifact::{NativeControlLoop, NativeIterationAction};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

pub(super) fn artifacts(source: &str) -> [mech_engine::ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4f4e4345), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let original = mech_engine::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}

pub(super) fn policy() -> MemoryBudgetLimits {
    let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    limits.max_compute_work = Some(134_217_728);
    limits.max_comparison_work = Some(1_000_000);
    limits.max_temporary_bytes = Some(128 * 1024 * 1024);
    limits
}

fn lane(
    descriptor: &NativeProgramDescriptor,
    memory: &ManagedMemoryBudget,
) -> (CallCacheLane, NativeLibraryWorkBudget) {
    let mut work = NativeWorkMeter::new(1_000_000_000).unwrap();
    work.bind_memory_budget(memory).unwrap();
    let lane = CallCacheLane::new(descriptor, 128 * 1024 * 1024, memory, &mut work).unwrap();
    let library = work
        .shared_library_budget(NativeExecutionPhase::Setup, 1_000_000, memory)
        .unwrap();
    (lane, library)
}

fn only_leaf(descriptor: &NativeProgramDescriptor) -> usize {
    let sites = descriptor
        .calls
        .iter()
        .enumerate()
        .filter_map(|(site, call)| call.as_ref().map(|_| site))
        .collect::<Vec<_>>();
    assert_eq!(sites.len(), 1);
    sites[0]
}

fn text(
    descriptor: &NativeProgramDescriptor,
    site: usize,
    text: &str,
    memory: &ManagedMemoryBudget,
) -> Value {
    let call = descriptor.calls[site].as_ref().unwrap();
    let mech_core::ResolvedOperationContract::Declared(contract) = call.template().contract()
    else {
        panic!("declared String length");
    };
    assert_eq!(contract.inputs.len(), 1);
    let value = ValueDraft {
        schema: contract.inputs[0].schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::String(text.into()),
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(
        &descriptor.schemas,
    ))
    .unwrap();
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}

#[test]
fn once_library_slot_retains_no_binding_or_planner_metadata() {
    for artifact in artifacts("+> sequence\nsequence/length(seed<string>)\n") {
        let jit = super::super::super::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let descriptor = jit.descriptor();
        let site = only_leaf(descriptor);
        assert!(descriptor.plan.blocks()[site].iteration().is_none());
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let (mut lane, library) = lane(descriptor, &memory);
        assert!(matches!(&lane.entries[site], CallCacheEntry::Once));
        let args = [Some(text(descriptor, site, "e\u{301}A", &memory))];
        let before = memory.used_bytes();
        let mut output = None;
        lane
            .invoke(site,
                descriptor.calls[site].as_ref().unwrap(),
                &args,
                &mut output,
                policy(),
                &library,
            )
            .unwrap();
        assert!(matches!(output.as_ref().unwrap().data(), ValueData::U64(2)));
        drop(output);
        // The output's complete ownership retires; there is no bound call,
        // planner population or weak String-index shell left in this slot.
        assert_eq!(memory.used_bytes(), before);
        assert!(matches!(&lane.entries[site], CallCacheEntry::Once));
        drop((args, lane, library));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn repeated_cache_demand_is_read_only_account_bound_and_keeps_nested_fuel() {
    let source = "+> sequence\ntext := payload<string>\nstep(x<u64>) => (next<u64>) := next := x + sequence/length(text).\nseed<u64> |4> step\n";
    for artifact in artifacts(source) {
        let jit = super::super::super::JitProgramArtifact::compile(
            &artifact, &mech_stdlib::source_catalog()).unwrap();
        let descriptor = jit.descriptor();
        let site = only_leaf(descriptor);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let (mut lane, library) = lane(descriptor, &memory);
        let demand = |lane: &CallCacheLane, work: &NativeLibraryWorkBudget| {
            let CallCacheEntry::Repeated(cache, _) = &lane.entries[site] else { panic!("repeated"); };
            cache.retained_metadata_demand(work)
        };
        let cold = demand(&lane, &library).unwrap();
        assert_eq!(cold.turn_peak_bytes, 0);
        assert_eq!(cold.retained_nodes, 0);
        let input = [Some(text(descriptor, site, &"é".repeat(4096), &memory))];
        let before = memory.used_bytes();
        let call = descriptor.calls[site].as_ref().unwrap();
        let mut output = None;
        lane.invoke(site, call, &input, &mut output, policy(), &library).unwrap();
        assert!(matches!(output.as_ref().unwrap().data(), ValueData::U64(4096)));
        drop(output.take());
        let held = memory.used_bytes();
        let nodes = memory.retained_snapshot_nodes();
        let preview = demand(&lane, &library).unwrap();
        assert_eq!(preview.turn_peak_bytes, held - before,
            "include independently admitted binding, plan and index metadata");
        assert!(preview.turn_peak_bytes > 0 && preview.retained_nodes > 0);
        let before_read = library.used().0;
        assert_eq!(lane.caller_retained_nodes(&library).unwrap(),
            lane._reservation.capacity_bytes() + preview.retained_nodes);
        assert_eq!(library.used().0 - before_read, 256,
            "aggregate lookup has fixed work independent of cache count");
        let second = demand(&lane, &library).unwrap();
        assert_eq!(preview, second);
        assert_eq!(memory.used_bytes(), held, "preview allocates no backing");
        assert_eq!(memory.retained_snapshot_nodes(), nodes, "preview retains no Values");
        let denied = library.with_call_limits(0, 0, || demand(&lane, &library)).unwrap();
        assert!(matches!(denied, Err(NativeCollectionInvocationError::Kernel(
            mech_core::ResidentKernelError::ResourceLimit { .. }))));
        assert_eq!(demand(&lane, &library).unwrap(), preview,
            "denial preserves metadata and never renews nested fuel");
        let mut foreign_meter = NativeWorkMeter::new(1_000_000).unwrap();
        foreign_meter.bind_memory_budget(&memory).unwrap();
        let foreign = foreign_meter.shared_library_budget(NativeExecutionPhase::Setup,
            1_000_000, &memory).unwrap();
        assert!(matches!(demand(&lane, &foreign), Err(NativeCollectionInvocationError::InvalidInput)));
        assert!(lane.caller_retained_nodes(&foreign).is_err());
        drop((foreign, foreign_meter));
        drop(input);
        assert_eq!(memory.retained_snapshot_nodes(), 0,
            "bound metadata must not keep the caller payload alive");
        assert_eq!(demand(&lane, &library).unwrap(), preview,
            "reading demand never needs the retired caller payload");
        drop((lane, library));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn repeated_library_slot_reuses_binding_but_reads_fresh_values_and_recovers() {
    let source = "+> sequence\ntext := payload<string>\nstep(x<u64>) => (next<u64>) := next := x + sequence/length(text).\nseed<u64> |4> step\n";
    for artifact in artifacts(source) {
        let jit = super::super::super::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let descriptor = jit.descriptor();
        let site = only_leaf(descriptor);
        assert!(descriptor.plan.blocks()[site].iteration().is_some());
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let (mut lane, library) = lane(descriptor, &memory);
        assert!(matches!(&lane.entries[site], CallCacheEntry::Repeated(..)));
        // Equal byte lengths and empty shape tapes are not value identity.
        let args = ["ABC", "e\u{301}"].map(|value| [Some(text(descriptor, site, value, &memory))]);
        let before = memory.used_bytes();
        let call = descriptor.calls[site].as_ref().unwrap();
        let mut output = None;
        let start = library.used().0;
        lane
            .invoke(site,call, &args[0], &mut output, policy(), &library)
            .unwrap();
        assert!(matches!(output.as_ref().unwrap().data(), ValueData::U64(3)));
        let first_work = library.used().0 - start;
        drop(output.take());
        assert!(
            memory.used_bytes() > before + call.binding_metadata_bytes() + 4096,
            "the repeatable slot must retain its fully admitted planner cache"
        );
        let start = library.used().0;
        lane
            .invoke(site,call, &args[1], &mut output, policy(), &library)
            .unwrap();
        assert!(matches!(output.as_ref().unwrap().data(), ValueData::U64(1)));
        assert!(library.used().0 - start < first_work);
        let prior = output.as_ref().unwrap().clone();
        let held = memory.used_bytes();
        let blocker = memory
            .reserve_capacity(memory.limit_bytes() - held)
            .unwrap();
        assert!(matches!(
            lane.invoke(site, call, &args[1], &mut output, policy(), &library),
            Err(NativeCollectionInvocationError::Memory(
                mech_core::MemoryRuntimeError::BudgetExceeded { .. }
            ))
        ));
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        drop(blocker);
        assert_eq!(memory.used_bytes(), held);
        drop(prior);
        lane
            .invoke(site,call, &args[1], &mut output, policy(), &library)
            .unwrap();
        assert!(matches!(output.as_ref().unwrap().data(), ValueData::U64(1)));
        drop((output, lane));
        assert_eq!(
            memory.used_bytes(),
            before - CallCacheLane::metadata_bytes(descriptor.plan.blocks().len()).unwrap()
        );
        drop((args, library));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn once_repeatable_witness_covers_activation_matches_counted_collections_and_tail() {
    let cases = [
        (
            "+> sequence\nclosed := sequence/length(\"ABC\")\nfresh := sequence/length(seed<string>)\n",
            false,
            false,
        ),
        (
            "+> sequence\npick(s<string>, flag<bool>) => (out<u64>) := out := (flag? | true => (true? | true => sequence/length(s) | false => 0u64) | false => sequence/length(s)).\npick(seed<string>, branch<bool>)\n",
            false,
            false,
        ),
        (
            "+> sequence\ntext := payload<string>\nstep(x<u64>) => (next<u64>) := next := (x? | 0u64 => sequence/length(text) | * => x + sequence/length(text)).\nseed<u64> |3> step\n",
            true,
            false,
        ),
        (
            "+> sequence\n[sequence/length(word) | word <- words<[string]>]\n",
            true,
            false,
        ),
        (
            "+> sequence\n#Scan(n<u64>, token<string>) => <u64>\n  | :Step(left<u64>, total<u64>)\n  | :Done(total<u64>).\n#Scan(n, token) -> :Step(n, 0u64)\n  :Step(left, total)\n    | left > 0u64 -> :Step(left - 1u64, total + sequence/length(token))\n    | * -> :Done(total)\n  :Done(total) => total.\n#Scan(input<u64>, payload<string>)\n",
            true,
            true,
        ),
    ];
    for (source, repeats, tail) in cases {
        for artifact in artifacts(source) {
            let jit = super::super::super::JitProgramArtifact::compile(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let descriptor = jit.descriptor();
            descriptor.plan.validate_current_visits().unwrap();
            assert_eq!(
                descriptor
                    .plan
                    .iterations()
                    .iter()
                    .any(|item| matches!(item, NativeControlLoop::Tail(_))),
                tail
            );
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let (lane, library) = lane(descriptor, &memory);
            let mut once_calls = 0;
            let mut repeated_calls = 0;
            for (site, block) in descriptor.plan.blocks().iter().enumerate() {
                if !matches!(block.action(), NativeIterationAction::LibraryCall { .. }) {
                    continue;
                }
                match (&lane.entries[site], block.iteration()) {
                    (CallCacheEntry::Once, None) => once_calls += 1,
                    (CallCacheEntry::Repeated(..), Some(_)) => repeated_calls += 1,
                    _ => panic!("cache storage differs from the validated lexical scope"),
                }
            }
            assert_eq!(repeated_calls > 0, repeats, "{source}");
            if !repeats {
                assert!(once_calls > 0, "{source}");
            }
            drop((lane, library));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn preserved_control_checkpoint_edges_cannot_replay_root_sites() {
    let source =
        "+> sequence\nclosed := sequence/length(\"ABC\")\nfresh := sequence/length(seed<string>)\n";
    for artifact in artifacts(source) {
        let jit = super::super::super::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let descriptor = jit.descriptor();
        assert!(descriptor.plan.has_activation());
        let boundary = descriptor.plan.activation_boundary().unwrap();
        let mut progress = vec![0; descriptor.plan.iterations().len() * 2];
        let mut control = Control {
            descriptor,
            progress: &mut progress,
            current: None,
            previous: None,
            completed: false,
            work: NativeWorkMeter::new(1_000_000_000).unwrap(),
            visits: 0,
            error: None,
        };
        let mut site = descriptor.plan.entry();
        let mut closed_call = None;
        while site != boundary {
            control.visit(u64::from(site)).unwrap();
            if matches!(
                descriptor.plan.blocks()[site as usize].action(),
                NativeIterationAction::LibraryCall { .. }
            ) {
                closed_call = Some(site);
            }
            let NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(next)) =
                descriptor.plan.blocks()[site as usize].terminator()
            else {
                panic!("straight-line phase fixture");
            };
            site = next;
        }
        assert!(closed_call.is_some());
        // Prepared::run preserves this exact continuation, rather than
        // restarting at entry; NativeProgramExecutionSession::resume consumes it.
        let before = control.visits;
        assert!(control.visit(u64::from(descriptor.plan.entry())).is_err());
        assert_eq!(control.visits, before);
        control.visit(u64::from(boundary)).unwrap();
        assert!(control.visit(u64::from(closed_call.unwrap())).is_err());
        let NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(mut next)) =
            descriptor.plan.blocks()[boundary as usize].terminator()
        else {
            panic!("checkpoint jump");
        };
        loop {
            control.visit(u64::from(next)).unwrap();
            if control.completed {
                break;
            }
            let NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(successor)) =
                descriptor.plan.blocks()[next as usize].terminator()
            else {
                panic!("straight-line turn fixture");
            };
            next = successor;
        }
        assert!(control.visit(u64::from(descriptor.plan.entry())).is_err());
    }
}

#[path = "program_call_cache_allocation_tests.rs"]
mod allocation_tests;
