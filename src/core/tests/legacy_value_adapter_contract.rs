#![cfg(all(
    feature = "bool",
    feature = "f64",
    feature = "matrix",
    feature = "matrixd",
    feature = "tuple"
))]

use mech_core::legacy_value::{LegacyValue, ValueKind};
use mech_core::snapshot::{SnapshotValidationContext, ValueData};
use mech_core::{
    DimensionEnvironmentBuilder, DimensionExpr, EnumVariantSchema, KindId, LegacyEmptyPolicy,
    LegacyExtentSite, LegacyMaterializationContext, LegacyNominalResolution, LegacyReferencePolicy,
    LegacyResolvedExtent, LegacySemanticContext, LegacySnapshotContext, LegacySnapshotError,
    NominalKey, NominalKind, Ref, SchemaBody, SchemaDraft, SchemaId, SchemaTable,
    SchemaTableBuilder, SemanticModelError, ToMatrix, legacy_from_snapshot, snapshot_from_legacy,
};

#[derive(Default)]
struct SemanticContext;

impl LegacySemanticContext for SemanticContext {
    fn resolve_named_kind(&mut self, legacy_id: u64) -> Result<KindId, SemanticModelError> {
        u32::try_from(legacy_id)
            .map(KindId::new)
            .map_err(|_| SemanticModelError::LegacyNamedKindUnresolved { legacy_id })
    }

    fn resolve_nominal(
        &mut self,
        kind: NominalKind,
        legacy_id: u64,
        _legacy_name: &str,
    ) -> Result<LegacyNominalResolution, SemanticModelError> {
        let key = NominalKey::from_bytes([legacy_id as u8; 32]);
        Ok(match kind {
            NominalKind::Atom => LegacyNominalResolution::Atom { key },
            NominalKind::Enum => LegacyNominalResolution::Enum {
                key,
                variants: vec![EnumVariantSchema {
                    name: "Only".to_owned(),
                    payload: Some(SchemaBody::Bool),
                }]
                .into_boxed_slice(),
            },
        })
    }

    fn resolve_unspecified_extent(
        &mut self,
        _site: &LegacyExtentSite,
        _dimensions: &mut DimensionEnvironmentBuilder,
    ) -> Result<LegacyResolvedExtent, SemanticModelError> {
        Err(SemanticModelError::LegacyExtentResolutionKindMismatch)
    }
}

impl LegacyMaterializationContext for SemanticContext {
    fn resolve_nominal(
        &mut self,
        _kind: NominalKind,
        key: NominalKey,
    ) -> Result<(u64, String), LegacySnapshotError> {
        Ok((u64::from(key.as_bytes()[0]), "Nominal".to_owned()))
    }
}

fn schema_table(body: SchemaBody) -> (SchemaTable, SchemaId) {
    let schema = SchemaDraft {
        dimension_parameters: Box::new([]),
        body,
    }
    .finalize()
    .unwrap();
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema).unwrap();
    let build = builder.finish().unwrap();
    let id = build.resolve(handle).unwrap();
    let (schemas, _) = build.into_parts();
    (schemas, id)
}

fn snapshot(
    legacy: &LegacyValue,
    schemas: &SchemaTable,
    schema: SchemaId,
    empty: LegacyEmptyPolicy,
    reference: LegacyReferencePolicy,
) -> Result<mech_core::Value, LegacySnapshotError> {
    let validation = SnapshotValidationContext::new(schemas);
    let mut semantic = SemanticContext;
    let mut context = LegacySnapshotContext::new(&mut semantic, empty, reference);
    snapshot_from_legacy(legacy, schema, Box::new([]), &validation, &mut context)
}

