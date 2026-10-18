//! The same maintained leaves consume/produce canonical values without a
//! dense String bridge. These are library tests, not native program execution.
use super::{map_ops, pure_text, text_ops};
use mech_core::{
    BoundResidentKernel, DeclaredOperationContract, DimensionExpr, FunctionCatalogBuilder,
    IntegerWidth, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentPortLayout, ResidentShape, ResidentValueKind, ResidentValueMut, ResidentValueRef,
    ResolvedInputPort, ResolvedOperationContract, ResolvedOutputPort, SchemaBody, SchemaDraft,
    SchemaId, SchemaTable, SchemaTableBuilder, Value, ValueData, ValueDataDraft, ValueDraft,
    snapshot::{SequenceView, SnapshotValidationContext},
};

struct Fixture {
    schemas: SchemaTable,
    text: SchemaId,
    integer: SchemaId,
    float: SchemaId,
    words: SchemaId,
    keys: SchemaId,
    values: SchemaId,
    counts: SchemaId,
}
impl Fixture {
    fn new() -> Self {
        let mut builder = SchemaTableBuilder::new();
        let mut add = |body| {
            builder
                .insert(
                    SchemaDraft {
                        dimension_parameters: Box::new([]),
                        body,
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap()
        };
        let text = add(SchemaBody::String);
        let integer = add(SchemaBody::UnsignedInteger(IntegerWidth::W64));
        let float = add(SchemaBody::FloatingPoint(mech_core::FloatWidth::W64));
        let row = |element, count| SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: Box::new([DimensionExpr::Constant(1), DimensionExpr::Constant(count)]),
        };
        let words = add(row(SchemaBody::String, 3));
        let keys = add(row(SchemaBody::String, 2));
        let values = add(row(SchemaBody::UnsignedInteger(IntegerWidth::W64), 2));
        let counts = add(SchemaBody::Map {
            key: Box::new(SchemaBody::String),
            value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            cardinality: DimensionExpr::Constant(2).into(),
        });
        let build = builder.finish().unwrap();
        let [text, integer, float, words, keys, values, counts] =
            [text, integer, float, words, keys, values, counts]
                .map(|id| build.resolve(id).unwrap());
        let (schemas, _) = build.into_parts();
        Self {
            schemas,
            text,
            integer,
            float,
            words,
            keys,
            values,
            counts,
        }
    }
    fn value(&self, schema: SchemaId, data: ValueDataDraft) -> Value {
        ValueDraft {
            schema,
            shape_values: Box::new([]),
            data,
        }
        .finalize(&SnapshotValidationContext::new(&self.schemas))
        .unwrap()
    }
    fn layout(&self, schema: SchemaId, kind: ResidentValueKind) -> ResidentPortLayout {
        let metadata = self.schemas.get(schema).unwrap();
        ResidentPortLayout {
            schema_id: schema,
            schema_key: metadata.key(),
            kind,
            shape: ResidentShape::SCALAR,
            shape_instance: metadata.instantiate_shape(Box::new([])).unwrap(),
            activation_fixed_shape: true,
            resolved_selector: None,
        }
    }
    fn request(
        &self,
        name: &str,
        inputs: &[SchemaId],
        output: SchemaId,
    ) -> (
        ResolvedOperationContract,
        Vec<ResidentPortLayout>,
        ResidentPortLayout,
    ) {
        let declaration = mech_core::maintained_operation_contract(
            name,
            inputs.len(),
            matches!(
                self.schemas.get(output).unwrap().body(),
                SchemaBody::Matrix { .. }
            ),
        )
        .unwrap();
        let policies = declaration.inputs.resolve(inputs.len()).unwrap();
        let policy = &declaration.outputs[0];
        let contract = ResolvedOperationContract::Declared(DeclaredOperationContract {
            inputs: inputs
                .iter()
                .zip(policies.iter())
                .map(|(&schema, policy)| ResolvedInputPort {
                    schema,
                    access: policy.access,
                    delivery: policy.delivery,
                })
                .collect(),
            outputs: Box::new([ResolvedOutputPort {
                schema: output,
                access: policy.access,
                delivery: policy.delivery,
                construction: policy.construction.clone(),
                alias: policy.alias,
                change_detection: policy.change_detection,
            }]),
            interaction: declaration.interaction,
        });
        (
            contract,
            inputs
                .iter()
                .map(|&id| self.layout(id, ResidentValueKind::Snapshot))
                .collect(),
            self.layout(output, ResidentValueKind::Snapshot),
        )
    }
    fn bind(&self, name: &str, inputs: &[SchemaId], output: SchemaId) -> BoundResidentKernel {
        let mut builder = FunctionCatalogBuilder::new();
        text_ops::install(&mut builder).unwrap();
        pure_text::install(&mut builder).unwrap();
        map_ops::install(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let (module, operation) = name.split_once('/').unwrap();
        let factory = catalog
            .resident_factory(&[module.to_owned()], operation)
            .unwrap();
        let (contract, inputs, output) = self.request(name, inputs, output);
        (factory.factory())(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &self.schemas,
            inputs: &inputs,
            output,
        })
        .unwrap()
    }
}
struct Inputs<'a>(&'a [ResidentValueRef<'a>]);
impl ResidentKernelInputs for Inputs<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
        self.0.get(index).copied()
    }
}
fn run(
    kernel: &BoundResidentKernel,
    inputs: &[&Value],
    output: &mut Option<Value>,
) -> Result<bool, ResidentKernelError> {
    let owned = inputs
        .iter()
        .map(|value| Some((*value).clone()))
        .collect::<Vec<_>>();
    let refs = owned
        .iter()
        .map(|value| ResidentValueRef::Snapshot(core::slice::from_ref(value)))
        .collect::<Vec<_>>();
    kernel.execute(
        &Inputs(&refs),
        ResidentValueMut::Snapshot(core::slice::from_mut(output)),
    )
}
fn string(value: &Option<Value>) -> &str {
    let ValueData::String(text) = value.as_ref().unwrap().data() else {
        panic!("expected String")
    };
    text
}

