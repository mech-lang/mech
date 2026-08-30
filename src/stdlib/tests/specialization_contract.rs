#![cfg(all(
    feature = "full_compiler",
    feature = "matrix2",
    feature = "vector2",
    not(feature = "no_std")
))]

use mech_core::snapshot::F64Bits;
use mech_core::{
    ApplicationRequirement, BytecodeCompilerContext, CardinalitySpec, EncodedConstant, FloatWidth,
    FunctionCatalog, MResult, OperationId, Ref, Register, SchemaBody, SpecializationContext,
    SpecializationInvocation, ValueCell, ValueDataDraft, hash_str,
};
use nalgebra::{DMatrix, DVector, Matrix2, Vector2};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct ValueSpec {
    kind: String,
    shape: Vec<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct RuntimeFactory {
    name: String,
    id_hex: String,
}

#[derive(Debug, Deserialize)]
struct SpecializationCase {
    name: String,
    operation: String,
    operation_id_hex: String,
    arguments: Vec<ValueSpec>,
    output: ValueSpec,
    runtime_factories: Vec<RuntimeFactory>,
}

#[derive(Debug, Deserialize)]
struct SpecializationCases {
    schema: u32,
    cases: Vec<SpecializationCase>,
}

#[derive(Default)]
struct RuntimeLinkageRecorder {
    registers: BTreeMap<usize, Register>,
    next_register: Register,
    runtime_ids: Vec<u64>,
    unexpected_emissions: Vec<&'static str>,
}

impl RuntimeLinkageRecorder {
    fn record_runtime(&mut self, function: u64) {
        self.runtime_ids.push(function);
    }
}

impl BytecodeCompilerContext for RuntimeLinkageRecorder {
    fn register_for_ptr_with_initialization_status(&mut self, pointer: usize) -> (Register, bool) {
        if let Some(register) = self.registers.get(&pointer) {
            return (*register, false);
        }
        let register = self.next_register;
        self.next_register += 1;
        self.registers.insert(pointer, register);
        (register, false)
    }

    fn intern_constant(&mut self, _constant: EncodedConstant) -> MResult<u32> {
        self.unexpected_emissions.push("constant");
        Ok(0)
    }

    fn define_symbol(
        &mut self,
        _pointer: usize,
        _register: Register,
        _name: &str,
        _mutable: bool,
    ) -> MResult<()> {
        self.unexpected_emissions.push("symbol");
        Ok(())
    }

    fn intern_requirement(&mut self, _requirement: ApplicationRequirement) -> MResult<u32> {
        self.unexpected_emissions.push("application requirement");
        Ok(0)
    }

    fn emit_const_load(&mut self, _destination: Register, _constant: u32) {
        self.unexpected_emissions.push("constant load");
    }

    fn emit_composite_pack(
        &mut self,
        _destination: Register,
        _template: u32,
        _children: Vec<Register>,
    ) {
        self.unexpected_emissions.push("composite pack");
    }

    fn emit_nullop(&mut self, function: u64, _destination: Register) {
        self.record_runtime(function);
    }

    fn emit_unop(&mut self, function: u64, _destination: Register, _source: Register) {
        self.record_runtime(function);
    }

    fn emit_binop(
        &mut self,
        function: u64,
        _destination: Register,
        _lhs: Register,
        _rhs: Register,
    ) {
        self.record_runtime(function);
    }

    fn emit_ternop(
        &mut self,
        function: u64,
        _destination: Register,
        _a: Register,
        _b: Register,
        _c: Register,
    ) {
        self.record_runtime(function);
    }

    fn emit_quadop(
        &mut self,
        function: u64,
        _destination: Register,
        _a: Register,
        _b: Register,
        _c: Register,
        _d: Register,
    ) {
        self.record_runtime(function);
    }

    fn emit_varop(&mut self, function: u64, _destination: Register, _arguments: Vec<Register>) {
        self.record_runtime(function);
    }

    fn emit_host_call(
        &mut self,
        _requirement: u32,
        _destination: Register,
        _arguments: Vec<Register>,
    ) {
        self.unexpected_emissions.push("host call");
    }

    fn emit_resource_read(&mut self, _requirement: u32, _destination: Register) {
        self.unexpected_emissions.push("resource read");
    }

    fn emit_resource_write(
        &mut self,
        _requirement: u32,
        _destination: Register,
        _source: Register,
    ) {
        self.unexpected_emissions.push("resource write");
    }

    fn emit_resource_send(&mut self, _requirement: u32, _destination: Register, _source: Register) {
        self.unexpected_emissions.push("resource send");
    }
}

fn f64_cell(value: f64) -> ValueCell {
    ValueCell::from_exact(value).unwrap()
}

fn dynamic_matrix(values: &[f64], rows: usize, columns: usize) -> ValueCell {
    ValueCell::from_exact_matrix_ref(
        Ref::new(DMatrix::from_row_slice(rows, columns, values)),
        rows,
        columns,
    )
    .unwrap()
}

fn dynamic_vector(values: &[f64]) -> ValueCell {
    ValueCell::from_exact_matrix_ref(
        Ref::new(DVector::from_column_slice(values)),
        values.len(),
        1,
    )
    .unwrap()
}

fn f64_set(values: &[f64]) -> ValueCell {
    ValueCell::from_schema_data(
        SchemaBody::Set {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
        },
        ValueDataDraft::Set(
            values
                .iter()
                .copied()
                .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    )
    .unwrap()
}

fn inputs(name: &str) -> Vec<ValueCell> {
    match name {
        "combinatorics_n_choose_k_f64" => vec![f64_cell(5.0), f64_cell(2.0)],
        "compare_lt_f64" => vec![f64_cell(1.0), f64_cell(2.0)],
        "logic_and_bool" => vec![
            ValueCell::from_exact(true).unwrap(),
            ValueCell::from_exact(false).unwrap(),
        ],
        "math_add_assign_f64" => vec![f64_cell(10.0), f64_cell(20.0)],
        "math_add_f64_f64" => vec![f64_cell(2.0), f64_cell(3.0)],
        "math_add_f64_matrixd" => vec![f64_cell(2.0), dynamic_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2)],
        "math_add_matrixd_f64" => vec![dynamic_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2), f64_cell(2.0)],
        "math_add_matrixd_matrixd" => vec![
            dynamic_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2),
            dynamic_matrix(&[5.0, 6.0, 7.0, 8.0], 2, 2),
        ],
        "math_add_vector2_f64" => vec![
            ValueCell::from_exact(Vector2::new(1.0, 2.0)).unwrap(),
            f64_cell(3.0),
        ],
        "math_cos_f64" => vec![f64_cell(0.0)],
        "matrix_matmul_matrix2_f64" => vec![
            ValueCell::from_exact(Matrix2::new(1.0, 2.0, 3.0, 4.0)).unwrap(),
            ValueCell::from_exact(Matrix2::new(5.0, 6.0, 7.0, 8.0)).unwrap(),
        ],
        "matrix_solve_matrixd_vectord_f64" => vec![
            dynamic_matrix(&[4.0, 1.0, 2.0, 3.0], 2, 2),
            dynamic_vector(&[1.0, 2.0]),
        ],
        "range_inclusive_f64" => vec![f64_cell(1.0), f64_cell(3.0)],
        "range_inclusive_increment_f64" => {
            vec![f64_cell(1.0), f64_cell(2.0), f64_cell(5.0)]
        }
        "set_union_f64" => vec![f64_set(&[1.0, 2.0]), f64_set(&[2.0, 3.0])],
        "stats_sum_column_f64" => {
            vec![dynamic_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2)]
        }
        "string_concat" => vec![
            ValueCell::from_exact("ab".to_string()).unwrap(),
            ValueCell::from_exact("cd".to_string()).unwrap(),
        ],
        other => panic!("unimplemented specialization contract case {other}"),
    }
}

