use crate::*;
#[cfg(feature = "matrix")]
use mech_core::nodes::Matrix as Mat;
#[cfg(feature = "enum")]
use mech_core::snapshot::EnumDraft;
#[cfg(feature = "map")]
use mech_core::snapshot::MapEntryDraft;
#[cfg(any(feature = "matrix", feature = "set"))]
use mech_core::snapshot::OptionDraft;

#[cfg(any(
    feature = "tuple",
    feature = "map",
    feature = "record",
    feature = "set",
    feature = "table",
    feature = "matrix"
))]
fn snapshot_draft(cell: &ValueCell) -> MResult<ValueDataDraft> {
    cell.snapshot()?.canonical_data_draft().map_err(|error| {
        MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
    })
}

#[cfg(any(
    feature = "tuple",
    feature = "map",
    feature = "record",
    feature = "table"
))]
fn required_cell(input: SpecializationInput, context: &'static str) -> MResult<ValueCell> {
    input.cell().cloned().map_err(|error| {
        MechError::new(
            CanonicalAggregateSourceAbsence { context },
            Some(format!("{error:?}")),
        )
        .with_compiler_loc()
    })
}

#[cfg(any(
    feature = "tuple",
    feature = "map",
    feature = "record",
    feature = "table"
))]
fn expression_value(
    expression_node: &Expression,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
    context: &'static str,
) -> MResult<ValueCell> {
    required_cell(expression(expression_node, env, p)?, context)
}

#[cfg(feature = "tuple")]
struct CanonicalTuplePack {
    output: ValueCell,
    elements: Box<[ValueCell]>,
}

#[cfg(feature = "tuple")]
impl CanonicalTuplePack {
    fn next_value(&self) -> MResult<mech_core::Value> {
        self.output.rebuild_tuple_cells(&self.elements)
    }
}

#[cfg(feature = "tuple")]
impl MechFunctionImpl for CanonicalTuplePack {
    fn solve_result(&self) -> MResult<()> {
        self.output.replace(&self.next_value()?)
    }

    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.clone()]
    }

    fn semantic_operation_name(&self) -> Option<&str> {
        Some("core/composite-pack")
    }

    fn to_string(&self) -> String {
        "CanonicalTuplePack".to_owned()
    }
}

#[cfg(all(feature = "tuple", feature = "semantic-compiler"))]
impl MechFunctionCompiler for CanonicalTuplePack {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        let mut cells = Vec::with_capacity(self.elements.len() + 1);
        cells.push(self.output.clone());
        cells.extend(self.elements.iter().cloned());
        cells
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        compile_value_cell_composite_register(&self.output, &self.elements, context)
    }
}

#[cfg(feature = "record")]
struct CanonicalRecordPack {
    output: ValueCell,
    fields: Box<[(String, ValueCell)]>,
}

#[cfg(feature = "record")]
impl CanonicalRecordPack {
    fn next_value(&self) -> MResult<mech_core::Value> {
        self.output.rebuild_record_cells(&self.fields)
    }
}

#[cfg(feature = "record")]
impl MechFunctionImpl for CanonicalRecordPack {
    fn solve_result(&self) -> MResult<()> {
        self.output.replace(&self.next_value()?)
    }

    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.clone()]
    }

    fn semantic_operation_name(&self) -> Option<&str> {
        Some("core/composite-pack")
    }

    fn to_string(&self) -> String {
        "CanonicalRecordPack".to_owned()
    }
}

#[cfg(all(feature = "record", feature = "semantic-compiler"))]
impl MechFunctionCompiler for CanonicalRecordPack {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        let mut cells = Vec::with_capacity(self.fields.len() + 1);
        cells.push(self.output.clone());
        cells.extend(self.fields.iter().map(|(_, cell)| cell.clone()));
        cells
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let fields = self
            .fields
            .iter()
            .map(|(_, cell)| cell.clone())
            .collect::<Vec<_>>();
        compile_value_cell_composite_register(&self.output, &fields, context)
    }
}

#[cfg(feature = "table")]
struct CanonicalTablePack {
    output: ValueCell,
    columns: Box<[(String, Box<[ValueCell]>)]>,
}

#[cfg(feature = "table")]
impl CanonicalTablePack {
    fn next_value(&self) -> MResult<mech_core::Value> {
        self.output.rebuild_table_cell_columns(&self.columns)
    }
}

#[cfg(feature = "table")]
impl MechFunctionImpl for CanonicalTablePack {
    fn solve_result(&self) -> MResult<()> {
        self.output.replace(&self.next_value()?)
    }

    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.clone()]
    }

    fn semantic_operation_name(&self) -> Option<&str> {
        Some("core/composite-pack")
    }

    fn to_string(&self) -> String {
        "CanonicalTablePack".to_owned()
    }
}

