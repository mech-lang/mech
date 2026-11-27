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
            diagnostic["selected"], false,
            "source candidates cannot lend runtime views"
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
fn numeric_regions_candidate_metadata_keeps_actual_checked_execution_unchanged() {
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
        let mut previous = None;
        for restored in [false, true] {
            let mut output = vec![None; artifact.outputs().len()];
            let (result, _) = if restored {
                observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
            } else {
                observe::run(&jit, &args, &mut output, mode.execution(), &memory)
            };
            let report = format!("{:?}", result.unwrap());
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
