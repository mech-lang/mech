//! Ordinary loop composition must fit the existing proof-storage byte budget.
use super::{artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeCompileLimits};

#[test]
fn nested_current_visit_definition_tables_keep_exact_admission_diagnostics() {
    use mech_engine::native_artifact::{
        NativeArtifactLoweringError as Error,
        lower_native_program_artifact_with_definition_table_limit as lower,
    };
    let source = "+> sequence\n[sequence/length([x + y | y <- values]) | x <- values<[u64]>]\n";
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(source).0 {
        let error = lower(&artifact, &catalog, 0).unwrap_err();
        let Error::DefinitionTableLimit {
            stage,
            required,
            limit: 0,
        } = error
        else {
            panic!("{error:?}")
        };
        assert_eq!(stage, "current visits before lowering");
        assert!(required > 0);
        let plan = lower(&artifact, &catalog, required).unwrap();
        assert!(plan.iterations().iter().any(|i| match i {
            mech_engine::native_artifact::NativeControlLoop::Collection(i) => i.parent().is_some(),
            _ => false,
        }));
        assert_eq!(plan.definition_table_byte_limit(), required);
        assert_eq!(
            lower(&artifact, &catalog, required - 1).unwrap_err(),
            Error::DefinitionTableLimit {
                stage,
                required,
                limit: required - 1
            }
        );
        let mut compilation = NativeCompileLimits::default();
        compilation.max_definition_table_bytes = required;
        JitProgramArtifact::compile_with_limits(&artifact, &catalog, compilation).unwrap();
        compilation.max_definition_table_bytes = required - 1;
        let error = JitProgramArtifact::compile_with_limits(&artifact, &catalog, compilation)
            .err()
            .unwrap()
            .to_string();
        assert!(
            error.contains("DefinitionTableLimit")
                && error.contains(stage)
                && error.contains(&required.to_string()),
            "{error}"
        );
    }
}

#[test]
fn sequential_recursive_calls_fit_compact_proof_rows_and_recover_after_denial() {
    let mut source = String::from(
        "walk(n<u64>) => <u64> | n => ((n > 0u64)? | true => walk(n - 1u64) | false => n).\n",
    );
    for index in 0..320 {
        source.push_str(&format!(
            "unused-{index} := walk(input<u64> + {index}u64)\n"
        ));
    }
    source.push_str("walk(input)\n");
    for artifact in artifacts(&source).0 {
        let plan = mech_engine::native_artifact::lower_native_program_artifact(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        assert!(plan.blocks().len() * plan.values().len() > 16 * 1024 * 1024);
        println!(
            "compact-proof blocks={} values={} old_dense_cells={}",
            plan.blocks().len(),
            plan.values().len(),
            plan.blocks().len() * plan.values().len()
        );
        // This is the existing explicit full-program diagnostic profile. The
        // independent generated-block ceiling and all defaults are unchanged.
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeCompileLimits {
                max_nodes: 16_384,
                ..Default::default()
            },
        )
        .unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(&artifact, &[("input", Data::U64(0))], &memory);
        let aliases = args.clone();
        let mut output = vec![None; artifact.outputs().len()];
        for denied in [false, true, false] {
            let old = output.clone();
            let retained = memory.used_bytes();
            let mut policy = limits();
            if denied {
                policy.execution.max_work = 1;
            }
            let result = jit.evaluate_into(&args, &mut output, policy, &memory);
            if denied {
                assert!(result.is_err());
                assert_eq!(memory.used_bytes(), retained);
                assert!(old.iter().zip(&output).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
            } else {
                result.unwrap();
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(0)
                );
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
        }
        drop((args, aliases, output));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}
