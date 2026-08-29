use mech_core::kind::Kind;
use mech_core::legacy_value::ValueKind;
use mech_core::{
    DimensionEnvironmentBuilder, DimensionExpr, DimensionLifetime, DimensionParameterId,
    DimensionParameterOrigin, EnumVariantSchema, KindExpr, KindId, KindNameCategory,
    LegacyExtentRole, LegacyExtentSite, LegacyNominalResolution, LegacyResolvedExtent,
    LegacySemanticContext, LegacyTypeSource, LegacyValueKindTag, NominalKey, NominalKind,
    SchemaBody, SemanticModelError, kind_expr_from_legacy, schema_from_legacy_value_kind,
};

#[derive(Default)]
struct FakeContext {
    extent_sites: Vec<LegacyExtentSite>,
}

impl LegacySemanticContext for FakeContext {
    fn resolve_named_kind(&mut self, legacy_id: u64) -> Result<KindId, SemanticModelError> {
        u32::try_from(legacy_id)
            .map(KindId::new)
            .map_err(|_| SemanticModelError::LegacyNamedKindUnresolved { legacy_id })
    }

    fn resolve_nominal(
        &mut self,
        nominal_kind: NominalKind,
        legacy_id: u64,
        _legacy_name: &str,
    ) -> Result<LegacyNominalResolution, SemanticModelError> {
        let key = NominalKey::from_bytes([legacy_id as u8; 32]);
        Ok(match nominal_kind {
            NominalKind::Atom => LegacyNominalResolution::Atom { key },
            NominalKind::Enum => LegacyNominalResolution::Enum {
                key,
                variants: vec![
                    EnumVariantSchema {
                        name: "First".to_owned(),
                        payload: None,
                    },
                    EnumVariantSchema {
                        name: "Second".to_owned(),
                        payload: Some(SchemaBody::Bool),
                    },
                ]
                .into_boxed_slice(),
            },
        })
    }

    fn resolve_unspecified_extent(
        &mut self,
        site: &LegacyExtentSite,
        dimensions: &mut DimensionEnvironmentBuilder,
    ) -> Result<LegacyResolvedExtent, SemanticModelError> {
        self.extent_sites.push(site.clone());
        let id = dimensions.declare(
            DimensionParameterOrigin::Inferred,
            DimensionLifetime::Turn,
            DimensionExpr::Constant(0),
            Some(DimensionExpr::Constant(1024)),
        )?;
        Ok(match site.role {
            LegacyExtentRole::MatrixDimensions => LegacyResolvedExtent::Dimensions(
                vec![DimensionExpr::Parameter(id)].into_boxed_slice(),
            ),
            LegacyExtentRole::TableRows
            | LegacyExtentRole::SetCardinality
            | LegacyExtentRole::MapCardinality => {
                LegacyResolvedExtent::Cardinality(DimensionExpr::Parameter(id))
            }
        })
    }
}

#[test]
fn all_seventeen_kind_variants_have_explicit_outcomes() {
    let kinds = vec![
        Kind::Any,
        Kind::None,
        Kind::Empty,
        Kind::Scalar(4),
        Kind::Id,
        Kind::Index,
        Kind::Atom(1, "Atom".to_owned()),
        Kind::Enum(2, "Enum".to_owned()),
        Kind::Matrix(Box::new(Kind::Id), vec![2, 3]),
        Kind::Option(Box::new(Kind::Id)),
        Kind::Tuple(vec![Kind::Id]),
        Kind::Record(vec![("x".to_owned(), Kind::Id)]),
        Kind::Table(vec![("x".to_owned(), Kind::Id)], 2),
        Kind::Set(Box::new(Kind::Id), Some(2)),
        Kind::Map(Box::new(Kind::Id), Box::new(Kind::Index)),
        Kind::Reference(Box::new(Kind::Id)),
        Kind::Kind(Box::new(Kind::Id)),
    ];
    assert_eq!(kinds.len(), 17);
    let mut context = FakeContext::default();
    let outcomes = kinds
        .iter()
        .map(|kind| kind_expr_from_legacy(kind, &mut context).unwrap().kind)
        .collect::<Vec<_>>();
    assert!(matches!(outcomes[0], KindExpr::Wildcard));
    assert!(matches!(outcomes[1], KindExpr::Never));
    assert!(matches!(outcomes[2], KindExpr::Hole));
    assert!(matches!(outcomes[3], KindExpr::Named(_)));
    assert!(matches!(outcomes[6], KindExpr::Atom(_)));
    assert!(matches!(outcomes[7], KindExpr::Enum(_)));
    assert!(matches!(outcomes[8], KindExpr::Matrix { .. }));
    assert!(matches!(outcomes[14], KindExpr::Map { .. }));
    assert!(matches!(outcomes[15], KindExpr::Reference(_)));
    assert!(matches!(outcomes[16], KindExpr::TypeOf(_)));
}

