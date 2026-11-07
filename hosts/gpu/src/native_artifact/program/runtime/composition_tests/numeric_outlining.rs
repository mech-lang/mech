//! Whole nested numeric computations keep one current native call boundary.
use super::*;

const SOURCE: &str = include_str!("../../../../../tests/fixtures/native/numeric-nested-phase.mec");

#[test]
fn numeric_phase_outlining_is_a_real_shared_native_definition() {
    for artifact in artifacts(SOURCE) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = object.metadata();
        let outlines = metadata
            .get("compiled_numeric_outlines")
            .and_then(serde_json::Value::as_array);
        assert!(
            outlines.is_some_and(|rows| !rows.is_empty()),
            "the complete nested buffer computation needs its own native function, not the parent's live register scope"
        );
        for row in outlines.unwrap() {
            assert!(row["code_bytes"].as_u64().unwrap() > 0);
            assert!(row["symbol"].as_str().unwrap().contains("_numeric_phase_"));
            let header = row["header"].as_u64().unwrap();
            let reports = metadata["compiled_numeric_write_loops"].as_array().unwrap();
            let body = reports
                .iter()
                .find(|body| body["header"].as_u64() == Some(header))
                .unwrap();
            assert_eq!(body["successful_backedge_helpers"], 0);
        }
    }
}

#[test]
fn numeric_phase_outlining_keeps_current_inputs_errors_publication_and_retry() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let inline = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::LoopOutlining,
        )
        .unwrap();
        let outlined =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let mut baseline = None;
        for jit in [&inline, &outlined] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut output = vec![None; artifact.outputs().len()];
                let mut records = Vec::new();
                for (steps, flag, allowed, deny, divisor, bias, failure) in [
                    (3, true, true, u64::MAX, 1, 7, None),
                    (3, false, true, u64::MAX, 2, 19, None),
                    (3, true, false, u64::MAX, 0, 0, Some("Integrity")),
                    (3, false, true, 1, 1, 0, Some("Integrity")),
                    (3, true, true, u64::MAX, 0, 0, Some("Arithmetic")),
                    (
                        3,
                        true,
                        true,
                        u64::MAX,
                        1,
                        u64::MAX - 26,
                        Some("Arithmetic"),
                    ),
                    (0, false, false, 0, 0, u64::MAX, None),
                    (1, false, true, u64::MAX, 1, 43, None),
                    (3, true, true, u64::MAX, 1, 7, None),
                ] {
                    let args = nested_phase::arguments(
                        &artifact, &memory, steps, flag, allowed, deny, divisor, bias,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let prior = output.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, counts) = if restored {
                        observe::run_restored(jit, &args, &mut output, mode.execution(), &memory)
                    } else {
                        observe::run(jit, &args, &mut output, mode.execution(), &memory)
                    };
                    let receipt = if let Some(kind) = failure {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(error.contains(kind), "{error}");
                        assert!(output.iter().zip(&prior).all(|(now, old)| {
                            now.as_ref()
                                .unwrap()
                                .shares_frozen_storage(old.as_ref().unwrap())
                        }));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                        format!("error:{error}")
                    } else {
                        let report = result.unwrap();
                        assert_eq!(counts.phases, 1);
                        let prefix = if steps == 0 {
                            0
                        } else {
                            3 / divisor + 8 / divisor + 15 / divisor
                        };
                        let expected = std::iter::once(Data::U64(9))
                            .chain((0..steps).map(|at| {
                                Data::U64(
                                    prefix
                                        + at
                                        + if flag == (at % 2 == 0) { 0 } else { 100 }
                                        + bias,
                                )
                            }))
                            .collect();
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::Matrix(expected)
                        );
                        format!("success:{}", report.work)
                    };
                    records.push((
                        receipt,
                        counts.helpers,
                        contents(
                            &output
                                .iter()
                                .map(|x| x.as_ref().unwrap().clone())
                                .collect::<Vec<_>>(),
                        ),
                    ));
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                assert_eq!(&records, baseline.get_or_insert(records.clone()));
                drop(output);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn numeric_phase_outlining_retains_finite_denial_cleanup_and_recovery() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        nested_phase::arguments(artifact, memory, 3, true, true, u64::MAX, 1, 7)
    });
}

#[test]
fn numeric_phase_outlining_leaves_current_funded_child_boundaries_inline() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-parent-child.mec");
    for artifact in artifacts(source) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = object.metadata();
        assert!(
            metadata["numeric_write_regions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| {
                    r["proof"]["phase"].is_object()
                        && !r["proof"]["exits"].as_array().unwrap().is_empty()
                })
        );
        assert!(
            metadata["compiled_numeric_outlines"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn numeric_phase_outlining_declines_without_changing_the_finite_envelope() {
    use crate::native_artifact::codegen::{control::CodegenReport, options::*};
    use crate::native_artifact::program::{
        codegen::numeric_outlined, prepare::PreparedNativeProgram,
    };
    for artifact in artifacts(SOURCE) {
        let catalog = mech_stdlib::source_catalog();
        let lower = || {
            mech_engine::native_artifact::lower_native_program_artifact_with_definition_table_limit(
                &artifact,
                &catalog,
                u64::MAX,
            )
            .unwrap()
        };
        let prepared = PreparedNativeProgram::prepare(&artifact, &catalog, lower()).unwrap();
        let mut closed = PreparedNativeProgram::prepare(&artifact, &catalog, lower())
            .unwrap()
            .into_metadata()
            .closed_tails;
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &catalog,
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = object.metadata();
        for row in metadata["compiled_numeric_outlines"].as_array().unwrap() {
            let header = row["header"].as_u64().unwrap() as usize;
            let body = metadata["compiled_numeric_write_loops"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["header"].as_u64() == Some(header as u64))
                .unwrap();
            let inline_work = body["fixed_template_instructions"].as_u64().unwrap()
                - row["additional_template_bound"].as_u64().unwrap();
            closed
                .regions
                .iter_mut()
                .find(|r| r.tail.header() as usize == header)
                .unwrap()
                .numeric
                .as_mut()
                .unwrap()
                .phase
                .as_mut()
                .unwrap()
                .boundary_work = inline_work;
        }
        let mut module = cranelift_jit::JITModule::new(
            cranelift_jit::JITBuilder::new(cranelift_module::default_libcall_names()).unwrap(),
        );
        let mut report = CodegenReport::default();
        let functions = numeric_outlined::define_functions(
            &CompilationControls::new(OptimizationOptions::default()),
            &mut module,
            "finite_outline_decline",
            crate::native_artifact::codegen::control::plan::CodePlan::Program(&prepared),
            &closed,
            NativeProgramLimitMode::NoLimits.compilation(),
            &mut report,
        )
        .unwrap();
        assert!(functions.is_empty());
        assert!(report.numeric_outlines.is_empty());
        // A refused optional definition must not leave an unresolved symbol.
        module.finalize_definitions().unwrap();
    }
}
