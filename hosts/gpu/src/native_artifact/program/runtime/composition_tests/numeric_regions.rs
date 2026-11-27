//! Real source DAGs, semantic escapes and disjoint private-bank lifetimes.
use super::*;
use crate::native_artifact::program::prepare::numeric_binary::Kind;
use crate::native_artifact::program::prepare::numeric_regions::{Candidate, Source, Step};
use mech_engine::native_artifact::{
    NativeIterationTerminator as Terminator, NativeMatchTerminator as Term,
    NativeProgramReadRole as Read,
};

const CHAIN: &str = "a := left<[f64]> + right<[f64]>\nb := a * gain<f64>\nb - left<[f64]>\n";

fn check(candidate: &Candidate, plan: &mech_engine::native_artifact::NativeProgramArtifactPlan) {
    let facts = plan.facts();
    let sites = candidate
        .steps
        .iter()
        .map(Step::site)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(candidate.steps[0].site(), candidate.entry);
    assert!(!sites.contains(&candidate.exit.unwrap_or(u32::MAX)));
    let mut starts = Vec::new();
    for (index, step) in candidate.steps.iter().enumerate() {
        let block = &plan.blocks()[step.site() as usize];
        assert_eq!(block.iteration(), None);
        assert!(matches!(
            block.terminator(),
            Terminator::Scalar(Term::Jump(_) | Term::Complete)
        ));
        let (inputs, output) = match step {
            Step::Operator {
                inputs,
                output,
                bank,
                ..
            } => {
                assert_eq!(*bank, starts.len());
                starts.push(index);
                (inputs.as_ref(), Some(*output))
            }
            Step::Alias { source, output, .. } => (core::slice::from_ref(source), Some(*output)),
            Step::Empty { .. } => (&[][..], None),
        };
        for input in inputs {
            match *input {
                Source::External(at) => assert!(at < candidate.external.len()),
                Source::Bank(bank) => {
                    assert!(starts[bank] < index);
                    assert!(candidate.last_reads[bank] >= index);
                }
            }
        }
        if let Some(output) = output {
            let escapes = facts
                .read_roles(output)
                .unwrap()
                .iter()
                .any(|role| !matches!(role, Read::Action(site) if sites.contains(site)));
            assert_eq!(candidate.outputs.iter().any(|o| o.value == output), escapes);
        }
    }
    assert_eq!(starts.len(), candidate.storage_slots.len());
    for (bank, &slot) in candidate.storage_slots.iter().enumerate() {
        assert!(slot < candidate.slots);
        for earlier in 0..bank {
            if candidate.storage_slots[earlier] == slot {
                assert!(
                    candidate.last_reads[earlier] < starts[bank],
                    "bank alias overlaps an input/store lifetime"
                );
            }
        }
    }
    for output in &candidate.outputs {
        if let Source::Bank(bank) = output.source {
            assert!(!candidate.private_banks.contains(&bank));
            assert_eq!(candidate.last_reads[bank], candidate.steps.len());
        }
    }
}

#[test]
fn numeric_regions_chain_reuses_only_dead_intermediates_and_rederives_after_image_load() {
    let mut expected = None;
    for artifact in artifacts(CHAIN) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let m = jit.descriptor().metadata();
        assert_eq!(
            m.numeric_regions.candidates.len(),
            1,
            "{:#?}",
            m.numeric_regions
        );
        let candidate = &m.numeric_regions.candidates[0];
        check(candidate, &m.plan);
        assert_eq!(candidate.storage_slots.len(), 3);
        assert_eq!(candidate.private_banks.len(), 2);
        assert_eq!(candidate.slots, 2);
        let inventory = serde_json::to_value(&m.numeric_regions).unwrap();
        assert_eq!(&inventory, expected.get_or_insert(inventory.clone()));
        let restored = super::super::tests::restore_admitted_helper_descriptor(&jit);
        assert_eq!(restored.metadata().numeric_regions, m.numeric_regions);
        let encoded = jit.descriptor().encode_build_owned_image().unwrap();
        assert!(
            !encoded
                .windows(b"numeric_regions".len())
                .any(|w| w == b"numeric_regions")
        );
    }
}