#[test]
fn all_thirty_two_value_kind_variants_map_or_error_explicitly() {
    let kinds = vec![
        ValueKind::U8,
        ValueKind::U16,
        ValueKind::U32,
        ValueKind::U64,
        ValueKind::U128,
        ValueKind::I8,
        ValueKind::I16,
        ValueKind::I32,
        ValueKind::I64,
        ValueKind::I128,
        ValueKind::F32,
        ValueKind::F64,
        ValueKind::C64,
        ValueKind::R64,
        ValueKind::String,
        ValueKind::Bool,
        ValueKind::Id,
        ValueKind::Index,
        ValueKind::Empty,
        ValueKind::Any,
        ValueKind::None,
        ValueKind::Matrix(Box::new(ValueKind::Bool), vec![2]),
        ValueKind::Enum(2, "Enum".to_owned()),
        ValueKind::Record(vec![("x".to_owned(), ValueKind::Bool)]),
        ValueKind::Map(Box::new(ValueKind::Bool), Box::new(ValueKind::String)),
        ValueKind::Atom(1, "Atom".to_owned()),
        ValueKind::Table(vec![("x".to_owned(), ValueKind::Bool)], 2),
        ValueKind::Tuple(vec![ValueKind::Bool]),
        ValueKind::Reference(Box::new(ValueKind::Bool)),
        ValueKind::Set(Box::new(ValueKind::Bool), Some(2)),
        ValueKind::Option(Box::new(ValueKind::Bool)),
        ValueKind::Kind(Box::new(ValueKind::Bool)),
    ];
    assert_eq!(kinds.len(), 32);
    let mut context = FakeContext::default();
    for (index, kind) in kinds.iter().enumerate() {
        let result = schema_from_legacy_value_kind(kind, &mut context);
        if matches!(index, 18 | 20 | 28) {
            assert!(matches!(
                result,
                Err(SemanticModelError::NonInstantiableLegacyValueKind { .. })
            ));
        } else {
            result.unwrap();
        }
    }
}