#[cfg(all(feature = "table", feature = "semantic-compiler"))]
impl MechFunctionCompiler for CanonicalTablePack {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        let mut cells = vec![self.output.clone()];
        cells.extend(
            self.columns
                .iter()
                .flat_map(|(_, values)| values.iter().cloned()),
        );
        cells
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let children = self
            .columns
            .iter()
            .flat_map(|(_, values)| values.iter().cloned())
            .collect::<Vec<_>>();
        compile_value_cell_composite_register(&self.output, &children, context)
    }
}

#[cfg(any(
    feature = "map",
    feature = "record",
    feature = "set",
    feature = "table",
    feature = "matrix"
))]
fn schema_mismatch(context: &'static str, expected: &SchemaBody, actual: &SchemaBody) -> MechError {
    MechError::new(
        CanonicalAggregateSchemaMismatch {
            context,
            expected: format!("{expected:?}"),
            actual: format!("{actual:?}"),
        },
        None,
    )
    .with_compiler_loc()
}

pub fn structure(
    structure: &Structure,
    _env: Option<&Environment>,
    _p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    #[allow(
        unreachable_patterns,
        reason = "the fallback is reachable only in narrow structure feature profiles"
    )]
    match structure {
        Structure::Empty => Ok(ValueCell::unit()),
        #[cfg(feature = "record")]
        Structure::Record(record_node) => record(record_node, _env, _p),
        #[cfg(feature = "matrix")]
        Structure::Matrix(matrix_node) => matrix(matrix_node, _env, _p),
        #[cfg(feature = "table")]
        Structure::Table(table_node) => table(table_node, _env, _p),
        #[cfg(feature = "tuple")]
        Structure::Tuple(tuple_node) => tuple(tuple_node, _env, _p),
        #[cfg(all(feature = "tuple", feature = "atom"))]
        Structure::TupleStruct(tuple_node) => tuple_struct(tuple_node, _env, _p),
        #[cfg(feature = "set")]
        Structure::Set(set_node) => set(set_node, _env, _p),
        #[cfg(feature = "map")]
        Structure::Map(map_node) => map(map_node, _env, _p),
        _ => Err(MechError::new(
            FeatureNotEnabledError,
            Some("feature not enabled for this structure kind".to_owned()),
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "tuple")]
pub fn tuple(
    tuple_node: &Tuple,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let mut elements = Vec::with_capacity(tuple_node.elements.len());
    for element in &tuple_node.elements {
        let value = expression_value(element, env, p, "tuple element")?;
        elements.push(value);
    }
    let output = ValueCell::tuple_from_cells(&elements)?;
    p.plan().register_instance(FunctionInstance::new(
        Box::new(CanonicalTuplePack {
            output: output.clone(),
            elements: elements.clone().into_boxed_slice(),
        }),
        FunctionInvocation::variadic(output.clone(), elements.into_boxed_slice()),
    ))?;
    Ok(output)
}

#[cfg(all(feature = "tuple", feature = "atom"))]
pub fn tuple_struct(
    tuple_node: &TupleStruct,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let payload = expression_value(&tuple_node.value, env, p, "tuple-struct payload")?;
    let variant_id = tuple_node.name.hash();
    let state = p.state.borrow();
    if let Some(definition) = state.enums.values().find(|definition| {
        definition
            .variants
            .iter()
            .any(|variant| variant.id == variant_id)
    }) {
        let schema = enum_schema(definition)?;
        let ordinal = definition
            .variants
            .iter()
            .position(|variant| variant.id == variant_id)
            .expect("matched enum variant remains present") as u32;
        return ValueCell::from_schema_data(
            schema,
            ValueDataDraft::Enum(EnumDraft {
                ordinal,
                payload: Some(Box::new(snapshot_draft(&payload)?)),
            }),
        );
    }
    drop(state);
    let tag = atom(
        &Atom {
            name: tuple_node.name.clone(),
        },
        p,
    )?;
    ValueCell::from_schema_data(
        SchemaBody::Tuple(
            vec![tag.closed_schema_body()?, payload.closed_schema_body()?].into_boxed_slice(),
        ),
        ValueDataDraft::Tuple(
            vec![snapshot_draft(&tag)?, snapshot_draft(&payload)?].into_boxed_slice(),
        ),
    )
}

#[cfg(feature = "map")]
pub fn map(
    map_node: &Map,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let mut key_schema = None;
    let mut value_schema = None;
    let mut entries = Vec::with_capacity(map_node.elements.len());
    for binding in &map_node.elements {
        let key = expression_value(&binding.key, env, p, "map key")?;
        let value = expression_value(&binding.value, env, p, "map value")?;
        let actual_key = key.closed_schema_body()?;
        let actual_value = value.closed_schema_body()?;
        if let Some(expected) = &key_schema {
            if expected != &actual_key {
                return Err(schema_mismatch("map key", expected, &actual_key));
            }
        } else {
            key_schema = Some(actual_key);
        }
        if let Some(expected) = &value_schema {
            if expected != &actual_value {
                return Err(schema_mismatch("map value", expected, &actual_value));
            }
        } else {
            value_schema = Some(actual_value);
        }
        entries.push(MapEntryDraft {
            items: vec![snapshot_draft(&key)?, snapshot_draft(&value)?].into_boxed_slice(),
        });
    }
    let key = key_schema.ok_or_else(|| {
        MechError::new(
            CanonicalAggregateTypeInferenceFailure {
                context: "empty map",
            },
            None,
        )
        .with_compiler_loc()
    })?;
    let value = value_schema.expect("non-empty map has a value schema");
    let cardinality = entries.len() as u64;
    ValueCell::from_schema_data(
        SchemaBody::Map {
            key: Box::new(key),
            value: Box::new(value),
            cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(cardinality)),
        },
        ValueDataDraft::Map(entries.into_boxed_slice()),
    )
}