#[test]
fn typed_wrappers_disappear_and_option_absence_requires_policy() {
    let (bool_schemas, bool_schema) = schema_table(SchemaBody::Bool);
    let typed = LegacyValue::Typed(Box::new(LegacyValue::Bool(Ref::new(true))), ValueKind::Bool);
    let value = snapshot(
        &typed,
        &bool_schemas,
        bool_schema,
        LegacyEmptyPolicy::Reject,
        LegacyReferencePolicy::Reject,
    )
    .unwrap();
    assert!(matches!(value.data(), ValueData::Bool(true)));

    let (option_schemas, option_schema) =
        schema_table(SchemaBody::Option(Box::new(SchemaBody::Bool)));
    assert!(matches!(
        snapshot(
            &LegacyValue::Empty,
            &option_schemas,
            option_schema,
            LegacyEmptyPolicy::Reject,
            LegacyReferencePolicy::Reject,
        ),
        Err(LegacySnapshotError::LegacyEmptyNotSnapshot)
    ));
    let absent = snapshot(
        &LegacyValue::Empty,
        &option_schemas,
        option_schema,
        LegacyEmptyPolicy::ResolveOptionAbsence,
        LegacyReferencePolicy::Reject,
    )
    .unwrap();
    assert!(matches!(value.data(), ValueData::Bool(true)));
    assert!(matches!(absent.data(), ValueData::Option(None)));

    let present = snapshot(
        &LegacyValue::Bool(Ref::new(false)),
        &option_schemas,
        option_schema,
        LegacyEmptyPolicy::ResolveOptionAbsence,
        LegacyReferencePolicy::Reject,
    )
    .unwrap();
    assert!(matches!(present.data(), ValueData::Option(Some(_))));
}

#[test]
fn reference_policy_detects_active_cycles_but_allows_completed_aliases() {
    let (bool_schemas, bool_schema) = schema_table(SchemaBody::Bool);
    let cell = Ref::new(LegacyValue::Empty);
    *cell.borrow_mut() = LegacyValue::MutableReference(cell.clone());
    let cycle = LegacyValue::MutableReference(cell);
    assert!(matches!(
        snapshot(
            &cycle,
            &bool_schemas,
            bool_schema,
            LegacyEmptyPolicy::Reject,
            LegacyReferencePolicy::SnapshotCurrentValue,
        ),
        Err(LegacySnapshotError::LegacyReferenceCycle)
    ));
    assert!(matches!(
        snapshot(
            &LegacyValue::MutableReference(Ref::new(LegacyValue::Bool(Ref::new(true)))),
            &bool_schemas,
            bool_schema,
            LegacyEmptyPolicy::Reject,
            LegacyReferencePolicy::Reject,
        ),
        Err(LegacySnapshotError::LegacyReferenceNotPermitted)
    ));

    let shared = Ref::new(LegacyValue::Bool(Ref::new(true)));
    let tuple = LegacyValue::Tuple(Ref::new(mech_core::MechTuple::from_vec(vec![
        LegacyValue::MutableReference(shared.clone()),
        LegacyValue::MutableReference(shared),
    ])));
    let (tuple_schemas, tuple_schema) = schema_table(SchemaBody::Tuple(
        vec![SchemaBody::Bool, SchemaBody::Bool].into_boxed_slice(),
    ));
    let value = snapshot(
        &tuple,
        &tuple_schemas,
        tuple_schema,
        LegacyEmptyPolicy::Reject,
        LegacyReferencePolicy::SnapshotCurrentValue,
    )
    .unwrap();
    assert!(matches!(value.data(), ValueData::Tuple(values) if values.len() == 2));
}

#[test]
fn matrix_value_is_schema_directed_homogeneous_and_logical_ordered() {
    let body = SchemaBody::Matrix {
        element: Box::new(SchemaBody::Bool),
        dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(2)].into_boxed_slice(),
    };
    let (schemas, schema) = schema_table(body);
    let matrix = <LegacyValue as ToMatrix>::to_matrixd(
        vec![
            LegacyValue::Bool(Ref::new(true)),
            LegacyValue::Bool(Ref::new(true)),
            LegacyValue::Bool(Ref::new(false)),
            LegacyValue::Bool(Ref::new(false)),
        ],
        2,
        2,
    );
    let value = snapshot(
        &LegacyValue::MatrixValue(matrix),
        &schemas,
        schema,
        LegacyEmptyPolicy::Reject,
        LegacyReferencePolicy::Reject,
    )
    .unwrap();
    assert_eq!(
        value.canonical_payload_bytes(&schemas).unwrap().as_ref(),
        &[1, 0, 1, 0]
    );

    let heterogeneous = <LegacyValue as ToMatrix>::to_matrixd(
        vec![
            LegacyValue::Bool(Ref::new(true)),
            LegacyValue::F64(Ref::new(1.0)),
        ],
        2,
        1,
    );
    let (schemas, schema) = schema_table(SchemaBody::Matrix {
        element: Box::new(SchemaBody::Bool),
        dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(1)].into_boxed_slice(),
    });
    assert!(matches!(
        snapshot(
            &LegacyValue::MatrixValue(heterogeneous),
            &schemas,
            schema,
            LegacyEmptyPolicy::Reject,
            LegacyReferencePolicy::Reject,
        ),
        Err(LegacySnapshotError::HeterogeneousMatrixUnsupported)
    ));
}

