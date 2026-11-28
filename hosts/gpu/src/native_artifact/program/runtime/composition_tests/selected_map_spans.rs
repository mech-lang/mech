//! Public source programs compare span publication with the retained old path.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use mech_core::snapshot::F64Bits;
fn output(values: &[Option<Value>]) -> Vec<Data> {
    contents(&values.iter().flatten().cloned().collect::<Vec<_>>())
}
fn pair(artifact: &ProgramArtifact) -> (JitProgramArtifact, JitProgramArtifact) {
    let catalog = mech_stdlib::source_catalog();
    let native = JitProgramArtifact::compile(artifact, &catalog).unwrap();
    let old = JitProgramArtifact::compile_for_test(
        artifact,
        &catalog,
        OptimizationPass::SelectedMapSpans,
    )
    .unwrap();
    let object = crate::native_artifact::emit_program_object(artifact, &catalog).unwrap();
    assert!(
        numeric_masked_maps::emitted_masks(&object.metadata()) > 0,
        "source-derived selected map"
    );
    (native, old)
}
fn compare(
    native: &JitProgramArtifact,
    old: &JitProgramArtifact,
    args: &[Value],
    memory: &ManagedMemoryBudget,
    policy: NativeProgramExecutionLimits,
    restored: bool,
    fast: &mut [Option<Value>],
    prior: &mut [Option<Value>],
) -> (observe::Counts, bool) {
    let before = contents(args);
    let previous = output(fast);
    let run = if restored {
        observe::run_restored
    } else {
        observe::run
    };
    let (a, counts) = run(native, args, fast, policy, memory);
    let (b, old_counts) = run(old, args, prior, policy, memory);
    assert_eq!(
        a.as_ref().map_err(|e| format!("{e:?}")),
        b.as_ref().map_err(|e| format!("{e:?}")),
        "unchanged source report/first error"
    );
    assert_eq!(
        output(fast),
        output(prior),
        "exact current values and F64 words"
    );
    if a.is_err() {
        assert_eq!(output(fast), previous, "atomic failure");
    }
    assert_eq!(counts.helpers, old_counts.helpers);
    assert_eq!(counts.admits, old_counts.admits);
    assert_eq!(contents(args), before);
    (counts, a.is_err())
}
#[test]
fn selected_map_spans_public_kinds_axes_comparators_bounds_repeated_states_bits_aliases_and_restore()
 {
    for (kind, literal) in [
        ("bool", "true"),
        ("u8", "255u8"),
        ("u16", "65535u16"),
        ("u32", "4294967295u32"),
        ("u64", "9u64"),
        ("i64", "-7i64"),
        ("f64", "2.5"),
    ] {
        let cells = (0..51)
            .map(|i| match kind {
                "bool" => Data::Bool(i % 2 == 0),
                "u8" => Data::U8(i as u8),
                "u16" => Data::U16((i * 17) as u16),
                "u32" => Data::U32(i as u32),
                "u64" => Data::U64(i as u64),
                "i64" => Data::I64(-(i as i64)),
                _ => Data::F64(F64Bits::from_bits(
                    [
                        0,
                        1,
                        0x8000_0000_0000_0000,
                        0x7ff8_0000_0000_0037,
                        0x7ff0_0000_0000_0041,
                        0xfff0_0000_0000_0000,
                    ][i % 6],
                )),
            })
            .collect::<Vec<_>>();
        for axis in ["i", "j"] {
            for op in ["==", "!=", "<", "<=", ">", ">="] {
                let source = numeric_masked_maps::MATRIX_SOURCE
                    .replace("3,4", "3,17")
                    .replace("..=4u64", "..=17u64")
                    .replace("[u64]", &format!("[{kind}]"))
                    .replace("data[i,j] + 1u64", literal)
                    .replace("$SELECT", &format!("{axis} {op} bound"));
                for artifact in artifacts(&source) {
                    let (native, old) = pair(&artifact);
                    let mode = NativeProgramLimitMode::NoLimits;
                    let memory = mode.memory_budget();
                    for bound in [0, 1, 2, 3, 17, 18, u64::MAX] {
                        for rounds in [0, 1, 3] {
                            let args = numeric_maps::typed_arguments(
                                &artifact,
                                &memory,
                                &[
                                    ("input", Data::Matrix(cells.clone().into()), &[3, 17]),
                                    ("bound", Data::U64(bound), &[]),
                                    ("rounds", Data::U64(rounds), &[]),
                                ],
                            );
                            let aliases = args.clone();
                            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                            for restored in [false, true] {
                                let mut fast = vec![None; artifact.outputs().len()];
                                let mut prior = fast.clone();
                                let (_, failed) = compare(
                                    &native,
                                    &old,
                                    &args,
                                    &memory,
                                    mode.execution(),
                                    restored,
                                    &mut fast,
                                    &mut prior,
                                );
                                assert!(!failed, "constant selected update succeeds");
                            }
                            assert_eq!(contents(&aliases), contents(&args));
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                retained
                            );
                        }
                    }
                    assert_eq!(memory.used_bytes(), 0);
                }
            }
        }
    }
}
fn cap(policy: &mut NativeProgramExecutionLimits, dimension: u8, value: u64) {
    match dimension {
        0 => policy.library.max_output_elements = Some(value),
        1 => policy.library.max_output_bytes = Some(value),
        2 => policy.library.max_temporary_bytes = Some(value),
        3 => policy.library.max_cloned_bytes = Some(value),
        4 => policy.library.max_retained_nodes = Some(value),
        5 => policy.library.max_compute_work = Some(value),
        6 => policy.library.max_comparison_work = Some(value),
        7 => policy.execution.max_work = value,
        _ => policy.execution.max_transport_bytes = value,
    }
}
#[test]
fn selected_map_spans_compound_domains_reverse_reads_partial_failures_finite_limits_cleanup_and_retry()
 {
    let compound = r#"#Span(initial<[u64]:3,17>,row<u64>,column<u64>,rounds<u64>,offset<u64>) => <[u64]:3,17>
  | :Step(data<[u64]:3,17>,pass<u64>)
  | :Done(data<[u64]:3,17>).
#Span(initial,row,column,rounds,offset) -> :Step(initial,0u64)
  :Step(data,pass)
    | pass < rounds -> {
        next := [[((i != row)? | true => ((j >= column)? | true => data[4u64 - i,18u64 - j + ((offset == 0u64)? | true => 0u64 | false => offset / (17u64 - j))] + 1u64 | false => data[i,j]) | false => data[i,j]) | i <- 1u64..=3u64]' | j <- 1u64..=17u64]
      } -> :Step(next,pass + 1u64)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[u64]:3,17>
answer := #Span(data,row<u64>,column<u64>,rounds<u64>,offset<u64>)
(answer,data)
"#;
    for artifact in artifacts(compound) {
        let (native, old) = pair(&artifact);
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let cells = Data::Matrix((0..51).map(|i| Data::U64(i + 1)).collect());
        for restored in [false, true] {
            let mut fast = vec![None; artifact.outputs().len()];
            let mut prior = fast.clone();
            let mut reached = false;
            for (row, column, rounds, offset) in [
                (2, 1, 3, 0),
                (0, 0, 1, 0),
                (u64::MAX, 18, 1, 0),
                (2, 1, 1, 1),
                (2, 1, 1, u64::MAX),
                (2, 2, 2, 0),
                (2, 1, 3, 0),
            ] {
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("input", cells.clone(), &[3, 17]),
                        ("row", Data::U64(row), &[]),
                        ("column", Data::U64(column), &[]),
                        ("rounds", Data::U64(rounds), &[]),
                        ("offset", Data::U64(offset), &[]),
                    ],
                );
                let aliases = args.clone();
                let before = contents(&args);
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (counts, failed) = compare(
                    &native,
                    &old,
                    &args,
                    &memory,
                    mode.execution(),
                    restored,
                    &mut fast,
                    &mut prior,
                );
                assert_eq!(failed, offset != 0, "late checked source failure");
                reached |= counts.phases > 0;
                if offset == 0 && row == 2 && column == 1 {
                    for dimension in 0..9 {
                        for bound in [0, 1, 64, 4096, 1_000_000_000_000] {
                            let mut policy = mode.execution();
                            cap(&mut policy, dimension, bound);
                            compare(
                                &native, &old, &args, &memory, policy, restored, &mut fast,
                                &mut prior,
                            );
                        }
                    }
                    compare(
                        &native,
                        &old,
                        &args,
                        &memory,
                        mode.execution(),
                        restored,
                        &mut fast,
                        &mut prior,
                    );
                }
                assert_eq!(contents(&aliases), before);
                // Completed output banks may be retained; remove them before
                // checking this invocation's whole managed-memory footprint.
                fast.fill(None);
                prior.fill(None);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
            }
            assert!(reached, "real numeric phases, not only ordinary fallback");
        }
        assert_eq!(memory.used_bytes(), 0);
    }
}
