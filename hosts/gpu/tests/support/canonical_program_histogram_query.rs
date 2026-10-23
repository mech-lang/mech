//! Original and decoded emitted CFGs; the resident implementation is only an oracle.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, Value, ValueDataDraft as Data};
use mech_engine::native_artifact::{NativeCollectionResult, NativeHistogramQueryStepKind as Step};
use mech_gpu::native_artifact::{JitProgramArtifact, emit_program_object};

fn source(body: &str, filter: &str) -> String {
    format!(
        "+> map\n+> text\nlookup(items<[string]>, query<string>) => (out<u64>) :=\n  words := [{body} | part <- items{filter}]\n  counts := map/histogram(words)\n  keys<{{string}}> := counts_keys\n  count := (query ∈ keys) ? | true => counts{{query}} | false => 0u64.\n  out := count.\nlookup(source<[string]>, query<string>)\n"
    )
}
fn strings(values: &[&str]) -> Data {
    Data::Matrix(
        values
            .iter()
            .map(|value| Data::String((*value).into()))
            .collect(),
    )
}
fn selected(jit: &JitProgramArtifact) -> usize {
    jit.descriptor()
        .plan()
        .collections()
        .iter()
        .filter(|collection| {
            matches!(
                collection.result(),
                NativeCollectionResult::GuardedHistogramQuery(_)
            )
        })
        .count()
}
fn prior(outputs: &[Option<Value>]) -> Vec<Value> {
    outputs
        .iter()
        .map(|value| value.as_ref().unwrap().clone())
        .collect()
}

#[test]
fn selected_query_demand_keeps_raw_utf8_counts_empty_inputs_and_fresh_queries() {
    for artifact in artifacts(&source("part", "")).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            selected(&jit),
            1,
            "must execute the composed demand, not ordinary Map lookup"
        );
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let sink = jit
            .descriptor()
            .plan()
            .collections()
            .iter()
            .find_map(|collection| {
                if let NativeCollectionResult::GuardedHistogramQuery(sink) = collection.result() {
                    Some(sink)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(
            sink.steps()
                .iter()
                .filter(|step| step.template().is_some())
                .count(),
            5
        );
        assert!(sink.steps().iter().any(|step| step.kind() == Step::Cleanup));
        let mut outputs = vec![None; artifact.outputs().len()];
        for (parts, query, count) in [
            (vec![], "", 0),
            (vec!["", "", "x"], "", 2),
            (vec!["é", "é", "é", "🧑‍🚀", "é\0", "\0"], "é", 2),
            (vec!["é", "é", "é", "🧑‍🚀", "é\0", "\0"], "é", 1),
            (vec!["a", "aa", "a\0", "a", "a\0z"], "a", 2),
            (vec!["hit", "hit", "hit", "later"], "missing", 0),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("source", strings(&parts)),
                    ("query", Data::String(query.into())),
                ],
                &memory,
            );
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
            equal(&artifact, &outputs, &reference(&artifact, &args));
            assert_eq!(
                outputs
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::U64(count)
            );
        }
        drop(outputs);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn selected_query_demand_preserves_late_body_error_and_lazy_filter_after_early_hits() {
    for filter in ["", ", part != \"skip\"", ", part != \"\""] {
        for artifact in artifacts(&source("part[1u64]", filter)).0 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(selected(&jit), 1);
            let mut outputs = vec![None; artifact.outputs().len()];
            for parts in [
                vec!["good", "great"],
                vec!["good", "", "great"],
                vec!["skip", "good", "é"],
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        ("source", strings(&parts)),
                        ("query", Data::String("g".into())),
                    ],
                    &memory,
                );
                match reference_result(&artifact, &args) {
                    Ok(expected) => {
                        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                            .unwrap();
                        equal(&artifact, &outputs, &expected);
                    }
                    Err(_) => {
                        let old = prior(&outputs);
                        let used = memory.used_bytes();
                        let error = jit
                            .evaluate_into(&args, &mut outputs, limits(), &memory)
                            .unwrap_err();
                        let expected_origin = jit.descriptor().plan().blocks().iter().find_map(|block| {
                            match block.action() {
                                mech_engine::native_artifact::NativeIterationAction::LibraryCall { template, .. }
                                    if template.primitive() == mech_engine::native_artifact::NativeCollectionPrimitive::StringScalarAccess => Some(template.origin()),
                                _ => None,
                            }
                        }).unwrap();
                        assert!(
                            matches!(error, mech_gpu::native_artifact::NativeProgramExecutionError::Library { origin, .. }
                            if origin == expected_origin),
                            "late selector must report its genuine original typed site: {error:?}"
                        );
                        equal(&artifact, &outputs, &old);
                        assert_eq!(memory.used_bytes(), used);
                    }
                }
            }
            drop(outputs);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn selected_query_demand_outer_loop_does_not_reuse_a_count_or_old_value() {
    let source = source("part", "");
    let source = source
        .rsplit_once("lookup(source<[string]>, query<string>)")
        .unwrap()
        .0
        .to_owned()
        + "text/join([number/format(lookup(source<[string]>, query), 0u64) | query <- queries<[string]>], \"|\")\n";
    let source = "+> number\n".to_owned() + &source;
    for artifact in artifacts(&source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(selected(&jit), 1);
        let args = inputs(
            &artifact,
            &[
                ("source", strings(&["x", "é", "x", "", "🧑‍🚀"])),
                ("queries", strings(&["x", "missing", "é", "", "x", "🧑‍🚀"])),
            ],
            &memory,
        );
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        equal(&artifact, &outputs, &reference(&artifact, &args));
        assert_eq!(
            outputs
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            Data::String("2|0|1|1|2|1".into())
        );
        drop((outputs, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn selected_query_demand_all_finite_dimensions_and_global_denials_are_atomic_and_recover() {
    for artifact in artifacts(&source("part", "")).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(selected(&jit), 1);
        let args = inputs(
            &artifact,
            &[
                ("source", strings(&["é\0", "x", "é\0", "later"])),
                ("query", Data::String("é\0".into())),
            ],
            &memory,
        );
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        let old = prior(&outputs);
        for dimension in 0..7 {
            // `old` deliberately pins the original publication while each
            // successful recovery may own a fresh, equal canonical output.
            let used = memory.used_bytes();
            let mut denied = limits();
            match dimension {
                0 => denied.library.max_output_elements = Some(0),
                1 => denied.library.max_output_bytes = Some(0),
                2 => denied.library.max_temporary_bytes = Some(0),
                3 => denied.library.max_cloned_bytes = Some(0),
                4 => denied.library.max_retained_nodes = Some(0),
                5 => denied.library.max_compute_work = Some(0),
                6 => denied.library.max_comparison_work = Some(0),
                _ => unreachable!(),
            }
            assert!(
                jit.evaluate_into(&args, &mut outputs, denied, &memory)
                    .is_err(),
                "dimension {dimension}"
            );
            equal(&artifact, &outputs, &old);
            assert_eq!(memory.used_bytes(), used);
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
        }
        let used = memory.used_bytes();
        let ballast = memory
            .reserve_capacity(memory.limit_bytes() - used - 1)
            .unwrap();
        let denied_used = memory.used_bytes();
        assert!(
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .is_err()
        );
        assert_eq!(memory.used_bytes(), denied_used);
        equal(&artifact, &outputs, &old);
        drop(ballast);
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        equal(&artifact, &outputs, &old);
        drop((outputs, args, old));
        assert_eq!(memory.used_bytes(), 0);
    }
}
