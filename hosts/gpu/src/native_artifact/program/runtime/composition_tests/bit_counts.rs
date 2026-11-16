//! Real source population counts, neighboring formulas and live intermediates.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

fn source(variant: u8, escaped: bool) -> String {
    let last_shift = if variant == 1 { 55 } else { 56 };
    let nibble_mask = if variant == 2 {
        0x0f0f_0f0f_0f0f_0f0eu64
    } else {
        0x0f0f_0f0f_0f0f_0f0fu64
    };
    let output_type = if escaped { "(u64,u64,u64,u64)" } else { "u64" };
    let result = if escaped { "(count, a, b, c)" } else { "count" };
    let append = if escaped {
        "a := sequence/append(output, selected.1)\n  b := sequence/append(a, selected.2)\n  c := sequence/append(b, selected.3)\n  out := sequence/append(c, selected.4)."
    } else {
        "out := sequence/append(output, selected)."
    };
    let width = if escaped { 4 } else { 1 };
    format!(
        r#"+> bits
+> sequence
count-word(value<u64>) => (out<{output_type}>) :=
  a := bits/wrapping-sub(value, bits/and(bits/shift-right-wrap(value, 1u64), 6148914691236517205u64))
  b := bits/wrapping-add(bits/and(a, 3689348814741910323u64), bits/and(bits/shift-right-wrap(a, 2u64), 3689348814741910323u64))
  c := bits/and(bits/wrapping-add(b, bits/shift-right-wrap(b, 4u64)), {nibble_mask}u64)
  count := bits/shift-right-wrap(bits/wrapping-mul(c, 72340172838076673u64), {last_shift}u64)
  out := {result}.
count-step(values<[u64]>, at<u64>, output<[u64]>) => (out<[u64]>) :=
  selected := count-word(values[at + 1u64])
  {append}
count-many(values<[u64]>, at<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := at < end
  next := (more? | true => count-step(values, at, output) | false => output)
  out := (more? | true => count-many(values, at + 1u64, end, next) | false => output).
data := words<[u64]>
limit := end<u64>
result := count-many(data, 0u64, limit, sequence/with-capacity(0u64, limit * {width}u64))
(result, data)
"#
    )
}

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    words: &[u64],
    end: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "words",
                Data::Matrix(words.iter().copied().map(Data::U64).collect()),
            ),
            ("end", Data::U64(end)),
        ],
        memory,
    )
}

fn oracle(value: u64, variant: u8, escaped: bool) -> Vec<Data> {
    let a = value.wrapping_sub((value >> 1) & 0x5555_5555_5555_5555);
    let b = (a & 0x3333_3333_3333_3333).wrapping_add((a >> 2) & 0x3333_3333_3333_3333);
    let c = b.wrapping_add(b >> 4)
        & if variant == 2 {
            0x0f0f_0f0f_0f0f_0f0e
        } else {
            0x0f0f_0f0f_0f0f_0f0f
        };
    let count = c.wrapping_mul(0x0101_0101_0101_0101) >> if variant == 1 { 55 } else { 56 };
    if variant == 0 {
        // Independent scalar bit summation, including the high bit and all64.
        assert_eq!(count, (0..64).map(|bit| (value >> bit) & 1).sum::<u64>());
    }
    if escaped {
        vec![count, a, b, c].into_iter().map(Data::U64).collect()
    } else {
        vec![Data::U64(count)]
    }
}

fn check(variant: u8, escaped: bool) {
    let text = source(variant, escaped);
    let mut words = vec![
        0,
        1,
        u64::MAX,
        1 << 63,
        (1 << 63) - 1,
        0x5555_5555_5555_5555,
        0xaaaa_aaaa_aaaa_aaaa,
    ];
    for bit in 0..64 {
        words.extend([1 << bit, !(1 << bit)]);
    }
    let mut seed = 0x4521_8597_abcd_ef01u64;
    for _ in 0..128 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        words.push(seed);
    }
    for artifact in artifacts(&text) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let ordinary =
            JitProgramArtifact::compile_for_test(&artifact, &catalog, OptimizationPass::BitCounts)
                .unwrap();
        if variant == 0 && !escaped {
            assert!(
                native.descriptor().code_bytes() < ordinary.descriptor().code_bytes(),
                "the actual source must select the population-count rewrite"
            );
        } else if variant != 0 {
            assert_eq!(
                native.descriptor().code_bytes(),
                ordinary.descriptor().code_bytes(),
                "a neighboring formula must retain its original expression"
            );
        }
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut outputs = [
                vec![None; artifact.outputs().len()],
                vec![None; artifact.outputs().len()],
            ];
            for end in [words.len() as u64, 0, words.len() as u64 + 1, 3] {
                let args = arguments(&artifact, &memory, &words, end);
                let aliases = args.clone();
                let before = contents(&args);
                let mut reports = Vec::new();
                for (jit, output) in [&native, &ordinary].into_iter().zip(&mut outputs) {
                    let prior = output.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let result = if restored {
                        observe::run_restored(jit, &args, output, mode.execution(), &memory).0
                    } else {
                        observe::run(jit, &args, output, mode.execution(), &memory).0
                    };
                    if end > words.len() as u64 {
                        reports.push(format!("{:?}", result.unwrap_err()));
                        assert!(output.iter().zip(prior).all(|(now, old)| {
                            now.as_ref()
                                .unwrap()
                                .shares_frozen_storage(old.as_ref().unwrap())
                        }));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    } else {
                        reports.push(format!("{:?}", result.unwrap()));
                        let expected = Data::Tuple(
                            vec![
                                Data::Matrix(
                                    words[..end as usize]
                                        .iter()
                                        .flat_map(|&word| oracle(word, variant, escaped))
                                        .collect(),
                                ),
                                Data::Matrix(words.iter().copied().map(Data::U64).collect()),
                            ]
                            .into(),
                        );
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            expected
                        );
                    }
                }
                assert_eq!(reports[0], reports[1]);
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            drop(outputs);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn bit_count_full_width_source_matches_ordinary_and_independent_counts() {
    check(0, false);
}
#[test]
fn bit_count_live_intermediates_keep_their_original_values() {
    check(0, true);
}
#[test]
fn bit_count_nearby_formulas_are_not_rewritten() {
    check(1, false);
    check(2, false);
}
#[test]
fn bit_count_finite_boundaries_keep_atomic_publication_and_retry() {
    resources::finite_boundaries(&source(0, false), |artifact, memory| {
        arguments(
            artifact,
            memory,
            &[0, u64::MAX, 1 << 63, 0x1234_5678_9abc_def0],
            4,
        )
    });
}
