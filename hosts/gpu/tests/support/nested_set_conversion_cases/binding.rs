//! Actual selected preparation and pre-binder admission, not a fake factory.
use super::*;
use mech_core::{MemoryBudgetDimension as Dimension, MemoryPlanError, ResidentKernelError};
use mech_engine::native_artifact::{
    NativeCollectionInvocationError as Error, NativeCollectionPrimitive, NativeIterationAction,
    NativeLibraryWorkBudget, prepare_native_program_library_call,
};

#[test]
fn recursive_selected_conversion_binding_is_funded_before_factory_construction() {
    for depth in [1, 2] {
        let mut source_kind = "[string]:1,2".to_owned();
        let mut target_kind = "{string}:1".to_owned();
        for _ in 0..depth {
            source_kind = format!("[{source_kind}]:1,1");
            target_kind = format!("[{target_kind}]:1,1");
        }
        let source = format!("unique<{target_kind}> := values<{source_kind}>\nunique\n");
        for artifact in variants(&source) {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let template = jit
                .descriptor()
                .plan()
                .blocks()
                .iter()
                .find_map(|block| match block.action() {
                    NativeIterationAction::LibraryCall { template, .. }
                        if template.primitive()
                            == NativeCollectionPrimitive::MatrixToSetConversion =>
                    {
                        Some(template)
                    }
                    _ => None,
                })
                .unwrap();
            let bound = mech_core::conversion_schema_staging_bound(
                &template.inputs()[0],
                template.output(),
            )
            .unwrap();
            let prepared =
                prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
            assert!(prepared.binding_metadata_bytes() >= bound.plan_bytes);
            assert!(prepared.binding_work() >= bound.plan_work + bound.plan_bytes * 8);
            let mut data = strings(&["z", "z"]);
            for _ in 0..depth {
                data = matrix([data]);
            }
            let input = args(&artifact, &data, &memory);
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&input, &mut output, limits(), &memory)
                .unwrap();
            let mut output = output.pop().unwrap();
            let prior = output.as_ref().unwrap().clone();
            let input = input.into_iter().map(Some).collect::<Vec<_>>();
            for dimension in [
                Dimension::ComputeWork,
                Dimension::TemporaryBytes,
                Dimension::RetainedNodes,
            ] {
                let mut denied = limits().library;
                let (required, limit) = match dimension {
                    Dimension::ComputeWork => {
                        let required = prepared.binding_work();
                        denied.max_compute_work = Some(required - 1);
                        (required, required - 1)
                    }
                    Dimension::TemporaryBytes => {
                        let required = prepared.binding_metadata_bytes();
                        denied.max_temporary_bytes = Some(required - 1);
                        (required, required - 1)
                    }
                    Dimension::RetainedNodes => {
                        denied.max_retained_nodes = Some(bound.plan_nodes - 1);
                        (bound.plan_nodes, bound.plan_nodes - 1)
                    }
                    _ => unreachable!(),
                };
                let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
                let before = memory.used_bytes();
                let error = prepared
                    .invoke_current(&input, &mut output, denied, &work)
                    .unwrap_err();
                match error {
                    Error::Kernel(ResidentKernelError::ResourceLimit {
                        dimension: actual,
                        required: actual_required,
                        limit: actual_limit,
                        ..
                    }) => {
                        assert_eq!(
                            (actual, actual_required, actual_limit),
                            (dimension, required, limit)
                        );
                    }
                    Error::Plan(MemoryPlanError::TargetLimitExceeded { violation }) => {
                        assert_eq!(
                            (violation.dimension, violation.required, violation.limit),
                            (dimension, required, limit)
                        );
                    }
                    error => {
                        panic!("expected exact pre-bind {dimension:?} boundary, got {error:?}")
                    }
                }
                assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
                assert_eq!(memory.used_bytes(), before);
            }
            let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
            prepared
                .invoke_current(&input, &mut output, limits().library, &work)
                .unwrap();
            assert!(
                output
                    .as_ref()
                    .unwrap()
                    .snapshot_eq(artifact.schemas(), &prior, artifact.schemas())
                    .unwrap()
            );
            drop((work, input, output, prior));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