fn kind(body: &SchemaBody) -> String {
    match body {
        SchemaBody::Bool => "bool".into(),
        SchemaBody::FloatingPoint(FloatWidth::W64) => "f64".into(),
        SchemaBody::String => "string".into(),
        SchemaBody::Matrix { element, .. } => kind(element),
        SchemaBody::Set { element, .. } => format!("{{{}}}", kind(element)),
        other => panic!("unsupported specialization contract schema {other:?}"),
    }
}

fn value_spec(cell: &ValueCell) -> ValueSpec {
    let body = cell.closed_schema_body().unwrap();
    let shape = match &body {
        SchemaBody::Matrix { dimensions, .. } => {
            let shape = cell.shape();
            dimensions
                .iter()
                .map(|dimension| {
                    usize::try_from(shape.resolve_dimension(dimension).unwrap()).unwrap()
                })
                .collect()
        }
        _ => Vec::new(),
    };
    ValueSpec {
        kind: kind(&body),
        shape,
    }
}

fn format_id(id: u64) -> String {
    format!("{id:016x}")
}

fn runtime_factory(catalog: &FunctionCatalog, id: u64) -> RuntimeFactory {
    let entry = catalog
        .runtime_entry_by_raw(id)
        .unwrap_or_else(|| panic!("specialization references unknown runtime ID {id:?}"));
    RuntimeFactory {
        name: entry.name.clone(),
        id_hex: format_id(id),
    }
}

