//! Failure publication/cleanup at the real selected nested conversion.
use super::*;

#[test]
fn nested_checked_cast_and_all_finite_dimensions_preserve_publication_and_recover() {
    use mech_core::{MemoryBudgetDimension as Dimension, MemoryPlanError, ResidentKernelError};
    use mech_engine::native_artifact::NativeCollectionInvocationError as LeafError;
    use mech_gpu::native_artifact::NativeProgramExecutionError as Error;
    let data = |values: &[f64]| {
        option(Some(matrix(
            values
                .iter()
                .map(|value| Data::F64(F64Bits::from_f64(*value))),
        )))
    };
    for artifact in variants("unique<{i64}?> := values<[f64]?>\nunique\n") {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let good = args(&artifact, &data(&[1.1, 1.9, 2.1]), &memory);
        let bad = args(&artifact, &data(&[3.0, f64::NAN]), &memory);
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&good, &mut output, limits(), &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|v| v.as_ref().unwrap().clone())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        assert!(reference_result(&artifact, &bad).is_err());
        assert!(matches!(
            jit.evaluate_into(&bad, &mut output, limits(), &memory),
            Err(Error::Library {
                error: LeafError::Kernel(ResidentKernelError::Arithmetic),
                ..
            })
        ));
        equal(&artifact, &output, &prior);
        assert!(
            output
                .iter()
                .zip(&prior)
                .all(|(v, prior)| v.as_ref().unwrap().shares_frozen_storage(prior))
        );
        assert_eq!(memory.used_bytes(), before);
        for dimension in [
            Dimension::OutputElements,
            Dimension::OutputBytes,
            Dimension::TemporaryBytes,
            Dimension::ClonedBytes,
            Dimension::RetainedNodes,
            Dimension::ComputeWork,
            Dimension::ComparisonWork,
        ] {
            let mut denied = limits();
            match dimension {
                Dimension::OutputElements => denied.library.max_output_elements = Some(0),
                Dimension::OutputBytes => denied.library.max_output_bytes = Some(0),
                Dimension::TemporaryBytes => denied.library.max_temporary_bytes = Some(0),
                Dimension::ClonedBytes => denied.library.max_cloned_bytes = Some(0),
                Dimension::RetainedNodes => denied.library.max_retained_nodes = Some(0),
                Dimension::ComputeWork => denied.library.max_compute_work = Some(0),
                Dimension::ComparisonWork => denied.library.max_comparison_work = Some(0),
                _ => unreachable!(),
            }
            let last = output
                .iter()
                .map(|v| v.as_ref().unwrap().clone())
                .collect::<Vec<_>>();
            let before = memory.used_bytes();
            let error = jit
                .evaluate_into(&good, &mut output, denied, &memory)
                .unwrap_err();
            let actual = match &error {
                Error::Library { error: LeafError::Kernel(ResidentKernelError::ResourceLimit { dimension, .. }), .. } => *dimension,
                Error::Library { error: LeafError::Plan(MemoryPlanError::TargetLimitExceeded { violation }), .. } => violation.dimension,
                Error::Library { error: LeafError::Binding(mech_engine::native_artifact::NativeArtifactLoweringError::ResidentBinding {
                    error: mech_engine::resident::ResidentActivationError::ResidentMemoryPlanRejected {
                        error: MemoryPlanError::TargetLimitExceeded { violation },
                    },
                }), .. } => violation.dimension,
                _ => panic!("expected typed {dimension:?} denial, got {error:?}"),
            };
            assert_eq!(actual, dimension);
            equal(&artifact, &output, &prior);
            assert!(
                output
                    .iter()
                    .zip(&last)
                    .all(|(v, last)| v.as_ref().unwrap().shares_frozen_storage(last))
            );
            assert_eq!(memory.used_bytes(), before);
            jit.evaluate_into(&good, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &prior);
        }
        drop((good, bad, output, prior));
        assert_eq!(memory.used_bytes(), 0);
    }
}
