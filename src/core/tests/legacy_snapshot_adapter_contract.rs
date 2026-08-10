#![cfg(all(
    feature = "bool",
    feature = "f64",
    feature = "matrix",
    feature = "matrixd",
    feature = "tuple"
))]

use mech_core::legacy_value::{LegacyValue, ValueKind};
use mech_core::snapshot::{
    ReifiedTypeDraft, SnapshotValidationContext, ValueData, ValueDataDraft, ValueDraft,
};
use mech_core::{
    CanonicalNominalPath, DimensionEnvironmentBuilder, DimensionExpr, DimensionLifetime,
    DimensionParameterDeclaration, DimensionParameterId, DimensionParameterOrigin,
    EnumVariantSchema, KindExpr, KindId, LegacyEmptyPolicy, LegacyExtentRole, LegacyExtentSite,
    LegacyMaterializationContext, LegacyNominalResolution, LegacyReferencePolicy,
    LegacyResolvedExtent, LegacySemanticContext, LegacySnapshotContext, LegacySnapshotError,
    NamedKindPathResolver, NominalKey, NominalKind, Ref, SchemaBody, SchemaDraft, SchemaField,
    SchemaId, SchemaTable, SchemaTableBuilder, SemanticModelError, ToMatrix, legacy_from_snapshot,
    snapshot_from_legacy,
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
    schema_table_with_dimensions(body, Box::new([]))
}

