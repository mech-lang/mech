//! A live shape and packed draft must both fit before projection can allocate.
use super::super::{artifacts, limits};
use mech_core::{ManagedMemoryBudget, SchemaBody as B, ValueDataDraft as Data};
use mech_engine::native_artifact::{
    NativeCollectionInvocationError as E, NativeIterationAction as A, NativeLibraryWorkBudget,
    NativeStructuralObservation as O, lower_native_program_artifact,
    native_project_structural_value,
};
use std::sync::Arc;

#[test]
fn dynamic_projection_admits_shape_metadata_and_expanded_drafts_atomically() {
    let source = "(xs, n) := (data<[u8]>, 7u64)\nxs\n";
    for artifact in artifacts(source).0 {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let (site, input) = plan
            .blocks()
            .iter()
            .enumerate()
            .find_map(|(site, block)| match block.action() {
                A::Structural {
                    source,
                    observation: O::CanonicalValue,
                    output,
                    ..
                } if matches!(
                    plan.values()[output.get() as usize].definition().body(),
                    B::Matrix { .. }
                ) =>
                {
                    Some((site, *source))
                }
                _ => None,
            })
            .unwrap();
        let schemas = Arc::new(artifact.schemas().clone());
        let schema_id = plan.values()[input.get() as usize].schema();
        let schema = schemas.get(schema_id).unwrap();
        let closed = B::Tuple(
            vec![
                B::Matrix {
                    element: Box::new(B::UnsignedInteger(mech_core::IntegerWidth::W8)),
                    dimensions: vec![
                        mech_core::DimensionExpr::Constant(1),
                        mech_core::DimensionExpr::Constant(4096),
                    ]
                    .into(),
                },
                B::UnsignedInteger(mech_core::IntegerWidth::W64),
            ]
            .into(),
        );
        let shape =
            mech_core::shape_for_schema_components(schema, &[(schema.body(), closed)], None)
                .unwrap();
        let value = mech_core::ValueDraft {
            schema: schema_id,
            shape_values: shape.parameter_values().to_vec().into(),
            data: Data::Tuple(
                vec![Data::Matrix(vec![Data::U8(31); 4096].into()), Data::U64(7)].into(),
            ),
        }
        .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
            &schemas,
        ))
        .unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut reservation = memory
            .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
            .unwrap();
        let value = value.into_memory_budget(&mut reservation).unwrap();
        drop(reservation);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000_000, &memory).unwrap();
        let before = memory.used_bytes();
        let mut policy = limits().library;
        policy.max_temporary_bytes = Some(0);
        let required = match native_project_structural_value(
            &plan, site, &schemas, &value, None, policy, &work,
        )
        .unwrap_err()
        {
            E::Plan(mech_core::MemoryPlanError::TargetLimitExceeded { violation }) => {
                assert_eq!(
                    violation.dimension,
                    mech_core::MemoryBudgetDimension::TemporaryBytes
                );
                violation.required
            }
            error => panic!("{error:?}"),
        };
        assert!(required > 4096);
        assert_eq!(memory.used_bytes(), before);
        // Funding shape closure alone must still fail before draft expansion.
        policy.max_temporary_bytes = Some(required);
        let required = match native_project_structural_value(
            &plan, site, &schemas, &value, None, policy, &work,
        )
        .unwrap_err()
        {
            E::Plan(mech_core::MemoryPlanError::TargetLimitExceeded { violation }) => {
                assert_eq!(
                    violation.dimension,
                    mech_core::MemoryBudgetDimension::TemporaryBytes
                );
                assert!(violation.required > required);
                violation.required
            }
            error => panic!("{error:?}"),
        };
        assert_eq!(memory.used_bytes(), before);
        policy.max_temporary_bytes = Some(required - 1);
        assert!(
            native_project_structural_value(&plan, site, &schemas, &value, None, policy, &work)
                .is_err()
        );
        assert_eq!(memory.used_bytes(), before);
        policy.max_temporary_bytes = Some(required);
        policy.max_cloned_bytes = Some(0);
        assert!(
            matches!(native_project_structural_value(&plan, site, &schemas, &value, None, policy, &work),
            Err(E::Plan(mech_core::MemoryPlanError::TargetLimitExceeded { violation }))
                if violation.dimension == mech_core::MemoryBudgetDimension::ClonedBytes)
        );
        assert_eq!(memory.used_bytes(), before);
        policy.max_cloned_bytes = limits().library.max_cloned_bytes;
        let result =
            native_project_structural_value(&plan, site, &schemas, &value, None, policy, &work)
                .unwrap();
        assert_eq!(
            result.canonical_data_draft().unwrap(),
            Data::Matrix(vec![Data::U8(31); 4096].into())
        );
        assert_eq!(result.shape().parameter_values(), &[1, 4096]);
        drop((result, value, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn dynamic_tuple_projections_check_factory_at_lowering_and_roundtrip_build_images() {
    use mech_engine::native_artifact::NativeArtifactLoweringError;
    use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramDescriptor};
    let catalog = mech_stdlib::source_catalog();
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory(
            ["access"],
            "scalar",
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
            |_| panic!("a same-name replacement cannot become native projection authority"),
        )
        .unwrap();
    for (key, entry) in catalog.resident_factories() {
        if key.module_path.as_ref() != ["access"] || key.operation_name != "scalar" {
            builder
                .insert_resident_factory_entry(entry.clone())
                .unwrap();
        }
    }
    let fake = builder.build().unwrap();
    for artifact in artifacts("(xs, n) := (data<[u16]>, 7u64)\nxs\n").0 {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let bytes = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: this unchanged build-owned image was produced locally above.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }.unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), bytes);
        assert!(matches!(
            lower_native_program_artifact(&artifact, &fake),
            Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
        ));
        // Projection is native code in the checked build, so loading it does
        // not select access/scalar again. The source-lowering check above is
        // the factory authority; the unsafe image loader requires build pins.
    }
}