#[test]
fn numeric_regions_shared_and_escaping_values_retain_every_semantic_read() {
    for source in [
        "a := left<[f64]> + right<[f64]>\nb := a * gain<f64>\n(a,b)\n",
        "a := left<[f64]> + right<[f64]>\nb := a * gain<f64>\n(a ** peer<[f64]>, b + a)\n",
    ] {
        for artifact in artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let m = jit.descriptor().metadata();
            assert_eq!(
                m.numeric_regions.candidates.len(),
                1,
                "{:#?}",
                m.numeric_regions
            );
            let candidate = &m.numeric_regions.candidates[0];
            check(candidate, &m.plan);
            assert_eq!(
                candidate.slots,
                candidate.storage_slots.len(),
                "live aliases cannot pool banks"
            );
            if candidate.storage_slots.len() == 2 {
                assert!(candidate.private_banks.is_empty());
            } else {
                assert_eq!(candidate.storage_slots.len(), 4);
            }
        }
    }
}

#[test]
fn numeric_regions_inventory_retains_product_transpose_reduction_and_broadcast_semantics() {
    let source = "+> stats\na := left<[f64]> ** right<[f64]>\nb := a'\nc := stats/sum/row(b)\nc * gain<f64>\n";
    for artifact in artifacts(source) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let m = jit.descriptor().metadata();
        assert_eq!(
            m.numeric_regions.candidates.len(),
            1,
            "{:#?}",
            m.numeric_regions
        );
        let candidate = &m.numeric_regions.candidates[0];
        check(candidate, &m.plan);
        let kinds = candidate
            .steps
            .iter()
            .filter_map(|step| match step {
                Step::Operator { kind, .. } => Some(*kind),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(matches!(
            kinds.as_slice(),
            [
                Kind::Product,
                Kind::Transpose { .. },
                Kind::Sum { axis: 0, .. },
                Kind::Elementwise(_)
            ]
        ));
        assert_eq!(candidate.private_banks.len(), 3);
        assert_eq!(candidate.slots, 2);
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        let diagnostic = &object.metadata()["matrix_expression_candidates"];
        assert_eq!(
            diagnostic["selected"], true,
            "source-closed region emission remains subject to current runtime admission"
        );
        assert_eq!(
            diagnostic["inventory"],
            serde_json::to_value(&m.numeric_regions).unwrap()
        );
        let restored = super::super::tests::restore_admitted_helper_descriptor(&jit);
        assert_eq!(restored.metadata().numeric_regions, m.numeric_regions);
    }
}

#[test]
fn numeric_regions_never_cross_branches_unknown_calls_or_tail_iterations() {
    let cases = [
        (
            "a := flag<bool>? | true => left<[f64]> + right<[f64]> | false => left - right.\na * gain<f64>\n",
            0,
        ),
        (
            "flag<bool>? | true => (left<[f64]> + right<[f64]>) * gain<f64> | false => (left - right) / gain.\n",
            2,
        ),
        (
            "+> sequence\na := left<[f64]> + right<[f64]>\nn := sequence/length(words<[u64]>)\n(a * gain<f64>, n)\n",
            0,
        ),
        (
            "scan(a<[f64]>,b<[f64]>,at<u64>) => (out<[f64]>) :=\n  next := (a + b) * 2.0\n  out := ((at < 2u64)? | true => scan(next,b,at + 1u64) | false => next).\nscan(left<[f64]>,right<[f64]>,0u64)\n",
            0,
        ),
    ];
    for (source, count) in cases {
        for artifact in artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let m = jit.descriptor().metadata();
            assert_eq!(
                m.numeric_regions.candidates.len(),
                count,
                "{source}: {:#?}",
                m.numeric_regions
            );
            for candidate in &m.numeric_regions.candidates {
                check(candidate, &m.plan);
            }
        }
    }
}

