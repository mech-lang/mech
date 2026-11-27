//! Repeated complete maps and later cold scalar ownership/failure boundaries.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

const SOURCE: &str = r#"#Update(initial<[K]:2,3>, peer<[K]:2,3>, rounds<u64>, marker<K>, failpass<u64>) => <([K]:2,3,[K]:2,3)>
  | :Step(a<[K]:2,3>, b<[K]:2,3>, pass<u64>)
  | :Done(a<[K]:2,3>, b<[K]:2,3>).
#Update(initial, peer, rounds, marker, failpass) -> :Step(initial,peer,0u64)
  :Step(a,b,pass)
    | pass < rounds -> {
        next-a := [[((i == 1u64)? | true => marker | false => a[i,j]) | i <- 1u64..=2u64]' | j <- 1u64..=3u64]
        checked := next-a[((pass == failpass)? | true => 0u64 | false => 2u64),3u64]
        ~next-b := b
        next-b[1u64,1u64] = marker
        next-b[2u64,3u64] = checked
      } -> :Step(next-a,next-b,pass + 1u64)
    | * -> :Done(a,b)
  :Done(a,b) => (a,b).
data := input<[K]:2,3>
peer-data := peer<[K]:2,3>
answer := #Update(data,peer-data,rounds<u64>,marker<K>,failpass<u64>)
(answer,data,peer-data)
"#;
fn output(values: &[Option<Value>]) -> Vec<Option<Data>> {
    values
        .iter()
        .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
        .collect()
}
fn run(
    jit: &JitProgramArtifact,
    args: &[Value],
    out: &mut [Option<Value>],
    policy: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
    restored: bool,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    observe::Counts,
) {
    if restored {
        observe::run_restored(jit, args, out, policy, memory)
    } else {
        observe::run(jit, args, out, policy, memory)
    }
}
#[test]
fn numeric_phase_direct_banks_keep_repeated_maps_cold_handoffs_aliases_finite_failure_and_retry() {
    for (kind, cells, marker) in super::super::typed_numeric_phases::cases() {
        for artifact in artifacts(&SOURCE.replace('K', kind)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let old = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::DirectPhaseMaps,
            )
            .unwrap();
            let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            let metadata = object.metadata();
            let guarded = [
                "compiled_readonly_numeric_loops",
                "compiled_numeric_write_loops",
            ]
            .into_iter()
            .flat_map(|key| metadata[key].as_array().unwrap())
            .map(|region| {
                region["guarded_direct_matrix_maps"]
                    .as_array()
                    .unwrap()
                    .len()
            })
            .sum::<usize>();
            assert!(guarded > 0, "actual guarded direct source map {kind}");
            let peer = cells.iter().rev().cloned().collect::<Vec<_>>();
            for restored in [false, true] {
                let memory = mode.memory_budget();
                for (rounds, failpass) in
                    [(0, u64::MAX), (1, u64::MAX), (3, u64::MAX), (3, 0), (3, 1)]
                {
                    let args = super::super::numeric_maps::typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            ("input", Data::Matrix(cells.clone().into()), &[2, 3]),
                            ("peer", Data::Matrix(peer.clone().into()), &[2, 3]),
                            ("marker", marker.clone(), &[]),
                            ("rounds", Data::U64(rounds), &[]),
                            ("failpass", Data::U64(failpass), &[]),
                        ],
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let mut actual = vec![None; artifact.outputs().len()];
                    let mut expected = actual.clone();
                    let (a, counts) = run(
                        &jit,
                        &args,
                        &mut actual,
                        mode.execution(),
                        &memory,
                        restored,
                    );
                    let (b, _) = run(
                        &old,
                        &args,
                        &mut expected,
                        mode.execution(),
                        &memory,
                        restored,
                    );
                    assert_eq!(a.is_err(), failpass < rounds);
                    assert_eq!(
                        format!("{a:?}"),
                        format!("{b:?}"),
                        "complete reports and exact source errors {kind}"
                    );
                    assert_eq!(output(&actual), output(&expected));
                    if a.is_ok() && rounds > 0 {
                        assert!(
                            !counts.first_change_sites.is_empty(),
                            "later cold scalar first change {kind}"
                        );
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                let args = super::super::numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("input", Data::Matrix(cells.clone().into()), &[2, 3]),
                        ("peer", Data::Matrix(peer.clone().into()), &[2, 3]),
                        ("marker", marker.clone(), &[]),
                        ("rounds", Data::U64(3), &[]),
                        ("failpass", Data::U64(u64::MAX), &[]),
                    ],
                );
                let aliases = args.clone();
                let before = contents(&args);
                let mut prior = vec![None; artifact.outputs().len()];
                run(&jit, &args, &mut prior, mode.execution(), &memory, restored)
                    .0
                    .unwrap();
                let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                for field in 0..5 {
                    let mut policy = mode.execution();
                    match field {
                        0 => policy.execution.max_work = 1,
                        1 => policy.library.max_compute_work = Some(0),
                        2 => policy.library.max_cloned_bytes = Some(0),
                        3 => policy.library.max_output_elements = Some(0),
                        4 => policy.library.max_comparison_work = Some(0),
                        _ => unreachable!(),
                    }
                    let mut actual = prior.clone();
                    let mut expected = prior.clone();
                    let (a, _) = run(&jit, &args, &mut actual, policy, &memory, restored);
                    let (b, _) = run(&old, &args, &mut expected, policy, &memory, restored);
                    assert!(a.is_err(), "finite denial {field}, {kind}: {a:?}");
                    assert_eq!(format!("{a:?}"), format!("{b:?}"));
                    assert_eq!(output(&actual), output(&expected));
                    for (now, previous) in actual.iter().zip(&prior) {
                        assert!(
                            now.as_ref()
                                .unwrap()
                                .shares_frozen_storage(previous.as_ref().unwrap()),
                            "atomic prior publication"
                        );
                    }
                    let (a, _) = run(
                        &jit,
                        &args,
                        &mut actual,
                        mode.execution(),
                        &memory,
                        restored,
                    );
                    let (b, _) = run(
                        &old,
                        &args,
                        &mut expected,
                        mode.execution(),
                        &memory,
                        restored,
                    );
                    assert_eq!(a.unwrap(), b.unwrap());
                    assert_eq!(output(&actual), output(&prior));
                    assert_eq!(output(&actual), output(&expected));
                    drop((actual, expected));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop((prior, args, aliases));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