#[test]
fn unspecified_extents_are_context_controlled_and_allocate_dense_parameters() {
    let kind = Kind::Tuple(vec![
        Kind::Matrix(Box::new(Kind::Id), Vec::new()),
        Kind::Set(Box::new(Kind::Id), None),
        Kind::Table(Vec::new(), 0),
        Kind::Map(Box::new(Kind::Id), Box::new(Kind::Index)),
    ]);
    let mut context = FakeContext::default();
    let resolution = kind_expr_from_legacy(&kind, &mut context).unwrap();
    assert_eq!(resolution.dimension_parameters.len(), 4);
    assert_eq!(
        context
            .extent_sites
            .iter()
            .map(|site| site.role)
            .collect::<Vec<_>>(),
        [
            LegacyExtentRole::MatrixDimensions,
            LegacyExtentRole::SetCardinality,
            LegacyExtentRole::TableRows,
            LegacyExtentRole::MapCardinality,
        ]
    );
    assert!(
        context
            .extent_sites
            .iter()
            .all(|site| site.source == LegacyTypeSource::Kind)
    );
    assert_eq!(
        resolution
            .dimension_parameters
            .iter()
            .map(|parameter| parameter.id.get())
            .collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
}

#[test]
fn concrete_matrix_extents_are_constants_while_map_always_uses_context() {
    let mut context = FakeContext::default();
    let matrix =
        kind_expr_from_legacy(&Kind::Matrix(Box::new(Kind::Id), vec![2, 3]), &mut context).unwrap();
    let KindExpr::Matrix { dimensions, .. } = matrix.kind else {
        panic!("expected matrix")
    };
    assert_eq!(
        dimensions.as_ref(),
        &[DimensionExpr::Constant(2), DimensionExpr::Constant(3)]
    );
    assert!(context.extent_sites.is_empty());

    kind_expr_from_legacy(
        &Kind::Map(Box::new(Kind::Id), Box::new(Kind::Index)),
        &mut context,
    )
    .unwrap();
    assert_eq!(context.extent_sites.len(), 1);
    assert_eq!(
        context.extent_sites[0].role,
        LegacyExtentRole::MapCardinality
    );
}

#[test]
fn enum_variants_come_from_context_and_ambiguous_value_kinds_are_structured_errors() {
    let mut context = FakeContext::default();
    let schema =
        schema_from_legacy_value_kind(&ValueKind::Enum(9, "Choice".to_owned()), &mut context)
            .unwrap();
    let SchemaBody::Enum { variants, .. } = schema.body() else {
        panic!("expected enum schema")
    };
    assert_eq!(
        variants
            .iter()
            .map(|variant| variant.name.as_str())
            .collect::<Vec<_>>(),
        ["First", "Second"]
    );

    assert_eq!(
        schema_from_legacy_value_kind(&ValueKind::Any, &mut context)
            .unwrap()
            .body(),
        &SchemaBody::Dynamic,
    );

    for (kind, expected) in [
        (ValueKind::None, LegacyValueKindTag::None),
        (ValueKind::Empty, LegacyValueKindTag::Empty),
        (
            ValueKind::Reference(Box::new(ValueKind::Bool)),
            LegacyValueKindTag::Reference,
        ),
    ] {
        assert!(matches!(
            schema_from_legacy_value_kind(&kind, &mut context),
            Err(SemanticModelError::NonInstantiableLegacyValueKind { kind }) if kind == expected
        ));
    }
}

#[test]
fn legacy_type_of_is_the_reified_type_schema() {
    let mut context = FakeContext::default();
    let schema =
        schema_from_legacy_value_kind(&ValueKind::Kind(Box::new(ValueKind::Bool)), &mut context)
            .unwrap();
    assert_eq!(schema.body(), &SchemaBody::ReifiedType);
}

#[derive(Clone, Copy)]
enum ExtentContextMode {
    MissingDeclaration,
    ForwardReference,
    Cycle,
    Hole,
    Valid,
}

struct ExtentContext(ExtentContextMode);

impl LegacySemanticContext for ExtentContext {
    fn resolve_named_kind(&mut self, legacy_id: u64) -> Result<KindId, SemanticModelError> {
        Ok(KindId::new(legacy_id as u32))
    }

    fn resolve_nominal(
        &mut self,
        nominal_kind: NominalKind,
        legacy_id: u64,
        legacy_name: &str,
    ) -> Result<LegacyNominalResolution, SemanticModelError> {
        Err(SemanticModelError::LegacyNominalUnresolved {
            kind: nominal_kind,
            legacy_id,
            legacy_name: legacy_name.to_owned(),
        })
    }

    fn resolve_unspecified_extent(
        &mut self,
        _site: &LegacyExtentSite,
        dimensions: &mut DimensionEnvironmentBuilder,
    ) -> Result<LegacyResolvedExtent, SemanticModelError> {
        let dimension = match self.0 {
            ExtentContextMode::MissingDeclaration => {
                DimensionExpr::Parameter(DimensionParameterId::new(999))
            }
            ExtentContextMode::ForwardReference => {
                let first = dimensions.declare(
                    DimensionParameterOrigin::Inferred,
                    DimensionLifetime::Turn,
                    DimensionExpr::Parameter(DimensionParameterId::new(1)),
                    None,
                )?;
                dimensions.declare(
                    DimensionParameterOrigin::Inferred,
                    DimensionLifetime::Turn,
                    DimensionExpr::Constant(0),
                    None,
                )?;
                DimensionExpr::Parameter(first)
            }
            ExtentContextMode::Cycle => {
                let first = dimensions.declare(
                    DimensionParameterOrigin::Inferred,
                    DimensionLifetime::Turn,
                    DimensionExpr::Parameter(DimensionParameterId::new(1)),
                    None,
                )?;
                dimensions.declare(
                    DimensionParameterOrigin::Inferred,
                    DimensionLifetime::Turn,
                    DimensionExpr::Parameter(first),
                    None,
                )?;
                DimensionExpr::Parameter(first)
            }
            ExtentContextMode::Hole => DimensionExpr::Hole,
            ExtentContextMode::Valid => DimensionExpr::Parameter(dimensions.declare(
                DimensionParameterOrigin::Inferred,
                DimensionLifetime::Turn,
                DimensionExpr::Constant(0),
                Some(DimensionExpr::Constant(1024)),
            )?),
        };
        Ok(LegacyResolvedExtent::Dimensions(
            vec![dimension].into_boxed_slice(),
        ))
    }
}

#[test]
fn legacy_kind_adapter_validates_context_produced_dimensions() {
    let unresolved_matrix = Kind::Matrix(Box::new(Kind::Id), Vec::new());
    for (mode, expected) in [
        (
            ExtentContextMode::MissingDeclaration,
            SemanticModelError::UnknownDimensionParameterV1 {
                id: DimensionParameterId::new(999),
            },
        ),
        (
            ExtentContextMode::ForwardReference,
            SemanticModelError::ForwardDimensionParameterReferenceV1 {
                parameter: DimensionParameterId::new(0),
                referenced: DimensionParameterId::new(1),
            },
        ),
        (
            ExtentContextMode::Cycle,
            SemanticModelError::CyclicDimensionParameterBoundsV1,
        ),
        (
            ExtentContextMode::Hole,
            SemanticModelError::UnresolvedDimensionHole,
        ),
    ] {
        assert_eq!(
            kind_expr_from_legacy(&unresolved_matrix, &mut ExtentContext(mode)).unwrap_err(),
            expected,
        );
    }

    let valid = kind_expr_from_legacy(
        &unresolved_matrix,
        &mut ExtentContext(ExtentContextMode::Valid),
    )
    .unwrap();
    assert_eq!(valid.dimension_parameters.len(), 1);
    assert!(matches!(
        valid.kind,
        KindExpr::Matrix { ref dimensions, .. }
            if dimensions.as_ref()
                == [DimensionExpr::Parameter(DimensionParameterId::new(0))]
    ));
}

#[test]
fn legacy_kind_adapter_rejects_duplicate_names_recursively() {
    let duplicate = Kind::Option(Box::new(Kind::Table(
        vec![("x".to_owned(), Kind::Id), ("x".to_owned(), Kind::Index)],
        1,
    )));
    assert!(matches!(
        kind_expr_from_legacy(&duplicate, &mut FakeContext::default()),
        Err(SemanticModelError::DuplicateKindName {
            category: KindNameCategory::TableColumn,
            ..
        })
    ));
}