#[test]
fn numeric_regions_current_loan_runs_chain_and_preserves_restored_execution() {
    use mech_core::snapshot::F64Bits;
    let data = |cells: &[f64]| {
        Data::Matrix(
            cells
                .iter()
                .map(|v| Data::F64(F64Bits::from_f64(*v)))
                .collect(),
        )
    };
    for artifact in artifacts(CHAIN) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let individual = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            crate::native_artifact::codegen::options::OptimizationPass::MatrixExpressionRegions,
        )
        .unwrap();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let args = super::numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                ("left", data(&[1.0, 2.0, 3.0, 4.0]), &[2, 2]),
                ("right", data(&[0.5; 4]), &[2, 2]),
                ("gain", Data::F64(F64Bits::from_f64(2.0)), &[]),
            ],
        );
        let mut reference = vec![None; artifact.outputs().len()];
        let (ordinary, ordinary_counts) = observe::run(
            &individual,
            &args,
            &mut reference,
            mode.execution(),
            &memory,
        );
        let ordinary = ordinary.unwrap();
        let finished = |counts: &observe::Counts| {
            counts
                .helpers
                .iter()
                .filter(|((_, op), _)| *op == GlobalHelper::NumericBinaryFinish.raw())
                .map(|(_, n)| *n)
                .sum::<u64>()
        };
        assert_eq!(finished(&ordinary_counts), 3);
        let mut previous = None;
        for restored in [false, true] {
            let mut output = vec![None; artifact.outputs().len()];
            let (result, counts) = if restored {
                observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
            } else {
                observe::run(&jit, &args, &mut output, mode.execution(), &memory)
            };
            let result = result.unwrap();
            assert_eq!(
                finished(&counts),
                1,
                "one complete region, not three individual loans: {:?}",
                counts.helpers
            );
            assert!(
                result.work >= ordinary.work,
                "all original source bills remain funded"
            );
            assert!(result.comparison_work >= ordinary.comparison_work);
            let report = format!("{result:?}");
            let actual = output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap();
            assert_eq!(actual, data(&[2.0, 3.0, 4.0, 5.0]));
            assert_eq!(&report, previous.get_or_insert_with(|| report.clone()));
        }
    }
}

fn matrix(cells: &[f64]) -> Data {
    Data::Matrix(
        cells
            .iter()
            .map(|&v| Data::F64(mech_core::snapshot::F64Bits::from_f64(v)))
            .collect(),
    )
}
fn output_data(values: &[Option<Value>]) -> Vec<Data> {
    values
        .iter()
        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}
fn compare_current_execution(
    source: &str,
    artifact: &ProgramArtifact,
    native: &JitProgramArtifact,
    individual: &JitProgramArtifact,
    named: &[(&str, Data, &[u64])],
    selected: bool,
) -> [u64; 2] {
    let mode = NativeProgramLimitMode::NoLimits;
    let memory = mode.memory_budget();
    let args = numeric_maps::typed_arguments(artifact, &memory, named);
    let before = contents(&args);
    let mut expected = vec![None; artifact.outputs().len()];
    let (reference, counts) =
        observe::run(individual, &args, &mut expected, mode.execution(), &memory);
    assert_eq!(counts.matrix_region_completions, 0);
    let baseline = (memory.used_bytes(), memory.retained_nodes());
    let mut cancellations = [0; 2];
    for restored in [false, true] {
        let mut actual = vec![None; artifact.outputs().len()];
        let (result, counts) = if restored {
            observe::run_restored(native, &args, &mut actual, mode.execution(), &memory)
        } else {
            observe::run(native, &args, &mut actual, mode.execution(), &memory)
        };
        assert_eq!(
            result.is_ok(),
            reference.is_ok(),
            "{source}: {result:?} / {reference:?}"
        );
        if let (Ok(a), Ok(b)) = (&result, &reference) {
            assert_eq!(
                output_data(&actual),
                output_data(&expected),
                "exact bits: {source}"
            );
            assert_eq!(
                counts.matrix_region_completions,
                u64::from(selected),
                "selection: {source}: {:?}",
                counts.helpers
            );
            assert!(
                a.work >= b.work,
                "all original source work: {source}: {a:?} / {b:?}"
            );
            assert!(a.comparison_work >= b.comparison_work);
            assert_eq!(a.visits, b.visits, "every original source visit remains");
        } else {
            assert!(actual.iter().all(Option::is_none));
            assert_eq!(counts.matrix_region_completions, 0);
        }
        cancellations[usize::from(restored)] = counts.matrix_region_cancellations;
        assert_eq!(contents(&args), before);
        drop(actual);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
    }
    drop((args, expected));
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    cancellations
}
fn compiled_pair(artifact: &ProgramArtifact) -> (JitProgramArtifact, JitProgramArtifact) {
    let catalog = mech_stdlib::source_catalog();
    (
        JitProgramArtifact::compile(artifact, &catalog).unwrap(),
        JitProgramArtifact::compile_for_test(
            artifact,
            &catalog,
            crate::native_artifact::codegen::options::OptimizationPass::MatrixExpressionRegions,
        )
        .unwrap(),
    )
}