#[test]
fn reverse_materialization_allocates_fresh_refs_and_rejects_non_rank_two_matrices() {
    let (schemas, schema) = schema_table(SchemaBody::Tuple(
        vec![SchemaBody::Bool, SchemaBody::Bool].into_boxed_slice(),
    ));
    let legacy = LegacyValue::Tuple(Ref::new(mech_core::MechTuple::from_vec(vec![
        LegacyValue::Bool(Ref::new(true)),
        LegacyValue::Bool(Ref::new(true)),
    ])));
    let value = snapshot(
        &legacy,
        &schemas,
        schema,
        LegacyEmptyPolicy::Reject,
        LegacyReferencePolicy::Reject,
    )
    .unwrap();
    let mut materialization = SemanticContext;
    let LegacyValue::Tuple(tuple) =
        legacy_from_snapshot(&value, &schemas, &mut materialization).unwrap()
    else {
        panic!("expected tuple")
    };
    let tuple = tuple.borrow();
    let (LegacyValue::Bool(first), LegacyValue::Bool(second)) =
        (&*tuple.elements[0], &*tuple.elements[1])
    else {
        panic!("expected bools")
    };
    assert!(!first.same_handle(second));

    let (schemas, schema) = schema_table(SchemaBody::Matrix {
        element: Box::new(SchemaBody::Bool),
        dimensions: vec![
            DimensionExpr::Constant(1),
            DimensionExpr::Constant(1),
            DimensionExpr::Constant(1),
        ]
        .into_boxed_slice(),
    });
    let validation = SnapshotValidationContext::new(&schemas);
    let value = mech_core::ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: mech_core::ValueDataDraft::Matrix(
            vec![mech_core::ValueDataDraft::Bool(true)].into_boxed_slice(),
        ),
    }
    .finalize(&validation)
    .unwrap();
    assert!(matches!(
        legacy_from_snapshot(&value, &schemas, &mut materialization),
        Err(LegacySnapshotError::UnsupportedLegacyMaterialization)
    ));
}

#[test]
fn typed_empty_and_selection_rules_are_exact() {
    let (option_schemas, option_schema) =
        schema_table(SchemaBody::Option(Box::new(SchemaBody::Bool)));
    let absent = snapshot(
        &LegacyValue::EmptyKind(ValueKind::Option(Box::new(ValueKind::Bool))),
        &option_schemas,
        option_schema,
        LegacyEmptyPolicy::Reject,
        LegacyReferencePolicy::Reject,
    )
    .unwrap();
    assert!(matches!(absent.data(), ValueData::Option(None)));

    let (matrix_schemas, matrix_schema) = schema_table(SchemaBody::Matrix {
        element: Box::new(SchemaBody::Bool),
        dimensions: vec![DimensionExpr::Constant(0), DimensionExpr::Constant(2)].into_boxed_slice(),
    });
    let empty = snapshot(
        &LegacyValue::EmptyKind(ValueKind::Matrix(Box::new(ValueKind::Bool), vec![0, 2])),
        &matrix_schemas,
        matrix_schema,
        LegacyEmptyPolicy::Reject,
        LegacyReferencePolicy::Reject,
    )
    .unwrap();
    assert!(
        empty
            .canonical_payload_bytes(&matrix_schemas)
            .unwrap()
            .is_empty()
    );

    let (bool_schemas, bool_schema) = schema_table(SchemaBody::Bool);
    assert!(matches!(
        snapshot(
            &LegacyValue::EmptyKind(ValueKind::Bool),
            &bool_schemas,
            bool_schema,
            LegacyEmptyPolicy::Reject,
            LegacyReferencePolicy::Reject,
        ),
        Err(LegacySnapshotError::InvalidTypedEmptySchema)
    ));
    assert!(matches!(
        snapshot(
            &LegacyValue::IndexAll,
            &bool_schemas,
            bool_schema,
            LegacyEmptyPolicy::Reject,
            LegacyReferencePolicy::Reject,
        ),
        Err(LegacySnapshotError::LegacySelectionValueRequiresC3)
    ));
}