#[cfg(feature = "record")]
pub fn record(
    record_node: &Record,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let mut cells = Vec::with_capacity(record_node.bindings.len());
    for binding in &record_node.bindings {
        let value = expression_value(&binding.value, env, p, "record field")?;
        let schema = match &binding.kind {
            Some(annotation) => schema_body_from_kind(&annotation.kind, p)?,
            None => value.closed_schema_body()?,
        };
        let actual = value.closed_schema_body()?;
        if schema != actual {
            return Err(schema_mismatch("record field", &schema, &actual));
        }
        let name = binding.name.to_string();
        cells.push((name, value));
    }
    let output = ValueCell::record_from_cells(&cells)?;
    p.plan().register_instance(FunctionInstance::new(
        Box::new(CanonicalRecordPack {
            output: output.clone(),
            fields: cells.clone().into_boxed_slice(),
        }),
        FunctionInvocation::variadic(
            output.clone(),
            cells
                .into_iter()
                .map(|(_, cell)| cell)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    ))?;
    Ok(output)
}

#[cfg(feature = "set")]
pub(crate) fn canonical_set_from_inputs(inputs: Vec<SpecializationInput>) -> MResult<ValueCell> {
    let concrete = inputs
        .iter()
        .filter_map(|input| input.cell().ok())
        .collect::<Vec<_>>();
    let Some(first) = concrete.first() else {
        return Err(MechError::new(
            CanonicalAggregateTypeInferenceFailure {
                context: "empty or all-absent set",
            },
            None,
        )
        .with_compiler_loc());
    };
    let element = first.closed_schema_body()?;
    for value in concrete.iter().skip(1) {
        let actual = value.closed_schema_body()?;
        if actual != element {
            return Err(schema_mismatch("set element", &element, &actual));
        }
    }
    let optional = inputs.iter().any(SpecializationInput::is_absent);
    let schema = if optional {
        SchemaBody::Option(Box::new(element))
    } else {
        element
    };
    let values = inputs
        .into_iter()
        .map(|input| match input {
            SpecializationInput::Cell(value) if optional => snapshot_draft(&value).map(|value| {
                ValueDataDraft::Option(OptionDraft {
                    present: true,
                    value: Some(Box::new(value)),
                })
            }),
            SpecializationInput::Cell(value) => snapshot_draft(&value),
            SpecializationInput::Absent => Ok(ValueDataDraft::Option(OptionDraft {
                present: false,
                value: None,
            })),
            SpecializationInput::MatrixAllSelection => Err(MechError::new(
                CanonicalAggregateSourceAbsence {
                    context: "set element",
                },
                Some("matrix all-selection is not a set value".to_owned()),
            )
            .with_compiler_loc()),
        })
        .collect::<MResult<Vec<_>>>()?;
    let cardinality = values.len() as u64;
    ValueCell::from_schema_data(
        SchemaBody::Set {
            element: Box::new(schema),
            cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(cardinality)),
        },
        ValueDataDraft::Set(values.into_boxed_slice()),
    )
}

#[cfg(feature = "set")]
pub fn set(
    set_node: &Set,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let mut inputs = Vec::with_capacity(set_node.elements.len());
    for element in &set_node.elements {
        inputs.push(expression(element, env, p)?);
    }
    canonical_set_from_inputs(inputs)
}