#[test]
fn numeric_regions_dynamic_broadcasts_pool_only_dead_banks_and_preserve_bits() {
    for artifact in artifacts(CHAIN) {
        let (native, individual) = compiled_pair(&artifact);
        for (ld, rd) in [
            ([2u64, 3], [2u64, 3]),
            ([3, 3], [1, 3]),
            ([2, 3], [2, 1]),
            ([1, 3], [2, 1]),
            ([2, 1], [1, 3]),
            ([1, 1], [2, 3]),
            ([2, 3], [1, 1]),
            ([1, 1], [1, 1]),
            ([0, 3], [0, 3]),
            ([2, 0], [2, 0]),
        ] {
            let left = (0..ld[0] * ld[1])
                .map(|i| if i % 3 == 0 { -0.0 } else { i as f64 + 0.25 })
                .collect::<Vec<_>>();
            let right = (0..rd[0] * rd[1])
                .map(|i| if i % 2 == 0 { 0.5 } else { -2.0 })
                .collect::<Vec<_>>();
            compare_current_execution(
                CHAIN,
                &artifact,
                &native,
                &individual,
                &[
                    ("left", matrix(&left), &ld),
                    ("right", matrix(&right), &rd),
                    (
                        "gain",
                        Data::F64(mech_core::snapshot::F64Bits::from_f64(2.0)),
                        &[],
                    ),
                ],
                true,
            );
        }
    }
}

#[test]
fn numeric_regions_ordered_products_transposes_reductions_and_produced_scalar_broadcasts() {
    for axis in ["row", "column"] {
        let source = format!(
            "+> stats\na := left<[f64]> ** right<[f64]>\nb := a'\nc := stats/sum/{axis}(b)\nc * gain<f64>\n"
        );
        for artifact in artifacts(&source) {
            let (native, individual) = compiled_pair(&artifact);
            for (ld, rd) in [
                ([2u64, 3], [3u64, 5]),
                ([3, 5], [5, 3]),
                ([1, 1], [1, 1]),
                ([2, 0], [0, 3]),
                ([0, 3], [3, 2]),
                ([2, 3], [3, 0]),
                ([0, 0], [0, 0]),
            ] {
                let left = (0..ld[0] * ld[1])
                    .map(|i| {
                        if i % 4 == 0 {
                            -0.0
                        } else {
                            i as f64 / 7.0 - 1.0
                        }
                    })
                    .collect::<Vec<_>>();
                let right = (0..rd[0] * rd[1])
                    .map(|i| i as f64 / 11.0 - 0.5)
                    .collect::<Vec<_>>();
                compare_current_execution(
                    &source,
                    &artifact,
                    &native,
                    &individual,
                    &[
                        ("left", matrix(&left), &ld),
                        ("right", matrix(&right), &rd),
                        (
                            "gain",
                            Data::F64(mech_core::snapshot::F64Bits::from_f64(-2.0)),
                            &[],
                        ),
                    ],
                    true,
                );
            }
        }
    }
    let source =
        "+> stats\na := stats/sum/row(input<[f64]>)\nb := stats/sum/column(a)\nb + peer<[f64]>\n";
    for artifact in artifacts(source) {
        let (native, individual) = compiled_pair(&artifact);
        compare_current_execution(
            source,
            &artifact,
            &native,
            &individual,
            &[
                ("input", matrix(&[1.0, 2.0, -0.0, -1.0, 0.25, 3.0]), &[2, 3]),
                ("peer", matrix(&[0.5, 1.0, 2.0, 3.0, 4.0, 5.0]), &[2, 3]),
            ],
            true,
        );
    }
}

#[test]
fn numeric_regions_shared_escaping_results_keep_independent_owners_and_alias_reads() {
    for source in [
        "a := left<[f64]> + right<[f64]>\nalias := a\nb := alias * gain<f64>\n(a, alias, b)\n",
        "a := left<[f64]> + right<[f64]>\nb := a * gain<f64>\n(a ** peer<[f64]>, b + a)\n",
    ] {
        for artifact in artifacts(source) {
            let (native, individual) = compiled_pair(&artifact);
            compare_current_execution(
                source,
                &artifact,
                &native,
                &individual,
                &[
                    ("left", matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]), &[2, 3]),
                    ("right", matrix(&[0.5, -0.0, 2.0]), &[1, 3]),
                    ("peer", matrix(&[0.5, 1.0, 2.0, 3.0, 4.0, 5.0]), &[3, 2]),
                    (
                        "gain",
                        Data::F64(mech_core::snapshot::F64Bits::from_f64(2.0)),
                        &[],
                    ),
                ],
                true,
            );
        }
    }
}

