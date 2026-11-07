//! A completed traversal need not preserve its exhausted transition payload.
use super::*;

fn source(best: &str, item: &str) -> String {
    let base = child_prepayment::source();
    assert_eq!(base.matches("| * -> :Done(k, best, item)").count(), 1);
    let exhausted = if best == "conditional" {
        format!(
            "| * -> {{\n        final-best := ((k == 18u64)? | true => 0u32 | false => best)\n      }} -> :Done(k, final-best, {item})"
        )
    } else {
        format!("| * -> :Done(k, {best}, {item})")
    };
    base.replace("| * -> :Done(k, best, item)", &exhausted)
        // One call exposes the error; a second empty call could reset the payload.
        .replace("nodes<[u64]>, 2u64, stop", "nodes<[u64]>, 1u64, stop")
}

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    size: usize,
    start: u64,
    stop: u64,
    price: u32,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("prices", Data::Matrix(vec![Data::U32(price); size].into())),
            ("nodes", Data::Matrix(vec![Data::U64(0); size].into())),
            ("start", Data::U64(start)),
            ("stop", Data::U64(stop)),
            ("seed", Data::U32(u32::MAX)),
            ("nonce", Data::U64(119)),
            ("fault", Data::U64(u64::MAX)),
        ],
        memory,
    )
}

pub(super) fn resident(artifact: &ProgramArtifact, args: &[Value]) -> Vec<Data> {
    use mech_engine::resident::{ActivationFacts, CapturedValueInput, activate};
    let mut instance = activate(
        mech_core::ReactiveInstanceId::new(0x45584954, 0),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let captured = instance
        .plan
        .inputs
        .iter()
        .map(|input| CapturedValueInput {
            slot: input.slot,
            value: &args[artifact
                .inputs()
                .iter()
                .position(|v| v.slot == input.artifact_slot)
                .unwrap()],
        })
        .collect::<Vec<_>>();
    instance
        .prepare_turn_values(&captured)
        .unwrap()
        .publish()
        .unwrap();
    (0..artifact.outputs().len())
        .map(|i| {
            instance
                .copied_output(i)
                .unwrap()
                .canonical_data_draft()
                .unwrap()
        })
        .collect()
}

fn reduction_count(artifact: &ProgramArtifact) -> usize {
    let object = crate::native_artifact::emit_program_object_with_limits(
        artifact,
        &mech_stdlib::source_catalog(),
        NativeProgramLimitMode::NoLimits.compilation(),
    )
    .unwrap();
    let metadata = object.metadata();
    [
        "compiled_readonly_numeric_loops",
        "compiled_numeric_write_loops",
    ]
    .into_iter()
    .flat_map(|key| metadata[key].as_array().into_iter().flatten())
    .flat_map(|region| {
        region["guarded_loop_windows"]
            .as_array()
            .into_iter()
            .flatten()
    })
    .filter(|window| !window["reduction"].is_null())
    .count()
}

#[test]
fn numeric_ordered_reduction_exhausted_payloads_follow_source_edges() {
    let mode = NativeProgramLimitMode::NoLimits;
    for (best, item, changed) in [
        ("best", "item", false),
        ("0u32", "item", true),
        ("best", "17u64", true),
        ("0u32", "17u64", true),
        ("conditional", "item", true),
    ] {
        for artifact in artifacts(&source(best, item)) {
            let native = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut outputs = vec![None; artifact.outputs().len()];
                for (size, start, stop, price) in [
                    (40, 0, 18, 9),
                    (65, 1, 17, 7),
                    (40, 0, 1, 3),
                    (65, 1, 18, 11),
                    (40, 0, 7, 5),
                    (40, 18, 18, 13),
                    (40, 19, 18, 2),
                    (0, 0, 0, 1),
                ] {
                    let args = arguments(&artifact, &memory, size, start, stop, price);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let expected = resident(&artifact, &args);
                    let result = if restored {
                        observe::run_restored(
                            &native,
                            &args,
                            &mut outputs,
                            mode.execution(),
                            &memory,
                        )
                        .0
                    } else {
                        observe::run(&native, &args, &mut outputs, mode.execution(), &memory).0
                    };
                    result.unwrap();
                    assert_eq!(
                        outputs
                            .iter()
                            .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
                            .collect::<Vec<_>>(),
                        expected,
                        "best={best}, item={item}, start={start}, stop={stop}, restored={restored}"
                    );
                    if size == 40 && start == 0 && stop == 18 {
                        assert_eq!(
                            expected.last().unwrap(),
                            &Data::Tuple(
                                vec![
                                    Data::U32(if best == "best" { price } else { 0 }),
                                    Data::U64(if item == "item" { 1 } else { 17 }),
                                    Data::U64(18)
                                ]
                                .into()
                            )
                        );
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(outputs);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
            assert_eq!(
                reduction_count(&artifact) == 0,
                changed,
                "only proved identity exhaustion may be bypassed: best={best}, item={item}"
            );
        }
    }
}

#[test]
fn numeric_ordered_reduction_exhaustion_denials_preserve_publication_and_retry() {
    resources::finite_boundaries(&source("0u32", "17u64"), |artifact, memory| {
        arguments(artifact, memory, 40, 0, 18, 9)
    });
}