fn schema_table_with_dimensions(
    body: SchemaBody,
    dimension_parameters: Box<[DimensionParameterDeclaration]>,
) -> (SchemaTable, SchemaId) {
    let schema = SchemaDraft {
        dimension_parameters,
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

fn round_trip(legacy: &LegacyValue, body: SchemaBody) -> (LegacyValue, Box<[u8]>) {
    let (schemas, schema) = schema_table(body);
    let original = snapshot(
        legacy,
        &schemas,
        schema,
        LegacyEmptyPolicy::Reject,
        LegacyReferencePolicy::Reject,
    )
    .unwrap();
    let mut materialization = SemanticContext;
    let materialized = legacy_from_snapshot(&original, &schemas, &mut materialization).unwrap();
    let reconstructed = snapshot(
        &materialized,
        &schemas,
        schema,
        LegacyEmptyPolicy::Reject,
        LegacyReferencePolicy::Reject,
    )
    .unwrap();
    assert!(
        original
            .snapshot_eq(&schemas, &reconstructed, &schemas)
            .unwrap()
    );
    let payload = reconstructed.canonical_payload_bytes(&schemas).unwrap();
    (materialized, payload)
}

struct ReifiedNamedKinds {
    bool_id: KindId,
    string_id: KindId,
    bool_path: CanonicalNominalPath,
    string_path: CanonicalNominalPath,
}

impl ReifiedNamedKinds {
    fn new() -> Self {
        Self {
            bool_id: KindId::new(mech_core::hash_str(&ValueKind::Bool.to_string()) as u32),
            string_id: KindId::new(mech_core::hash_str(&ValueKind::String.to_string()) as u32),
            bool_path: CanonicalNominalPath::new(vec!["legacy".to_owned(), "Bool".to_owned()])
                .unwrap(),
            string_path: CanonicalNominalPath::new(vec!["legacy".to_owned(), "String".to_owned()])
                .unwrap(),
        }
    }
}

impl NamedKindPathResolver for ReifiedNamedKinds {
    fn canonical_path(&self, id: KindId) -> Option<&CanonicalNominalPath> {
        if id == self.bool_id {
            Some(&self.bool_path)
        } else if id == self.string_id {
            Some(&self.string_path)
        } else {
            None
        }
    }
}

struct ReifiedSemanticContext;

impl LegacySemanticContext for ReifiedSemanticContext {
    fn resolve_named_kind(&mut self, legacy_id: u64) -> Result<KindId, SemanticModelError> {
        Ok(KindId::new(legacy_id as u32))
    }

    fn resolve_nominal(
        &mut self,
        kind: NominalKind,
        legacy_id: u64,
        legacy_name: &str,
    ) -> Result<LegacyNominalResolution, SemanticModelError> {
        Err(SemanticModelError::LegacyNominalUnresolved {
            kind,
            legacy_id,
            legacy_name: legacy_name.to_owned(),
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

struct ExtentSemanticContext {
    lifetime: DimensionLifetime,
    lower_bound: u64,
    upper_bound: u64,
}

impl LegacySemanticContext for ExtentSemanticContext {
    fn resolve_named_kind(&mut self, legacy_id: u64) -> Result<KindId, SemanticModelError> {
        Ok(KindId::new(legacy_id as u32))
    }

    fn resolve_nominal(
        &mut self,
        kind: NominalKind,
        legacy_id: u64,
        legacy_name: &str,
    ) -> Result<LegacyNominalResolution, SemanticModelError> {
        Err(SemanticModelError::LegacyNominalUnresolved {
            kind,
            legacy_id,
            legacy_name: legacy_name.to_owned(),
        })
    }

    fn resolve_unspecified_extent(
        &mut self,
        site: &LegacyExtentSite,
        dimensions: &mut DimensionEnvironmentBuilder,
    ) -> Result<LegacyResolvedExtent, SemanticModelError> {
        let id = dimensions.declare(
            DimensionParameterOrigin::Inferred,
            self.lifetime,
            DimensionExpr::Constant(self.lower_bound),
            Some(DimensionExpr::Constant(self.upper_bound)),
        )?;
        match site.role {
            LegacyExtentRole::MatrixDimensions => Ok(LegacyResolvedExtent::Dimensions(
                vec![DimensionExpr::Parameter(id)].into_boxed_slice(),
            )),
            _ => Err(SemanticModelError::LegacyExtentResolutionKindMismatch),
        }
    }
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
fn reverse_options_preserve_presence_at_every_nesting_level() {
    let bool_option = ValueKind::Option(Box::new(ValueKind::Bool));
    let nested_option = ValueKind::Option(Box::new(bool_option.clone()));
    let nested_body = SchemaBody::Option(Box::new(SchemaBody::Option(Box::new(SchemaBody::Bool))));

    let (none, none_payload) = round_trip(
        &LegacyValue::EmptyKind(nested_option.clone()),
        nested_body.clone(),
    );
    assert!(matches!(none, LegacyValue::EmptyKind(kind) if kind == nested_option));

    let (some_none, some_none_payload) = round_trip(
        &LegacyValue::Typed(
            Box::new(LegacyValue::EmptyKind(bool_option.clone())),
            nested_option.clone(),
        ),
        nested_body.clone(),
    );
    assert!(matches!(
        some_none,
        LegacyValue::Typed(inner, kind)
            if kind == nested_option
                && matches!(inner.as_ref(), LegacyValue::EmptyKind(inner_kind) if inner_kind == &bool_option)
    ));

    let (some_some, some_some_payload) = round_trip(
        &LegacyValue::Typed(
            Box::new(LegacyValue::Typed(
                Box::new(LegacyValue::Bool(Ref::new(false))),
                bool_option.clone(),
            )),
            nested_option.clone(),
        ),
        nested_body,
    );
    assert!(matches!(
        some_some,
        LegacyValue::Typed(outer, outer_kind)
            if outer_kind == nested_option
                && matches!(outer.as_ref(), LegacyValue::Typed(inner, inner_kind)
                    if inner_kind == &bool_option
                        && matches!(inner.as_ref(), LegacyValue::Bool(_)))
    ));

    assert_ne!(none_payload, some_none_payload);
    assert_ne!(none_payload, some_some_payload);
    assert_ne!(some_none_payload, some_some_payload);

    let (absent, _) = round_trip(
        &LegacyValue::EmptyKind(ValueKind::Option(Box::new(ValueKind::Bool))),
        SchemaBody::Option(Box::new(SchemaBody::Bool)),
    );
    assert!(matches!(
        absent,
        LegacyValue::EmptyKind(ValueKind::Option(_))
    ));
    let (present, _) = round_trip(
        &LegacyValue::Typed(
            Box::new(LegacyValue::Bool(Ref::new(false))),
            ValueKind::Option(Box::new(ValueKind::Bool)),
        ),
        SchemaBody::Option(Box::new(SchemaBody::Bool)),
    );
    assert!(matches!(
        present,
        LegacyValue::Typed(_, ValueKind::Option(_))
    ));
}

#[test]
fn options_round_trip_inside_tuples_and_enum_payloads() {
    let option_kind = ValueKind::Option(Box::new(ValueKind::Bool));
    let tuple = LegacyValue::Tuple(Ref::new(mech_core::MechTuple::from_vec(vec![
        LegacyValue::Typed(
            Box::new(LegacyValue::Bool(Ref::new(false))),
            option_kind.clone(),
        ),
    ])));
    let (tuple, _) = round_trip(
        &tuple,
        SchemaBody::Tuple(vec![SchemaBody::Option(Box::new(SchemaBody::Bool))].into_boxed_slice()),
    );
    let LegacyValue::Tuple(tuple) = tuple else {
        panic!("expected tuple")
    };
    assert!(matches!(
        &*tuple.borrow().elements[0],
        LegacyValue::Typed(_, ValueKind::Option(_))
    ));

    #[cfg(feature = "enum")]
    {
        let enum_id = 7;
        let variant_id = mech_core::hash_str("Only");
        let names = Ref::new(mech_core::Dictionary::new());
        names.borrow_mut().insert(enum_id, "Choice".to_owned());
        names.borrow_mut().insert(variant_id, "Only".to_owned());
        let legacy = LegacyValue::Enum(Ref::new(mech_core::MechEnum {
            id: enum_id,
            variants: vec![(
                variant_id,
                Some(LegacyValue::Typed(
                    Box::new(LegacyValue::Bool(Ref::new(false))),
                    option_kind,
                )),
            )],
            names,
        }));
        let (materialized, _) = round_trip(
            &legacy,
            SchemaBody::Enum {
                key: NominalKey::from_bytes([enum_id as u8; 32]),
                variants: vec![EnumVariantSchema {
                    name: "Only".to_owned(),
                    payload: Some(SchemaBody::Option(Box::new(SchemaBody::Bool))),
                }]
                .into_boxed_slice(),
            },
        );
        let LegacyValue::Enum(value) = materialized else {
            panic!("expected enum")
        };
        assert!(matches!(
            value.borrow().variants[0].1,
            Some(LegacyValue::Typed(_, ValueKind::Option(_)))
        ));
    }
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
fn legacy_borrow_conflicts_are_fallible_at_scalar_reference_and_aggregate_boundaries() {
    let (bool_schemas, bool_schema) = schema_table(SchemaBody::Bool);
    let scalar = Ref::new(true);
    let scalar_guard = scalar.borrow_mut();
    assert!(matches!(
        snapshot(
            &LegacyValue::Bool(scalar.clone()),
            &bool_schemas,
            bool_schema,
            LegacyEmptyPolicy::Reject,
            LegacyReferencePolicy::Reject,
        ),
        Err(LegacySnapshotError::LegacyBorrowConflict)
    ));
    drop(scalar_guard);

    let reference = Ref::new(LegacyValue::Bool(Ref::new(true)));
    let reference_guard = reference.borrow_mut();
    assert!(matches!(
        snapshot(
            &LegacyValue::MutableReference(reference.clone()),
            &bool_schemas,
            bool_schema,
            LegacyEmptyPolicy::Reject,
            LegacyReferencePolicy::SnapshotCurrentValue,
        ),
        Err(LegacySnapshotError::LegacyBorrowConflict)
    ));
    drop(reference_guard);

    let (tuple_schemas, tuple_schema) =
        schema_table(SchemaBody::Tuple(vec![SchemaBody::Bool].into_boxed_slice()));
    let tuple = Ref::new(mech_core::MechTuple::from_vec(vec![LegacyValue::Bool(
        Ref::new(true),
    )]));
    let tuple_guard = tuple.borrow_mut();
    assert!(matches!(
        snapshot(
            &LegacyValue::Tuple(tuple.clone()),
            &tuple_schemas,
            tuple_schema,
            LegacyEmptyPolicy::Reject,
            LegacyReferencePolicy::Reject,
        ),
        Err(LegacySnapshotError::LegacyBorrowConflict)
    ));
    drop(tuple_guard);

    #[cfg(feature = "record")]
    {
        let field_id = mech_core::hash_str("x");
        let record = Ref::new(mech_core::MechRecord::from_parts(
            1,
            vec![ValueKind::Bool],
            vec![(field_id, "x".to_owned(), LegacyValue::Bool(Ref::new(true)))],
        ));
        let (record_schemas, record_schema) = schema_table(SchemaBody::Record(
            vec![SchemaField {
                name: "x".to_owned(),
                schema: SchemaBody::Bool,
            }]
            .into_boxed_slice(),
        ));
        let record_guard = record.borrow_mut();
        assert!(matches!(
            snapshot(
                &LegacyValue::Record(record.clone()),
                &record_schemas,
                record_schema,
                LegacyEmptyPolicy::Reject,
                LegacyReferencePolicy::Reject,
            ),
            Err(LegacySnapshotError::LegacyBorrowConflict)
        ));
        drop(record_guard);
    }
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
fn composite_matrix_mismatches_use_the_bounded_heterogeneous_error() {
    let tuple_matrix = <LegacyValue as ToMatrix>::to_matrixd(
        vec![LegacyValue::Tuple(Ref::new(
            mech_core::MechTuple::from_vec(vec![LegacyValue::F64(Ref::new(1.0))]),
        ))],
        1,
        1,
    );
    let (schemas, schema) = schema_table(SchemaBody::Matrix {
        element: Box::new(SchemaBody::Tuple(vec![SchemaBody::Bool].into_boxed_slice())),
        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(1)].into_boxed_slice(),
    });
    assert!(matches!(
        snapshot(
            &LegacyValue::MatrixValue(tuple_matrix),
            &schemas,
            schema,
            LegacyEmptyPolicy::Reject,
            LegacyReferencePolicy::Reject,
        ),
        Err(LegacySnapshotError::HeterogeneousMatrixUnsupported)
    ));

    #[cfg(feature = "record")]
    {
        let field_id = mech_core::hash_str("x");
        let record_matrix = <LegacyValue as ToMatrix>::to_matrixd(
            vec![LegacyValue::Record(Ref::new(
                mech_core::MechRecord::from_parts(
                    1,
                    vec![ValueKind::F64],
                    vec![(field_id, "x".to_owned(), LegacyValue::F64(Ref::new(1.0)))],
                ),
            ))],
            1,
            1,
        );
        let (schemas, schema) = schema_table(SchemaBody::Matrix {
            element: Box::new(SchemaBody::Record(
                vec![SchemaField {
                    name: "x".to_owned(),
                    schema: SchemaBody::Bool,
                }]
                .into_boxed_slice(),
            )),
            dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(1)]
                .into_boxed_slice(),
        });
        assert!(matches!(
            snapshot(
                &LegacyValue::MatrixValue(record_matrix),
                &schemas,
                schema,
                LegacyEmptyPolicy::Reject,
                LegacyReferencePolicy::Reject,
            ),
            Err(LegacySnapshotError::HeterogeneousMatrixUnsupported)
        ));
    }
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
fn legacy_type_of_kinds_preserve_their_inner_kind() {
    let (schemas, schema) = schema_table(SchemaBody::ReifiedType);
    let validation = SnapshotValidationContext::new(&schemas);
    let mut semantic = SemanticContext;
    let mut context = LegacySnapshotContext::new(
        &mut semantic,
        LegacyEmptyPolicy::Reject,
        LegacyReferencePolicy::Reject,
    );
    let id = snapshot_from_legacy(
        &LegacyValue::Kind(ValueKind::Kind(Box::new(ValueKind::Id))),
        schema,
        Box::new([]),
        &validation,
        &mut context,
    )
    .unwrap();
    let mut semantic = SemanticContext;
    let mut context = LegacySnapshotContext::new(
        &mut semantic,
        LegacyEmptyPolicy::Reject,
        LegacyReferencePolicy::Reject,
    );
    let index = snapshot_from_legacy(
        &LegacyValue::Kind(ValueKind::Kind(Box::new(ValueKind::Index))),
        schema,
        Box::new([]),
        &validation,
        &mut context,
    )
    .unwrap();
    assert_ne!(
        id.canonical_payload_bytes(&schemas).unwrap(),
        index.canonical_payload_bytes(&schemas).unwrap()
    );
    assert_ne!(
        id.value_hash(&schemas).unwrap(),
        index.value_hash(&schemas).unwrap()
    );

    let named = ReifiedNamedKinds::new();
    let validation = SnapshotValidationContext::with_named_kinds(&schemas, &named);
    let reify_named = |kind| {
        let mut semantic = ReifiedSemanticContext;
        let mut context = LegacySnapshotContext::new(
            &mut semantic,
            LegacyEmptyPolicy::Reject,
            LegacyReferencePolicy::Reject,
        );
        snapshot_from_legacy(
            &LegacyValue::Kind(ValueKind::Kind(Box::new(kind))),
            schema,
            Box::new([]),
            &validation,
            &mut context,
        )
        .unwrap()
    };
    let bool_kind = reify_named(ValueKind::Bool);
    let string_kind = reify_named(ValueKind::String);
    assert_ne!(
        bool_kind.canonical_payload_bytes(&schemas).unwrap(),
        string_kind.canonical_payload_bytes(&schemas).unwrap()
    );
    assert_ne!(
        bool_kind.value_hash(&schemas).unwrap(),
        string_kind.value_hash(&schemas).unwrap()
    );
}

#[test]
fn reified_closed_kinds_are_intentionally_not_reverse_decoded() {
    let (schemas, schema) = schema_table(SchemaBody::ReifiedType);
    let validation = SnapshotValidationContext::new(&schemas);
    for kind in [
        KindExpr::Wildcard,
        KindExpr::Reference(Box::new(KindExpr::Id)),
    ] {
        let value = ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Type(ReifiedTypeDraft::Kind {
                kind,
                dimension_parameters: Box::new([]),
            }),
        }
        .finalize(&validation)
        .unwrap();
        let mut materialization = SemanticContext;
        assert!(matches!(
            legacy_from_snapshot(&value, &schemas, &mut materialization),
            Err(LegacySnapshotError::UnsupportedLegacyMaterialization)
        ));
    }
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

#[test]
fn nested_typed_schemas_compare_complete_projected_schema_keys() {
    let parameter = DimensionParameterDeclaration {
        id: DimensionParameterId::new(0),
        origin: DimensionParameterOrigin::Explicit,
        lifetime: DimensionLifetime::Activation,
        lower_bound: DimensionExpr::Constant(1),
        upper_bound: Some(DimensionExpr::Constant(3)),
    };
    let matrix = SchemaBody::Matrix {
        element: Box::new(SchemaBody::Bool),
        dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0))].into_boxed_slice(),
    };
    let (schemas, schema) = schema_table_with_dimensions(
        SchemaBody::Tuple(vec![matrix].into_boxed_slice()),
        vec![parameter].into_boxed_slice(),
    );
    let legacy = LegacyValue::Tuple(Ref::new(mech_core::MechTuple::from_vec(vec![
        LegacyValue::Typed(
            Box::new(LegacyValue::Empty),
            ValueKind::Matrix(Box::new(ValueKind::Bool), Vec::new()),
        ),
    ])));

    for mut semantic in [
        ExtentSemanticContext {
            lifetime: DimensionLifetime::Turn,
            lower_bound: 1,
            upper_bound: 3,
        },
        ExtentSemanticContext {
            lifetime: DimensionLifetime::Activation,
            lower_bound: 0,
            upper_bound: 3,
        },
        ExtentSemanticContext {
            lifetime: DimensionLifetime::Activation,
            lower_bound: 1,
            upper_bound: 4,
        },
    ] {
        let validation = SnapshotValidationContext::new(&schemas);
        let mut context = LegacySnapshotContext::new(
            &mut semantic,
            LegacyEmptyPolicy::Reject,
            LegacyReferencePolicy::Reject,
        );
        assert!(matches!(
            snapshot_from_legacy(
                &legacy,
                schema,
                vec![2].into_boxed_slice(),
                &validation,
                &mut context,
            ),
            Err(LegacySnapshotError::LegacyTypedSchemaMismatch)
        ));
    }
}

#[cfg(feature = "rational")]
#[test]
fn rational_range_failures_have_a_specific_adapter_error() {
    let (schemas, schema) = schema_table(SchemaBody::Rational64);
    let legacy = LegacyValue::R64(Ref::new(mech_core::R64(num_rational::Rational64::new_raw(
        1, -1,
    ))));
    assert!(matches!(
        snapshot(
            &legacy,
            &schemas,
            schema,
            LegacyEmptyPolicy::Reject,
            LegacyReferencePolicy::Reject,
        ),
        Err(LegacySnapshotError::LegacyRationalOutOfRange)
    ));

    let value = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::Rational64 {
            numerator: 1,
            denominator: u64::MAX,
        },
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap();
    let mut materialization = SemanticContext;
    assert!(matches!(
        legacy_from_snapshot(&value, &schemas, &mut materialization),
        Err(LegacySnapshotError::LegacyRationalOutOfRange)
    ));
}