#[test]
fn all_seven_maintained_collection_leaves_compose_with_canonical_string_transport() {
    let f = Fixture::new();
    let input = f.value(f.text, ValueDataDraft::String("one\ntwo\none".into()));
    let separator = f.value(f.text, ValueDataDraft::String(" / ".into()));
    let precision = f.value(f.integer, ValueDataDraft::U64(0));
    let mut words = None;
    run(
        &f.bind("text/lines", &[f.text], f.words),
        &[&input],
        &mut words,
    )
    .unwrap();
    let mut counts = None;
    run(
        &f.bind("map/histogram", &[f.words], f.counts),
        &[words.as_ref().unwrap()],
        &mut counts,
    )
    .unwrap();
    let mut keys = None;
    run(
        &f.bind("map/keys", &[f.counts], f.keys),
        &[counts.as_ref().unwrap()],
        &mut keys,
    )
    .unwrap();
    let mut values = None;
    run(
        &f.bind("map/values", &[f.counts], f.values),
        &[counts.as_ref().unwrap()],
        &mut values,
    )
    .unwrap();
    let ValueData::Matrix(matrix) = values.as_ref().unwrap().data() else {
        panic!("expected counts")
    };
    let SequenceView::U64(counts) = matrix.elements() else {
        panic!("expected exact U64 counters")
    };
    assert_eq!(counts, &[2, 1]);
    let join = f.bind("text/join", &[f.keys, f.text], f.text);
    let mut joined = None;
    assert!(run(&join, &[keys.as_ref().unwrap(), &separator], &mut joined).unwrap());
    assert_eq!(string(&joined), "one / two");
    assert!(!run(&join, &[keys.as_ref().unwrap(), &separator], &mut joined).unwrap());
    let mut length = None;
    run(
        &f.bind("sequence/length", &[f.text], f.integer),
        &[joined.as_ref().unwrap()],
        &mut length,
    )
    .unwrap();
    assert!(matches!(length.as_ref().unwrap().data(), ValueData::U64(9)));
    let mut formatted = None;
    run(
        &f.bind("number/format", &[f.integer, f.integer], f.text),
        &[length.as_ref().unwrap(), &precision],
        &mut formatted,
    )
    .unwrap();
    assert_eq!(string(&formatted), "9");
    let maximum = f.value(f.integer, ValueDataDraft::U64(u64::MAX));
    run(
        &f.bind("number/format", &[f.integer, f.integer], f.text),
        &[&maximum, &precision],
        &mut formatted,
    )
    .unwrap();
    assert_eq!(string(&formatted), "18446744073709551615");
}

