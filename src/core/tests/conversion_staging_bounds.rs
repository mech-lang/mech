use mech_core::snapshot::{OptionDraft, SnapshotValidationContext};
use mech_core::*;

fn matrix(element: SchemaBody) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(4)].into_boxed_slice(),
    }
}
fn set(element: SchemaBody) -> SchemaBody {
    SchemaBody::Set {
        element: Box::new(element),
        cardinality: CardinalitySpec::Dynamic { upper_bound: None },
    }
}
fn schema(body: SchemaBody) -> Schema {
    SchemaDraft {
        body,
        dimension_parameters: Box::new([]),
    }
    .finalize()
    .unwrap()
}

#[test]
fn metadata_nodes_follow_real_trees_not_payload_name_bytes() {
    let enum_schema = |name: String| {
        schema(SchemaBody::Enum {
            key: NominalKey::from_bytes([83; 32]),
            variants: vec![EnumVariantSchema {
                name,
                payload: None,
            }]
            .into_boxed_slice(),
        })
    };
    let short = enum_schema("x".into());
    let long = enum_schema("x".repeat(4096));
    let short = conversion_schema_staging_bound(&short, &short).unwrap();
    let long = conversion_schema_staging_bound(&long, &long).unwrap();
    assert_eq!(short.metadata_nodes, long.metadata_nodes);
    assert_eq!(short.plan_nodes, long.plan_nodes);
    assert!(long.temporary_bytes > short.temporary_bytes);
    assert!(long.setup_work > short.setup_work);
    assert!(short.plan_bytes >= short.temporary_bytes);
    assert!(short.plan_work >= short.setup_work);
    assert!(short.setup_clone_bytes >= short.temporary_bytes);
    assert!(conversion_staging_demand(short, u64::MAX, 1, 1).is_none());
    assert!(conversion_staging_demand(short, 1, 1, u64::MAX).is_none());
}

#[test]
fn shared_population_counts_packed_atom_lanes_and_stops_before_descent() {
    let atom = SchemaBody::Atom(NominalKey::from_bytes([84; 32]));
    let source = schema(SchemaBody::Option(Box::new(matrix(atom.clone()))));
    let target = schema(SchemaBody::Option(Box::new(set(atom))));
    let selected = plan_explicit_cast(
        &ResolvedType::from_schema(&source, &source.instantiate_shape(Box::new([])).unwrap())
            .unwrap(),
        &ResolvedType::from_schema(&target, &target.instantiate_shape(Box::new([])).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(conversion_materialization_phases(&selected.step), 1);
    assert_eq!(conversion_option_present_depth(&selected.step), 0);
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(source).unwrap();
    let build = builder.finish().unwrap();
    let id = build.resolve(handle).unwrap();
    let table = build.into_parts().0;
    for present in [false, true] {
        let value = ValueDraft {
            schema: id,
            shape_values: Box::new([]),
            data: ValueDataDraft::Option(OptionDraft {
                present,
                value: present.then(|| {
                    Box::new(ValueDataDraft::Matrix(
                        vec![ValueDataDraft::Atom; 4].into_boxed_slice(),
                    ))
                }),
            }),
        }
        .finalize(&SnapshotValidationContext::new(&table))
        .unwrap();
        let mut work = 0;
        let nodes = conversion_draft_population(value.data(), |amount| {
            work += amount;
            Ok::<(), ()>(())
        })
        .unwrap();
        assert_eq!((nodes, work), if present { (6, 192) } else { (1, 64) });
        let mut callbacks = 0;
        let denied = conversion_draft_population(value.data(), |_| {
            callbacks += 1;
            Err("denied")
        });
        assert_eq!(denied, Err(ConversionPopulationError::Observer("denied")));
        assert_eq!(callbacks, 1);
    }
}