#[test]
fn numeric_regions_nonfinite_results_cancel_to_maintained_kernels() {
    let source = "a := left<[f64]> + right<[f64]>\nb := a * gain<f64>\nb - left<[f64]>\n";
    for artifact in artifacts(source) {
        let (native, individual) = compiled_pair(&artifact);
        for (left, right) in [
            (f64::from_bits(0xfff8000000000037), 1.0),
            (f64::from_bits(0x7ff0000000000042), 2.0),
            (f64::INFINITY, f64::NEG_INFINITY),
            (f64::MAX, f64::MAX),
        ] {
            compare_current_execution(
                source,
                &artifact,
                &native,
                &individual,
                &[
                    ("left", matrix(&[left, -0.0, 1.0]), &[1, 3]),
                    ("right", matrix(&[right, 0.5, 2.0]), &[1, 3]),
                    (
                        "gain",
                        Data::F64(mech_core::snapshot::F64Bits::from_f64(2.0)),
                        &[],
                    ),
                ],
                false,
            );
        }
    }
    let source = "a := left<[f64]> + right<[f64]>\nb := a / divisor<[f64]>\nb * 2.0\n";
    for artifact in artifacts(source) {
        let (native, individual) = compiled_pair(&artifact);
        for divisor in [
            2.0,
            -0.25,
            0.0,
            -0.0,
            f64::from_bits(1),
            f64::INFINITY,
            f64::NAN,
        ] {
            compare_current_execution(
                source,
                &artifact,
                &native,
                &individual,
                &[
                    ("left", matrix(&[1.0, 2.0, 3.0]), &[1, 3]),
                    ("right", matrix(&[0.5]), &[1, 1]),
                    ("divisor", matrix(&[divisor]), &[1, 1]),
                ],
                divisor == 2.0 || divisor == -0.25 || divisor.is_infinite(),
            );
        }
    }
}

#[test]
fn numeric_regions_finite_caps_keep_prior_outputs_and_release_all_private_claims() {
    for artifact in artifacts(CHAIN) {
        let (native, individual) = compiled_pair(&artifact);
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let args = numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                ("left", matrix(&[1.0, 2.0, 3.0]), &[1, 3]),
                ("right", matrix(&[0.5]), &[1, 1]),
                (
                    "gain",
                    Data::F64(mech_core::snapshot::F64Bits::from_f64(2.0)),
                    &[],
                ),
            ],
        );
        let mut prior = vec![None; artifact.outputs().len()];
        observe::run(&native, &args, &mut prior, mode.execution(), &memory)
            .0
            .unwrap();
        let baseline = (memory.used_bytes(), memory.retained_nodes());
        for field in 0..12 {
            let mut policy = mode.execution();
            match field {
                0 => policy.library.max_output_elements = Some(0),
                1 => policy.library.max_output_bytes = Some(0),
                2 => policy.library.max_temporary_bytes = Some(0),
                3 => policy.library.max_cloned_bytes = Some(0),
                4 => policy.library.max_retained_nodes = Some(0),
                5 => policy.library.max_compute_work = Some(0),
                6 => policy.library.max_comparison_work = Some(0),
                7 => policy.library.max_scalar_instructions = None,
                8 => policy.library.max_transfer_bytes = None,
                9 => policy.library.max_storage_buffer_bytes = None,
                10 => policy.library.max_storage_bindings = None,
                11 => policy.execution.max_work = 1,
                _ => unreachable!(),
            }
            for restored in [false, true] {
                let mut actual = prior.clone();
                let mut expected = prior.clone();
                let (a, counts) = if restored {
                    observe::run_restored(&native, &args, &mut actual, policy, &memory)
                } else {
                    observe::run(&native, &args, &mut actual, policy, &memory)
                };
                let (b, _) = observe::run(&individual, &args, &mut expected, policy, &memory);
                assert_eq!(a.is_ok(), b.is_ok(), "cap{field}: {a:?} / {b:?}");
                assert_eq!(counts.matrix_region_completions, 0);
                assert_eq!(output_data(&actual), output_data(&expected));
                if a.is_err() {
                    for (a, p) in actual.iter().zip(&prior) {
                        assert!(
                            a.as_ref()
                                .unwrap()
                                .shares_frozen_storage(p.as_ref().unwrap())
                        );
                    }
                }
                drop((actual, expected));
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
            }
        }
        let mut output = prior.clone();
        let (result, counts) =
            observe::run_restored(&native, &args, &mut output, mode.execution(), &memory);
        result.unwrap();
        assert_eq!(counts.matrix_region_completions, 1);
        assert_eq!(output_data(&output), output_data(&prior));
        drop((output, prior, args));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