#[test]
fn canonical_specialization_preserves_the_frozen_operation_factory_and_storage_cases() {
    let expected: SpecializationCases = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/architecture/function-system/specialization-cases.json"
    )))
    .unwrap();
    assert_eq!(expected.schema, 2);
    assert_eq!(expected.cases.len(), 17);

    let catalog = mech_stdlib::source_catalog();
    for case in expected.cases {
        let operation = OperationId::from_name(&case.operation);
        assert_eq!(
            format_id(operation.raw()),
            case.operation_id_hex,
            "{}",
            case.name
        );
        assert_eq!(operation.raw(), hash_str(&case.operation), "{}", case.name);

        let arguments = inputs(&case.name);
        assert_eq!(
            arguments.iter().map(value_spec).collect::<Vec<_>>(),
            case.arguments,
            "{} input contract",
            case.name,
        );
        let invocation = SpecializationInvocation::from_cells(arguments.into_boxed_slice());
        let mut context = SpecializationContext::for_invocation(&invocation, Some(&catalog))
            .unwrap_or_else(|error| panic!("{} schema context: {error:?}", case.name));
        let specializer = catalog
            .specializer(operation)
            .unwrap_or_else(|| panic!("{} missing operation {}", case.name, case.operation));
        let specialized = specializer
            .specializer
            .specialize_invocation(&invocation, &mut context)
            .unwrap_or_else(|error| panic!("{} specialization: {error:?}", case.name));
        assert_eq!(
            value_spec(specialized.output()),
            case.output,
            "{} output",
            case.name
        );

        let mut recorder = RuntimeLinkageRecorder::default();
        specialized
            .instance()
            .implementation()
            .compile(&mut recorder)
            .unwrap_or_else(|error| panic!("{} linkage: {error:?}", case.name));
        assert!(
            recorder.unexpected_emissions.is_empty(),
            "{} emitted non-runtime operations: {:?}",
            case.name,
            recorder.unexpected_emissions,
        );
        assert_eq!(recorder.runtime_ids.len(), 1, "{} runtime count", case.name);
        assert_eq!(
            vec![runtime_factory(&catalog, recorder.runtime_ids[0])],
            case.runtime_factories,
            "{} runtime linkage",
            case.name,
        );
    }
}