#[cfg(feature = "table")]
pub fn table(
    table_node: &Table,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let names = table_node
        .header
        .0
        .iter()
        .map(|field| field.name.to_string())
        .collect::<Vec<_>>();
    let mut columns = vec![Vec::<ValueCell>::new(); names.len()];
    for row in &table_node.rows {
        if row.columns.len() != names.len() {
            return Err(MechError::new(
                DimensionMismatch {
                    dims: vec![names.len(), row.columns.len()],
                },
                None,
            )
            .with_compiler_loc());
        }
        for (index, column) in row.columns.iter().enumerate() {
            columns[index].push(expression_value(&column.element, env, p, "table cell")?);
        }
    }
    let mut schema_columns = Vec::with_capacity(names.len());
    let mut cell_columns = Vec::with_capacity(names.len());
    for (index, (field, mut values)) in table_node.header.0.iter().zip(columns).enumerate() {
        let schema = if let Some(annotation) = &field.kind {
            schema_body_from_kind(&annotation.kind, p)?
        } else {
            values
                .first()
                .ok_or_else(|| {
                    MechError::new(
                        CanonicalAggregateTypeInferenceFailure {
                            context: "unannotated empty table column",
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?
                .closed_schema_body()?
        };
        if !matches!(schema, SchemaBody::Dynamic) {
            for value in &mut values {
                let actual = value.closed_schema_body()?;
                if actual == schema {
                    continue;
                }
                #[cfg(feature = "convert")]
                {
                    *value = crate::literals::convert_literal_cell(value.clone(), &schema)?;
                    continue;
                }
                #[cfg(not(feature = "convert"))]
                return Err(schema_mismatch("table column", &schema, &actual));
            }
        }
        schema_columns.push(SchemaField {
            name: names[index].clone(),
            schema,
        });
        cell_columns.push((names[index].clone(), values.into_boxed_slice()));
    }
    let row_count = table_node.rows.len() as u64;
    let canonical_columns = schema_columns
        .into_iter()
        .zip(cell_columns.iter())
        .map(|(field, (_, values))| (field, values.clone()))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let output = ValueCell::table_from_cell_columns(
        canonical_columns,
        CardinalitySpec::Exact(DimensionExpr::Constant(row_count)),
    )?;
    let dependencies = cell_columns
        .iter()
        .flat_map(|(_, values)| values.iter().cloned())
        .collect::<Vec<_>>();
    p.plan().register_instance(FunctionInstance::new(
        Box::new(CanonicalTablePack {
            output: output.clone(),
            columns: cell_columns.into_boxed_slice(),
        }),
        FunctionInvocation::variadic(output.clone(), dependencies.into_boxed_slice()),
    ))?;
    Ok(output)
}

#[cfg(feature = "matrix")]
#[derive(Clone)]
struct MatrixBlock {
    element: SchemaBody,
    rows: usize,
    columns: usize,
    values: Vec<ValueDataDraft>,
}

#[cfg(feature = "matrix")]
struct MatrixCellBlock {
    rows: usize,
    columns: usize,
    values: Vec<Option<ValueCell>>,
}

#[cfg(feature = "matrix")]
struct CanonicalMatrixPack {
    output: ValueCell,
    rows: Box<[Box<[Option<ValueCell>]>]>,
    optional: bool,
}

#[cfg(feature = "matrix")]
impl CanonicalMatrixPack {
    fn next_value(&self) -> MResult<mech_core::Value> {
        let matrix = matrix_from_source_rows(&self.rows, self.optional)?;
        self.output.rebuild_matrix_drafts(
            vec![matrix.rows as u64, matrix.columns as u64].into_boxed_slice(),
            matrix.values.into_boxed_slice(),
        )
    }
}

#[cfg(feature = "matrix")]
impl MechFunctionImpl for CanonicalMatrixPack {
    fn solve_result(&self) -> MResult<()> {
        self.output.replace(&self.next_value()?)
    }

    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.clone()]
    }

    fn to_string(&self) -> String {
        "CanonicalMatrixPack".to_owned()
    }
}

#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
impl MechFunctionCompiler for CanonicalMatrixPack {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        let mut cells = vec![self.output.clone()];
        cells.extend(
            self.rows
                .iter()
                .flatten()
                .filter_map(|cell| cell.as_ref().cloned()),
        );
        cells
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let layout = matrix_cell_layout(&self.rows)?;
        compile_value_cell_matrix_literal_register(
            &self.output,
            layout.rows.try_into().map_err(|_| {
                MechError::new(
                    GenericError {
                        msg: "matrix literal rows exceed bytecode-v1 limits".to_owned(),
                    },
                    None,
                )
                .with_compiler_loc()
            })?,
            layout.columns.try_into().map_err(|_| {
                MechError::new(
                    GenericError {
                        msg: "matrix literal columns exceed bytecode-v1 limits".to_owned(),
                    },
                    None,
                )
                .with_compiler_loc()
            })?,
            &layout.values,
            context,
        )
    }
}

#[cfg(feature = "matrix")]
fn matrix_block(cell: &ValueCell) -> MResult<MatrixBlock> {
    let schema = cell.closed_schema_body()?;
    let draft = snapshot_draft(cell)?;
    match (schema, draft) {
        (
            SchemaBody::Matrix {
                element,
                dimensions,
            },
            ValueDataDraft::Matrix(values),
        ) => {
            let [
                DimensionExpr::Constant(rows),
                DimensionExpr::Constant(columns),
            ] = dimensions.as_ref()
            else {
                return Err(MechError::new(
                    CanonicalAggregateTypeInferenceFailure {
                        context: "matrix literal extent",
                    },
                    None,
                )
                .with_compiler_loc());
            };
            Ok(MatrixBlock {
                element: *element,
                rows: *rows as usize,
                columns: *columns as usize,
                values: values.into_vec(),
            })
        }
        (element, value) => Ok(MatrixBlock {
            element,
            rows: 1,
            columns: 1,
            values: vec![value],
        }),
    }
}

#[cfg(feature = "matrix")]
fn matrix_cell_block(cell: &ValueCell) -> MResult<MatrixCellBlock> {
    let SchemaBody::Matrix { dimensions, .. } = cell.closed_schema_body()? else {
        return Ok(MatrixCellBlock {
            rows: 1,
            columns: 1,
            values: vec![Some(cell.clone())],
        });
    };
    let [
        DimensionExpr::Constant(rows),
        DimensionExpr::Constant(columns),
    ] = dimensions.as_ref()
    else {
        return Err(MechError::new(
            CanonicalAggregateTypeInferenceFailure {
                context: "matrix literal extent",
            },
            None,
        )
        .with_compiler_loc());
    };
    Ok(MatrixCellBlock {
        rows: *rows as usize,
        columns: *columns as usize,
        values: cell
            .matrix_elements()?
            .expect("matrix schema retains matrix elements")
            .into_iter()
            .map(Some)
            .collect(),
    })
}

#[cfg(feature = "matrix")]
fn horizontal_cell_blocks(blocks: Vec<MatrixCellBlock>) -> MResult<MatrixCellBlock> {
    let Some(first) = blocks.first() else {
        return Err(MechError::new(
            CanonicalAggregateTypeInferenceFailure {
                context: "empty matrix row",
            },
            None,
        )
        .with_compiler_loc());
    };
    let rows = first.rows;
    let columns = blocks.iter().map(|block| block.columns).sum();
    if let Some(block) = blocks.iter().find(|block| block.rows != rows) {
        return Err(MechError::new(
            DimensionMismatch {
                dims: vec![rows, block.rows],
            },
            None,
        )
        .with_compiler_loc());
    }
    let mut values = Vec::with_capacity(rows.saturating_mul(columns));
    for row in 0..rows {
        for block in &blocks {
            let start = row * block.columns;
            values.extend_from_slice(&block.values[start..start + block.columns]);
        }
    }
    Ok(MatrixCellBlock {
        rows,
        columns,
        values,
    })
}

#[cfg(feature = "matrix")]
fn matrix_cell_layout(rows: &[Box<[Option<ValueCell>]>]) -> MResult<MatrixCellBlock> {
    let mut blocks = Vec::with_capacity(rows.len());
    for row in rows {
        blocks.push(horizontal_cell_blocks(
            row.iter()
                .map(|cell| match cell {
                    Some(cell) => matrix_cell_block(cell),
                    None => Ok(MatrixCellBlock {
                        rows: 1,
                        columns: 1,
                        values: vec![None],
                    }),
                })
                .collect::<MResult<Vec<_>>>()?,
        )?);
    }
    let Some(first) = blocks.first() else {
        return Ok(MatrixCellBlock {
            rows: 0,
            columns: 0,
            values: Vec::new(),
        });
    };
    let columns = first.columns;
    if let Some(block) = blocks.iter().find(|block| block.columns != columns) {
        return Err(MechError::new(
            DimensionMismatch {
                dims: vec![columns, block.columns],
            },
            None,
        )
        .with_compiler_loc());
    }
    Ok(MatrixCellBlock {
        rows: blocks.iter().map(|block| block.rows).sum(),
        columns,
        values: blocks.into_iter().flat_map(|block| block.values).collect(),
    })
}

#[cfg(feature = "matrix")]
fn horizontal(blocks: Vec<MatrixBlock>) -> MResult<MatrixBlock> {
    let Some(first) = blocks.first() else {
        return Err(MechError::new(
            CanonicalAggregateTypeInferenceFailure {
                context: "empty matrix row",
            },
            None,
        )
        .with_compiler_loc());
    };
    let element = first.element.clone();
    let rows = first.rows;
    let mut columns = 0;
    for block in &blocks {
        if block.rows != rows {
            return Err(MechError::new(
                DimensionMismatch {
                    dims: vec![rows, block.rows],
                },
                None,
            )
            .with_compiler_loc());
        }
        if block.element != element {
            return Err(schema_mismatch(
                "horizontal matrix literal",
                &element,
                &block.element,
            ));
        }
        columns += block.columns;
    }
    let mut values = Vec::with_capacity(rows.saturating_mul(columns));
    for row in 0..rows {
        for block in &blocks {
            let start = row * block.columns;
            values.extend_from_slice(&block.values[start..start + block.columns]);
        }
    }
    Ok(MatrixBlock {
        element,
        rows,
        columns,
        values,
    })
}

#[cfg(feature = "matrix")]
fn vertical(blocks: Vec<MatrixBlock>) -> MResult<MatrixBlock> {
    let Some(first) = blocks.first() else {
        return ValueCell::dynamic_matrix(
            SchemaBody::Tuple(Box::new([])),
            vec![0, 0].into_boxed_slice(),
            Box::new([]),
        )
        .and_then(|cell| matrix_block(&cell));
    };
    let element = first.element.clone();
    let columns = first.columns;
    let rows = blocks.iter().map(|block| block.rows).sum();
    for block in &blocks {
        if block.columns != columns {
            return Err(MechError::new(
                DimensionMismatch {
                    dims: vec![columns, block.columns],
                },
                None,
            )
            .with_compiler_loc());
        }
        if block.element != element {
            return Err(schema_mismatch(
                "vertical matrix literal",
                &element,
                &block.element,
            ));
        }
    }
    let values = blocks.into_iter().flat_map(|block| block.values).collect();
    Ok(MatrixBlock {
        element,
        rows,
        columns,
        values,
    })
}

#[cfg(feature = "matrix")]
fn matrix_from_source_rows(
    inputs: &[Box<[Option<ValueCell>]>],
    optional: bool,
) -> MResult<MatrixBlock> {
    let element = inputs
        .iter()
        .flatten()
        .find_map(|input| input.as_ref())
        .map(matrix_block)
        .transpose()?
        .map(|block| block.element)
        .unwrap_or_else(|| SchemaBody::Tuple(Box::new([])));
    let rows = inputs
        .iter()
        .map(|row| {
            let blocks = row
                .iter()
                .map(|input| match input {
                    Some(value) => {
                        let mut block = matrix_block(value)?;
                        if optional {
                            if block.element != element {
                                return Err(schema_mismatch(
                                    "optional matrix literal",
                                    &element,
                                    &block.element,
                                ));
                            }
                            block.element = SchemaBody::Option(Box::new(element.clone()));
                            block.values = block
                                .values
                                .into_iter()
                                .map(|value| {
                                    ValueDataDraft::Option(OptionDraft {
                                        present: true,
                                        value: Some(Box::new(value)),
                                    })
                                })
                                .collect();
                        }
                        Ok(block)
                    }
                    None => Ok(MatrixBlock {
                        element: SchemaBody::Option(Box::new(element.clone())),
                        rows: 1,
                        columns: 1,
                        values: vec![ValueDataDraft::Option(OptionDraft {
                            present: false,
                            value: None,
                        })],
                    }),
                })
                .collect::<MResult<Vec<_>>>()?;
            horizontal(blocks)
        })
        .collect::<MResult<Vec<_>>>()?;
    vertical(rows)
}

#[cfg(feature = "matrix")]
pub fn matrix(
    matrix_node: &Mat,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let rows = matrix_node
        .rows
        .iter()
        .map(|row| {
            row.columns
                .iter()
                .map(|column| match expression(&column.element, env, p)? {
                    SpecializationInput::Cell(value) => Ok(Some(value)),
                    SpecializationInput::Absent => Ok(None),
                    SpecializationInput::MatrixAllSelection => Err(MechError::new(
                        CanonicalAggregateSourceAbsence {
                            context: "matrix element",
                        },
                        Some(
                            "matrix all-selection is only valid in a selector position".to_owned(),
                        ),
                    )
                    .with_compiler_loc()),
                })
                .collect::<MResult<Vec<_>>>()
                .map(Vec::into_boxed_slice)
        })
        .collect::<MResult<Vec<_>>>()?
        .into_boxed_slice();
    let optional = rows.iter().flatten().any(Option::is_none);
    if optional && rows.iter().flatten().all(Option::is_none) {
        return Err(MechError::new(
            CanonicalAggregateTypeInferenceFailure {
                context: "all-absent matrix literal",
            },
            None,
        )
        .with_compiler_loc());
    }
    let matrix = matrix_from_source_rows(&rows, optional)?;
    #[cfg(feature = "matrix_horzcat")]
    if !optional && (rows.len() == 1 || cfg!(feature = "matrix_vertcat")) {
        let plan = p.plan();
        let mut row_values = Vec::with_capacity(rows.len());
        for row in &rows {
            let arguments = row
                .iter()
                .map(|cell| {
                    cell.clone().ok_or_else(|| {
                        MechError::new(
                            CanonicalAggregateSourceAbsence {
                                context: "matrix row",
                            },
                            None,
                        )
                        .with_compiler_loc()
                    })
                })
                .collect::<MResult<Vec<_>>>()?;
            row_values.push(execute_catalog_operation_with_registration_arguments(
                p,
                &plan,
                "matrix/horzcat",
                arguments.clone(),
                arguments,
            )?);
        }
        if row_values.len() == 1 {
            return Ok(row_values.pop().expect("one matrix row was constructed"));
        }
        #[cfg(feature = "matrix_vertcat")]
        {
            return execute_catalog_operation_with_registration_arguments(
                p,
                &plan,
                "matrix/vertcat",
                row_values.clone(),
                row_values,
            );
        }
    }
    let output = ValueCell::dynamic_matrix(
        matrix.element,
        vec![matrix.rows as u64, matrix.columns as u64].into_boxed_slice(),
        matrix.values.into_boxed_slice(),
    )?;
    let dependencies = rows
        .iter()
        .flatten()
        .filter_map(|cell| cell.as_ref().cloned())
        .collect::<Vec<_>>();
    p.plan().register_instance(FunctionInstance::new(
        Box::new(CanonicalMatrixPack {
            output: output.clone(),
            rows,
            optional,
        }),
        FunctionInvocation::variadic(output.clone(), dependencies.into_boxed_slice()),
    ))?;
    Ok(output)
}

#[cfg(feature = "kind_annotation")]
pub(crate) fn schema_body_from_kind(
    kind: &mech_core::nodes::Kind,
    p: &InterpreterExecution<'_>,
) -> MResult<SchemaBody> {
    match kind {
        mech_core::nodes::Kind::Any => Ok(SchemaBody::Dynamic),
        mech_core::nodes::Kind::Scalar(identifier) => {
            schema_body_from_scalar_name(&identifier.to_string(), p)
        }
        mech_core::nodes::Kind::Atom(identifier) => {
            let path = source_nominal_path(&identifier.to_string())?;
            Ok(SchemaBody::Atom(NominalKey::from_path(
                NominalKind::Atom,
                &path,
            )))
        }
        mech_core::nodes::Kind::Option(element) => Ok(SchemaBody::Option(Box::new(
            schema_body_from_kind(element, p)?,
        ))),
        mech_core::nodes::Kind::Tuple(elements) => Ok(SchemaBody::Tuple(
            elements
                .iter()
                .map(|element| schema_body_from_kind(element, p))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        )),
        mech_core::nodes::Kind::Record(fields) => Ok(SchemaBody::Record(
            fields
                .iter()
                .map(|(name, kind)| {
                    Ok(SchemaField {
                        name: name.to_string(),
                        schema: schema_body_from_kind(kind, p)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        )),
        mech_core::nodes::Kind::Matrix((element, dimensions)) => Ok(SchemaBody::Matrix {
            element: Box::new(schema_body_from_kind(element, p)?),
            dimensions: dimensions
                .iter()
                .map(|dimension| {
                    crate::literals::literal_usize(dimension, p).map(|value| {
                        value.map_or(DimensionExpr::Hole, |value| {
                            DimensionExpr::Constant(value as u64)
                        })
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        }),
        mech_core::nodes::Kind::Map(key, value) => Ok(SchemaBody::Map {
            key: Box::new(schema_body_from_kind(key, p)?),
            value: Box::new(schema_body_from_kind(value, p)?),
            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
        }),
        mech_core::nodes::Kind::Table((columns, rows)) => Ok(SchemaBody::Table {
            columns: columns
                .iter()
                .map(|(name, kind)| {
                    Ok(SchemaField {
                        name: name.to_string(),
                        schema: schema_body_from_kind(kind, p)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
            rows: crate::literals::literal_usize(rows, p)?
                .map_or(CardinalitySpec::Dynamic { upper_bound: None }, |value| {
                    CardinalitySpec::Exact(DimensionExpr::Constant(value as u64))
                }),
        }),
        mech_core::nodes::Kind::Set(element, cardinality) => Ok(SchemaBody::Set {
            element: Box::new(schema_body_from_kind(element, p)?),
            cardinality: cardinality
                .as_ref()
                .map(|value| crate::literals::literal_usize(value, p))
                .transpose()?
                .flatten()
                .map_or(CardinalitySpec::Dynamic { upper_bound: None }, |value| {
                    CardinalitySpec::Exact(DimensionExpr::Constant(value as u64))
                }),
        }),
        _ => Err(MechError::new(
            CanonicalAggregateTypeInferenceFailure {
                context: "kind annotation",
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "kind_annotation")]
fn schema_body_from_scalar_name(name: &str, p: &InterpreterExecution<'_>) -> MResult<SchemaBody> {
    Ok(match name.rsplit('/').next().unwrap_or(name) {
        "u8" => SchemaBody::UnsignedInteger(IntegerWidth::W8),
        "u16" => SchemaBody::UnsignedInteger(IntegerWidth::W16),
        "u32" => SchemaBody::UnsignedInteger(IntegerWidth::W32),
        "u64" => SchemaBody::UnsignedInteger(IntegerWidth::W64),
        "u128" => SchemaBody::UnsignedInteger(IntegerWidth::W128),
        "i8" => SchemaBody::SignedInteger(IntegerWidth::W8),
        "i16" => SchemaBody::SignedInteger(IntegerWidth::W16),
        "i32" => SchemaBody::SignedInteger(IntegerWidth::W32),
        "i64" => SchemaBody::SignedInteger(IntegerWidth::W64),
        "i128" => SchemaBody::SignedInteger(IntegerWidth::W128),
        "f32" => SchemaBody::FloatingPoint(FloatWidth::W32),
        "f64" => SchemaBody::FloatingPoint(FloatWidth::W64),
        "c64" => SchemaBody::Complex(FloatWidth::W64),
        "r64" => SchemaBody::Rational64,
        "string" => SchemaBody::String,
        "bool" => SchemaBody::Bool,
        "id" => SchemaBody::Id,
        "index" => SchemaBody::Index,
        _ => {
            let id = hash_str(name);
            let state = p.state.borrow();
            if let Some(schema) = state.kinds.get(&id) {
                return Ok(schema.clone());
            }
            let definition = state.enums.get(&id).ok_or_else(|| {
                MechError::new(
                    CanonicalAggregateTypeInferenceFailure {
                        context: "named kind annotation",
                    },
                    Some(name.to_owned()),
                )
                .with_compiler_loc()
            })?;
            enum_schema(definition)?
        }
    })
}

#[cfg(all(feature = "tuple", feature = "atom"))]
pub(crate) fn enum_schema(definition: &CanonicalEnumDefinition) -> MResult<SchemaBody> {
    let path = source_nominal_path(&definition.name)?;
    let variants = definition
        .variants
        .iter()
        .map(|variant| EnumVariantSchema {
            name: variant.name.clone(),
            payload: variant.payload.clone(),
        })
        .collect::<Vec<_>>();
    Ok(SchemaBody::Enum {
        key: NominalKey::from_path(NominalKind::Enum, &path),
        variants: variants.into_boxed_slice(),
    })
}

#[cfg(any(feature = "kind_annotation", all(feature = "tuple", feature = "atom")))]
fn source_nominal_path(name: &str) -> MResult<CanonicalNominalPath> {
    Ok(CanonicalNominalPath::new(
        name.split('/')
            .filter(|segment| !segment.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>(),
    )?)
}

#[derive(Debug, Clone)]
pub struct CanonicalAggregateSourceAbsence {
    pub context: &'static str,
}

impl MechErrorKind for CanonicalAggregateSourceAbsence {
    fn name(&self) -> &str {
        "CanonicalAggregateSourceAbsence"
    }

    fn message(&self) -> String {
        format!("source absence is not a value in {}", self.context)
    }
}

#[derive(Debug, Clone)]
pub struct CanonicalAggregateSchemaMismatch {
    pub context: &'static str,
    pub expected: String,
    pub actual: String,
}

impl MechErrorKind for CanonicalAggregateSchemaMismatch {
    fn name(&self) -> &str {
        "CanonicalAggregateSchemaMismatch"
    }

    fn message(&self) -> String {
        format!(
            "{} expected schema {}, found {}",
            self.context, self.expected, self.actual
        )
    }
}

#[derive(Debug, Clone)]
pub struct CanonicalAggregateTypeInferenceFailure {
    pub context: &'static str,
}

impl MechErrorKind for CanonicalAggregateTypeInferenceFailure {
    fn name(&self) -> &str {
        "CanonicalAggregateTypeInferenceFailure"
    }

    fn message(&self) -> String {
        format!(
            "cannot infer a closed canonical schema for {}",
            self.context
        )
    }
}

#[cfg(all(
    test,
    feature = "f64",
    feature = "kind_annotation",
    feature = "semantic-compiler",
    feature = "table"
))]
mod canonical_kind_annotation_tests {
    use super::*;

    #[test]
    fn wildcard_table_column_uses_the_canonical_dynamic_schema() {
        let tree = mech_syntax::parser::parse(
            "value := | payload<*> amount<f64> |\n         | \"item\"     1           |\nvalue",
        )
        .unwrap();
        let mut interpreter = Interpreter::with_function_catalog(
            0,
            10_000,
            crate::test_support::catalog::function_catalog(),
        );
        let output = interpreter.interpret(&tree).unwrap().unwrap();
        let SchemaBody::Table { columns, .. } = output.closed_schema_body().unwrap() else {
            panic!("source table must retain its canonical table schema")
        };
        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].name, "payload");
        assert_eq!(columns[0].schema, SchemaBody::Dynamic);
        assert_eq!(columns[1].name, "amount");
        assert_eq!(
            columns[1].schema,
            SchemaBody::FloatingPoint(FloatWidth::W64)
        );
    }
}