#[test]
fn numeric_regions_physical_peak_refusal_replays_source_then_retries_whole_region() {
    for artifact in artifacts(CHAIN) {
        let (native, individual) = compiled_pair(&artifact);
        let mode = NativeProgramLimitMode::NoLimits;
        // Admit the complete declared source peak before looking for its
        // refusal boundary. This is a standalone account, not a benchmark cap.
        let m = native.descriptor().metadata();
        let demands = m.numeric_regions.candidates[0]
            .steps
            .iter()
            .filter_map(|step| {
                if let Step::Operator { site, .. } = step {
                    Some(
                        m.calls[*site as usize]
                            .as_ref()
                            .unwrap()
                            .f64_matrix_binary_source_bound(6, 3)
                            .unwrap()
                            .0,
                    )
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let peak = demands.iter().map(|d| d.turn_peak_bytes).sum::<u64>();
        let node_peak = demands.iter().map(|d| d.retained_nodes).sum::<u64>();
        let byte_limit = peak
            .checked_mul(2)
            .unwrap()
            .checked_add(32 * 1024 * 1024)
            .unwrap();
        let memory = ManagedMemoryBudget::new(byte_limit).with_limits(
            byte_limit,
            node_peak
                .checked_mul(2)
                .unwrap()
                .checked_add(10000)
                .unwrap(),
        );
        let args = numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                ("left", matrix(&[1.0, 2.0, 3.0]), &[1, 3]),
                ("right", matrix(&[0.5]), &[1, 1]),
                (
                    "gain",
                    Data::F64(mech_core::snapshot::F64Bits::from_f64(2.0)),
                    &[],
                ),
            ],
        );
        let mut prior = vec![None; artifact.outputs().len()];
        observe::run(&individual, &args, &mut prior, mode.execution(), &memory)
            .0
            .unwrap();
        let baseline = (memory.used_bytes(), memory.retained_nodes());
        for dimension in ["bytes", "nodes"] {
            let available = if dimension == "bytes" {
                memory.limit_bytes() - baseline.0
            } else {
                memory.limit_nodes() - baseline.1
            };
            let trial = |allowance| {
                let bytes = (dimension == "bytes")
                    .then(|| memory.reserve_capacity(available - allowance).unwrap());
                let nodes = (dimension == "nodes")
                    .then(|| memory.reserve_nodes(available - allowance).unwrap());
                let mut output = prior.clone();
                let result =
                    observe::run(&individual, &args, &mut output, mode.execution(), &memory).0;
                drop((bytes, nodes, output));
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
                result.is_ok()
            };
            let (mut low, mut high) = (0, available);
            assert!(!trial(low) && trial(high));
            while low < high {
                let mid = low + (high - low) / 2;
                if trial(mid) {
                    high = mid
                } else {
                    low = mid + 1
                }
            }
            let bytes =
                (dimension == "bytes").then(|| memory.reserve_capacity(available - low).unwrap());
            let nodes =
                (dimension == "nodes").then(|| memory.reserve_nodes(available - low).unwrap());
            let mut output = prior.clone();
            let (result, counts) =
                observe::run_restored(&native, &args, &mut output, mode.execution(), &memory);
            result.unwrap();
            assert_eq!(
                counts.matrix_region_completions, 0,
                "{dimension} peak must decline"
            );
            assert_eq!(output_data(&output), output_data(&prior));
            drop((bytes, nodes, output));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
        }
        let mut output = prior.clone();
        let (result, counts) = observe::run(&native, &args, &mut output, mode.execution(), &memory);
        result.unwrap();
        assert_eq!(counts.matrix_region_completions, 1);
        drop((output, prior, args));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

mod loan_checks {
    use super::*;
    use crate::native_artifact::program::protocol::abi::{Frame, Helper, NumericBinaryFrame};
    use std::{cell::Cell, ffi::c_void};
    thread_local! {
        static ORIGINAL:Cell<Option<Helper>>=const{Cell::new(None)};
        static CASE:Cell<u8>=const{Cell::new(0)};
        static CHECKED:Cell<u64>=const{Cell::new(0)};
    }
    unsafe extern "C" fn helper(
        context: *mut c_void,
        mut site: u64,
        op: u64,
        mut a: u64,
        mut b: u64,
        mut c: u64,
        out: *mut u64,
    ) -> u64 {
        let case = CASE.with(Cell::get);
        if op == GlobalHelper::NumericBinaryPrepare.raw() && c == 1 && case == 12 {
            a ^= 1;
        }
        if op == GlobalHelper::NumericBinaryFinish.raw() && matches!(c, 1 | 2) {
            CHECKED.with(|n| n.set(n.get() + 1));
            if case == 1 {
                a = 1;
            }
            if case == 2 {
                b += 1;
            }
            if (3..=8).contains(&case) {
                // These are only the real currently-issued frames, suspended
                // after native arithmetic and before canonical publication.
                let index = if case == 4 { b as usize - 1 } else { 0 };
                let frame = unsafe { &mut *(a as *mut NumericBinaryFrame).add(index) };
                match case {
                    3 | 4 => frame.output ^= 1,
                    5 => frame.left ^= 1,
                    6 => frame.left_mode += 1,
                    7 => frame.left_scalar ^= 1,
                    8 => frame.count += 1,
                    _ => unreachable!(),
                }
            }
            let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
            match case {
                9 => invocation.state.consumed = true,
                10 => invocation.state.control.previous = None,
                11 => {
                    let visits = invocation.state.control.visits;
                    assert!(invocation.visit(site).is_err());
                    assert!(
                        invocation
                            .helper(site, GlobalHelper::Move.raw(), [0; 3])
                            .is_err()
                    );
                    assert_eq!(invocation.state.control.visits, visits);
                }
                13 => c = 0,
                14 => site += 1,
                15 => invocation.state.control.current = Some(site as u32),
                16 => c = 3,
                _ => {}
            }
        }
        unsafe { ORIGINAL.with(Cell::get).unwrap()(context, site, op, a, b, c, out) }
    }
    #[test]
    fn numeric_regions_loans_reject_forged_frames_cursor_modes_and_interleaving_then_retry() {
        for artifact in artifacts(CHAIN) {
            let (jit, _) = compiled_pair(&artifact);
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            for left in [1.0, f64::INFINITY] {
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("left", matrix(&[left, 2.0, 3.0]), &[1, 3]),
                        ("right", matrix(&[0.5]), &[1, 1]),
                        (
                            "gain",
                            Data::F64(mech_core::snapshot::F64Bits::from_f64(2.0)),
                            &[],
                        ),
                    ],
                );
                let mut prior = vec![None; artifact.outputs().len()];
                observe::run(&jit, &args, &mut prior, mode.execution(), &memory)
                    .0
                    .unwrap();
                let baseline = (memory.used_bytes(), memory.retained_nodes());
                for restored in [false, true] {
                    let restored = restored.then(|| {
                        super::super::super::tests::restore_admitted_helper_descriptor(&jit)
                    });
                    let descriptor = restored.as_ref().unwrap_or_else(|| jit.descriptor());
                    for case in 0..=16 {
                        CASE.with(|v| v.set(case));
                        CHECKED.with(|v| v.set(0));
                        let mut output = prior.clone();
                        let entry = jit.test_checked_entry();
                        let result = execute(
                            descriptor,
                            |raw: *mut Frame| unsafe {
                                let old = (*raw).helper;
                                ORIGINAL.with(|v| v.set(Some(old)));
                                (*raw).helper = helper;
                                let status = entry(raw);
                                (*raw).helper = old;
                                ORIGINAL.with(|v| v.set(None));
                                status
                            },
                            &args,
                            &mut output,
                            mode.execution(),
                            &memory,
                        );
                        assert_eq!(
                            result.is_ok(),
                            case == 0 || case == 11,
                            "case{case}: {result:?}"
                        );
                        assert_eq!(CHECKED.with(Cell::get), u64::from(case != 12));
                        if result.is_err() {
                            for (a, p) in output.iter().zip(&prior) {
                                assert!(
                                    a.as_ref()
                                        .unwrap()
                                        .shares_frozen_storage(p.as_ref().unwrap())
                                );
                            }
                        } else {
                            assert_eq!(output_data(&output), output_data(&prior));
                        }
                        drop(output);
                        assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
                        let mut output = prior.clone();
                        observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                            .0
                            .unwrap();
                        drop(output);
                        assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
                    }
                }
                drop((prior, args));
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
            }
        }
    }
}

#[test]
fn numeric_regions_transport_only_actual_scalar_homes_across_managed_captures() {
    let source = "a := left<[f64]> + first<f64>\nb := a * second<f64>\nb - third<f64>\n";
    for artifact in artifacts(source) {
        let (native, individual) = compiled_pair(&artifact);
        assert_eq!(native.descriptor().metadata().max_library_inputs, 3);
        compare_current_execution(
            source,
            &artifact,
            &native,
            &individual,
            &[
                ("left", matrix(&[1.0, 2.0, 3.0, 4.0]), &[2, 2]),
                (
                    "first",
                    Data::F64(mech_core::snapshot::F64Bits::from_f64(0.5)),
                    &[],
                ),
                (
                    "second",
                    Data::F64(mech_core::snapshot::F64Bits::from_f64(2.0)),
                    &[],
                ),
                (
                    "third",
                    Data::F64(mech_core::snapshot::F64Bits::from_f64(-1.0)),
                    &[],
                ),
            ],
            true,
        );
    }
    for artifact in artifacts(CHAIN) {
        let (native, _) = compiled_pair(&artifact);
        assert_eq!(
            native.descriptor().metadata().max_library_inputs,
            2,
            "three captures include two managed matrices; they need no transport words"
        );
    }
}

#[test]
fn numeric_regions_native_result_checks_cancel_both_simd_lanes_tail_and_later_operator() {
    let source = "a := left<[f64]> + right<[f64]>\nb := a * gain<f64>\nb - left<[f64]>\n";
    for artifact in artifacts(source) {
        let (native, individual) = compiled_pair(&artifact);
        for at in 0..3 {
            for bad in [
                f64::INFINITY,
                f64::from_bits(0xfff8000000000037),
                f64::from_bits(0x7ff0000000000042),
            ] {
                let mut left = [1.0, -0.0, 0.5];
                left[at] = bad;
                let cancellations = compare_current_execution(
                    source,
                    &artifact,
                    &native,
                    &individual,
                    &[
                        ("left", matrix(&left), &[1, 3]),
                        ("right", matrix(&[0.5]), &[1, 1]),
                        (
                            "gain",
                            Data::F64(mech_core::snapshot::F64Bits::from_f64(2.0)),
                            &[],
                        ),
                    ],
                    false,
                );
                assert_eq!(cancellations, [1, 1]);
            }
            let mut left = [1.0, -0.0, 0.5];
            left[at] = f64::MAX / 2.0;
            let cancellations = compare_current_execution(
                source,
                &artifact,
                &native,
                &individual,
                &[
                    ("left", matrix(&left), &[1, 3]),
                    ("right", matrix(&[0.5]), &[1, 1]),
                    (
                        "gain",
                        Data::F64(mech_core::snapshot::F64Bits::from_f64(4.0)),
                        &[],
                    ),
                ],
                false,
            );
            assert_eq!(
                cancellations,
                [1, 1],
                "second operator overflow must release the entire loan"
            );
        }
    }
}

#[test]
fn numeric_regions_finite_cancellation_and_computed_divisors_need_no_magnitude_proof() {
    for (source, left, right) in [
        (
            "a := left<[f64]> + right<[f64]>\nb := a * 2.0\nb + right<[f64]>\n",
            f64::MAX / 2.0,
            -f64::MAX / 2.0,
        ),
        (
            "a := left<[f64]> + right<[f64]>\nb := right / a\nb * 2.0\n",
            f64::MAX / 4.0,
            -f64::MAX / 8.0,
        ),
    ] {
        for artifact in artifacts(source) {
            let (native, individual) = compiled_pair(&artifact);
            let cancellations = compare_current_execution(
                source,
                &artifact,
                &native,
                &individual,
                &[
                    ("left", matrix(&[left, -0.0, f64::from_bits(1)]), &[1, 3]),
                    ("right", matrix(&[right, -1.0, f64::from_bits(1)]), &[1, 3]),
                ],
                true,
            );
            assert_eq!(cancellations, [0, 0]);
        }
    }
}

#[test]
fn numeric_regions_transpose_sum_and_product_nonfinite_results_replay_exact_bits() {
    let cases: &[(&str, &str, &[f64], &[u64])] = &[
        (
            "a := input<[f64]>'\nb := a * 2.0\nb + a\n",
            "input",
            &[1.0, f64::INFINITY, -0.0],
            &[1, 3],
        ),
        (
            "+> stats\na := stats/sum/row(input<[f64]>)\nb := a * 2.0\nb + a\n",
            "input",
            &[f64::MAX, f64::MAX, -0.0, 1.0],
            &[2, 2],
        ),
        (
            "a := input<[f64]> ** input<[f64]>\nb := a * 2.0\nb + a\n",
            "input",
            &[f64::MAX, 0.0, 0.0, 1.0],
            &[2, 2],
        ),
    ];
    for &(source, name, values, dimensions) in cases {
        for artifact in artifacts(source) {
            let (native, individual) = compiled_pair(&artifact);
            let cancellations = compare_current_execution(
                source,
                &artifact,
                &native,
                &individual,
                &[(name, matrix(values), dimensions)],
                false,
            );
            assert_eq!(cancellations, [1, 1]);
        }
    }
}