#[test]
fn canonical_string_inputs_reject_wrong_types_and_dense_substitution_before_publication() {
    let f = Fixture::new();
    let input = f.value(f.text, ValueDataDraft::String("λ\n🙂\nλ".into()));
    let wrong = f.value(f.integer, ValueDataDraft::U64(7));
    let lines = f.bind("text/lines", &[f.text], f.words);
    let mut output = None;
    run(&lines, &[&input], &mut output).unwrap();
    let before = output.as_ref().unwrap().clone();
    assert!(run(&lines, &[&wrong], &mut output).is_err());
    let dense = ["λ\n🙂\nλ".to_owned()];
    assert!(
        lines
            .execute(
                &Inputs(&[ResidentValueRef::String(&dense)]),
                ResidentValueMut::Snapshot(core::slice::from_mut(&mut output))
            )
            .is_err()
    );
    assert!(
        before
            .snapshot_eq(&f.schemas, output.as_ref().unwrap(), &f.schemas)
            .unwrap()
    );
    run(&lines, &[&input], &mut output).unwrap();
    let length = f.bind("sequence/length", &[f.text], f.integer);
    let unicode = f.value(f.text, ValueDataDraft::String("a\u{301}λ🙂".into()));
    let mut size = None;
    run(&length, &[&unicode], &mut size).unwrap();
    assert!(matches!(size.as_ref().unwrap().data(), ValueData::U64(3)));
}

#[test]
fn canonical_text_binders_reject_forged_schema_keys_and_join_preserves_output_on_error() {
    let f = Fixture::new();
    let (contract, mut inputs, output) = f.request("text/lines", &[f.text], f.words);
    inputs[0].schema_key = f.schemas.get(f.integer).unwrap().key();
    assert!(
        text_ops::bind_lines(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &f.schemas,
            inputs: &inputs,
            output
        })
        .is_err()
    );
    let keys = f.value(
        f.keys,
        ValueDataDraft::Matrix(
            vec![
                ValueDataDraft::String("λ".into()),
                ValueDataDraft::String("🙂".into()),
            ]
            .into_boxed_slice(),
        ),
    );
    let separator = f.value(f.text, ValueDataDraft::String("".into()));
    let wrong = f.value(f.integer, ValueDataDraft::U64(9));
    let join = f.bind("text/join", &[f.keys, f.text], f.text);
    let mut joined = None;
    run(&join, &[&keys, &separator], &mut joined).unwrap();
    assert_eq!(string(&joined), "λ🙂");
    assert!(run(&join, &[&keys, &wrong], &mut joined).is_err());
    assert_eq!(string(&joined), "λ🙂");
    assert!(!run(&join, &[&keys, &separator], &mut joined).unwrap());
}

#[test]
fn canonical_float_format_preserves_percentages_signed_zero_and_precision_failure_atomicity() {
    let f = Fixture::new();
    let formatter = f.bind("number/format", &[f.float, f.integer], f.text);
    let precision = f.value(f.integer, ValueDataDraft::U64(3));
    let invalid = f.value(f.integer, ValueDataDraft::U64(13));
    let mut output = None;
    for (number, expected) in [(100.0 / 3.0, "33.333"), (-0.0, "-0.000"), (1.0, "1.000")] {
        let value = f.value(
            f.float,
            ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(number)),
        );
        run(&formatter, &[&value, &precision], &mut output).unwrap();
        assert_eq!(string(&output), expected);
        assert!(run(&formatter, &[&value, &invalid], &mut output).is_err());
        assert_eq!(string(&output), expected);
        assert!(!run(&formatter, &[&value, &precision], &mut output).unwrap());
    }
}
