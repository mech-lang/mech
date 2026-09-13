use super::*;
use mech_syntax::document::parser::{canonical::parse_canonical_phase_2i_rule_for_test, rules};
use mech_syntax::document::{ParseConfig, RuleId, TextSnapshot};

fn parse<T: AstNode>(source: &str, rule: RuleId) -> T {
    fn find<T: AstNode>(node: SyntaxNode) -> Option<T> {
        T::cast(node.clone()).or_else(|| node.children().find_map(find::<T>))
    }
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x54b), Revision(1), source).unwrap(),
        rule,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean(), "{source}");
    assert_eq!(parsed.consumed.end.0 as usize, source.len(), "{source}");
    find(parsed.syntax()).unwrap()
}

fn selected(definition: &str, expression: &str) -> CanonicalSourceProgram {
    let definition: VariableDefineSyntax = parse(definition, rules::VARIABLE_DEFINE);
    let expression: ExpressionSyntax = parse(expression, rules::EXPRESSION);
    let mut builder = SemanticBuilder::new(SourceSemanticAnchor::for_node(expression.syntax()));
    builder
        .declare_definition_input_annotations(&definition, &BTreeSet::new())
        .unwrap();
    builder.definition(&definition).unwrap();
    builder
        .declare_input_annotations(expression.syntax(), &BTreeSet::new())
        .unwrap();
    let value = builder.expression(&expression).unwrap().0;
    builder.publish("result", None, value, expression.syntax());
    builder.finish().unwrap()
}

#[cfg(feature = "resident-artifact")]
#[test]
fn literal_masks_execute_true_cardinality_in_linear_and_rectangle_selection() {
    use crate::resident::{ActivationFacts, activate};
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ValueData};
    let mut catalog = FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (definition, expression, expected) in [
        ("a := [1u8 2u8]", "a[[true false]]", vec![1.0]),
        ("a := [1 2]", "a[[true false]]", vec![1.0]),
        ("a := [1 2]", "a[[false false]]", vec![]),
        ("a := [1 2; 3 4]", "a[[false true],:]", vec![3.0, 4.0]),
        ("a := [1 2; 3 4]", "a[[false false],:]", vec![]),
        ("a := [1 2; 3 4]", "a[:,[false false]]", vec![]),
        ("a := [1 2; 3 4]", "a[[false false],[true false]]", vec![]),
        ("a := [1 2; 3 4]", "a[:,[true false]]", vec![1.0, 3.0]),
        ("a := [1 2; 3 4]", "a[[false true],[true false]]", vec![3.0]),
        (
            "a := [1u8 2u8; 3u8 4u8]",
            "a[[false true],[true false]]",
            vec![3.0],
        ),
    ] {
        let compiled = selected(definition, expression);
        let artifact = compiled.compile_artifact().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x54b, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{definition}; {expression}: {error:?}"));
        instance
            .turn(&[])
            .unwrap_or_else(|error| panic!("{expression}: {error:?}"));
        let value = instance.copied_output(0).unwrap();
        let ValueData::Matrix(matrix) = value.data() else {
            panic!("{expression}: {:?}", value.data())
        };
        let actual = match matrix.elements() {
            mech_core::snapshot::SequenceView::F64(values) => values
                .iter()
                .map(|value| value.to_f64())
                .collect::<Vec<_>>(),
            mech_core::snapshot::SequenceView::U8(values) => values
                .iter()
                .map(|value| f64::from(*value))
                .collect::<Vec<_>>(),
            other => panic!("unexpected matrix representation {other:?}"),
        };
        assert_eq!(actual, expected, "{expression}");
    }
}

#[test]
fn logical_selection_schema_tracks_population_instead_of_mask_length() {
    let compiled = selected("a := [1 2]", "a[[true false]]");
    let schema = compiled
        .schemas()
        .get(compiled.program().outputs[0].schema)
        .unwrap();
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = schema.body()
    else {
        panic!()
    };
    assert_eq!(
        element.as_ref(),
        &SchemaBody::FloatingPoint(FloatWidth::W64)
    );
    assert!(matches!(dimensions[0], DimensionExpr::Parameter(_)));
    assert_eq!(dimensions[1], DimensionExpr::Constant(1));
    assert_eq!(schema.dimension_parameters().len(), 1);
    for count in 0..=2 {
        schema.instantiate_shape(Box::new([count])).unwrap();
    }
    assert!(schema.instantiate_shape(Box::new([3])).is_err());
}

#[cfg(feature = "resident-artifact")]
#[test]
fn live_masks_revalidate_population_and_preserve_outputs_after_rejection() {
    use crate::resident::{ActivationFacts, CapturedSignalInput, activate};
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    let mut catalog = FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for definition in ["a := [1 2]", "a := [1u8 2u8]"] {
        let compiled = selected(definition, "a[mask<[bool]:1,2>]");
        let artifact = compiled.compile_artifact().unwrap();
        assert!(
            activate(
                ReactiveInstanceId::new(0x54c, 0),
                &artifact,
                &catalog,
                &ActivationFacts::default()
            )
            .is_err()
        );
        let mut facts = ActivationFacts::default();
        for slot in artifact.slots() {
            let schema = artifact.schemas().get(slot.schema).unwrap();
            if !schema.dimension_parameters().is_empty() {
                assert_eq!(schema.dimension_parameters().len(), 1);
                facts
                    .slot_shapes
                    .insert(slot.slot, schema.instantiate_shape(Box::new([1])).unwrap());
            }
        }
        let mut instance = activate(
            ReactiveInstanceId::new(0x54c, 0),
            &artifact,
            &catalog,
            &facts,
        )
        .unwrap();
        for (mask, expected) in [([1u8, 0], 1.0), ([0, 1], 2.0)] {
            let inputs = [CapturedSignalInput {
                slot: instance.plan.inputs[0].slot,
                value: ResidentValueRef::Bool(&mask),
            }];
            instance.turn(&inputs).unwrap();
            let value = instance.copied_output(0).unwrap();
            let ValueData::Matrix(matrix) = value.data() else {
                panic!()
            };
            match matrix.elements() {
                mech_core::snapshot::SequenceView::F64(values) => {
                    assert_eq!(values[0].to_f64(), expected)
                }
                mech_core::snapshot::SequenceView::U8(values) => {
                    assert_eq!(f64::from(values[0]), expected)
                }
                _ => panic!(),
            }
        }
        let previous = instance.copied_output(0).unwrap();
        for mask in [[1u8, 1], [0, 0], [2, 0]] {
            let inputs = [CapturedSignalInput {
                slot: instance.plan.inputs[0].slot,
                value: ResidentValueRef::Bool(&mask),
            }];
            assert!(instance.turn(&inputs).is_err(), "{definition}: {mask:?}");
            assert!(
                previous
                    .language_eq(
                        artifact.schemas(),
                        &instance.copied_output(0).unwrap(),
                        artifact.schemas()
                    )
                    .unwrap()
            );
        }
        let mask = [1u8, 0];
        let inputs = [CapturedSignalInput {
            slot: instance.plan.inputs[0].slot,
            value: ResidentValueRef::Bool(&mask),
        }];
        instance.turn(&inputs).unwrap();
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn rectangle_masks_validate_each_axis_even_when_total_cardinality_is_unchanged() {
    use crate::resident::{ActivationFacts, CapturedSignalInput, activate};
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef};
    let compiled = selected("a := [1 2; 3 4]", "a[rows<[bool]:1,2>,columns<[bool]:1,2>]");
    let artifact = compiled.compile_artifact().unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut facts = ActivationFacts::default();
    for slot in artifact.slots() {
        let schema = artifact.schemas().get(slot.schema).unwrap();
        if !schema.dimension_parameters().is_empty() {
            assert_eq!(schema.dimension_parameters().len(), 2);
            facts.slot_shapes.insert(
                slot.slot,
                schema.instantiate_shape(Box::new([1, 2])).unwrap(),
            );
        }
    }
    let mut instance = activate(
        ReactiveInstanceId::new(0x54d, 0),
        &artifact,
        &catalog,
        &facts,
    )
    .unwrap();
    let captured = |instance: &crate::resident::ReactiveInstance,
                    rows: &'static [u8],
                    columns: &'static [u8]| {
        instance
            .plan
            .inputs
            .iter()
            .map(|input| CapturedSignalInput {
                slot: input.slot,
                value: ResidentValueRef::Bool(
                    if artifact
                        .inputs()
                        .iter()
                        .find(|declaration| declaration.slot == input.artifact_slot)
                        .unwrap()
                        .name
                        == crate::encode_source_input_name("rows")
                    {
                        rows
                    } else {
                        columns
                    },
                ),
            })
            .collect::<Vec<_>>()
    };
    instance
        .turn(&captured(&instance, &[1, 0], &[1, 1]))
        .unwrap();
    let previous = instance.copied_output(0).unwrap();
    assert!(
        instance
            .turn(&captured(&instance, &[1, 1], &[1, 0]))
            .is_err()
    );
    assert!(
        previous
            .language_eq(
                artifact.schemas(),
                &instance.copied_output(0).unwrap(),
                artifact.schemas()
            )
            .unwrap()
    );
}

#[cfg(feature = "resident-artifact")]
#[test]
fn masks_must_cover_the_selected_source_axis() {
    use crate::resident::{ActivationFacts, activate};
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId};
    let mut catalog = FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for expression in [
        "a[[true]]",
        "a[[true true]]",
        "a[[true false true]]",
        "a[[true],:]",
        "a[:,[true]]",
        "a[[true],[true false]]",
    ] {
        let compiled = selected("a := [1 2; 3 4]", expression);
        let artifact = compiled.compile_artifact().unwrap();
        assert!(
            activate(
                ReactiveInstanceId::new(0x54e, 0),
                &artifact,
                &catalog,
                &ActivationFacts::default()
            )
            .is_err(),
            "{expression}"
        );
    }
}
