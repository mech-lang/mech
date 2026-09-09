use crate::intrinsics::*;
use mech_core::snapshot::{OptionDraft, TableColumnDraft};
use std::sync::LazyLock;

pub(crate) static PURE_TABLE_JOIN_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

#[derive(Clone, Copy, Debug)]
pub(crate) enum JoinMode {
    Inner,
    LeftOuter,
    RightOuter,
    FullOuter,
    LeftSemi,
    LeftAnti,
}

#[derive(Clone)]
struct CanonicalTable {
    fields: Box<[SchemaField]>,
    columns: Box<[TableColumnDraft]>,
    canonical_columns: Box<[Box<[ValueData]>]>,
    rows: usize,
}

impl CanonicalTable {
    fn from_cell(cell: &ValueCell) -> MResult<Self> {
        let snapshot = cell.snapshot()?;
        Self::from_value(cell, &snapshot)
    }

    fn from_value(cell: &ValueCell, snapshot: &mech_core::Value) -> MResult<Self> {
        let SchemaBody::Table { columns, .. } = cell.closed_schema_body()? else {
            return Err(table_join_error("input must be a canonical table"));
        };
        let ValueDataDraft::Table(values) = snapshot.canonical_data_draft().map_err(|error| {
            MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
        })?
        else {
            return Err(table_join_error("table input has a non-table payload"));
        };
        let rows = values.first().map_or(0, |column| column.values.len());
        if values.iter().any(|column| column.values.len() != rows) {
            return Err(table_join_error(
                "table columns have inconsistent row counts",
            ));
        }
        let ValueData::Table(table) = snapshot.data() else {
            return Err(table_join_error("table input has a non-table snapshot"));
        };
        let canonical_columns = (0..columns.len())
            .map(|index| {
                table
                    .column(index)
                    .map(|column| column.to_values().into_boxed_slice())
                    .ok_or_else(|| table_join_error("table snapshot omitted a schema column"))
            })
            .collect::<MResult<Vec<_>>>()?
            .into_boxed_slice();
        Ok(Self {
            fields: columns,
            columns: values,
            canonical_columns,
            rows,
        })
    }

    fn value(&self, column: usize, row: usize) -> &ValueDataDraft {
        &self.columns[column].values[row]
    }

    fn canonical_value(&self, column: usize, row: usize) -> &ValueData {
        &self.canonical_columns[column][row]
    }
}

fn table_join_error(message: impl Into<String>) -> MechError {
    MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
    .with_compiler_loc()
}

fn optional_schema(schema: &SchemaBody) -> SchemaBody {
    match schema {
        SchemaBody::Option(_) => schema.clone(),
        schema => SchemaBody::Option(Box::new(schema.clone())),
    }
}

fn present_for_schema(
    target: &SchemaBody,
    source: &SchemaBody,
    value: &ValueDataDraft,
) -> ValueDataDraft {
    if matches!(target, SchemaBody::Option(_)) && !matches!(source, SchemaBody::Option(_)) {
        ValueDataDraft::Option(OptionDraft {
            present: true,
            value: Some(Box::new(value.clone())),
        })
    } else {
        value.clone()
    }
}

fn absent_for_schema(target: &SchemaBody) -> MResult<ValueDataDraft> {
    if matches!(target, SchemaBody::Option(_)) {
        Ok(ValueDataDraft::Option(OptionDraft {
            present: false,
            value: None,
        }))
    } else {
        Err(table_join_error(
            "outer join attempted to omit a non-optional output column",
        ))
    }
}

fn rows_match(
    lhs: &CanonicalTable,
    lhs_row: usize,
    rhs: &CanonicalTable,
    rhs_row: usize,
    common: &[(usize, usize)],
) -> bool {
    common.iter().all(|(left, right)| {
        mech_core::snapshot::schema_data_language_eq(
            &lhs.fields[*left].schema,
            lhs.canonical_value(*left, lhs_row),
            rhs.canonical_value(*right, rhs_row),
        )
    })
}

fn common_columns(lhs: &[SchemaField], rhs: &[SchemaField]) -> MResult<Vec<(usize, usize)>> {
    let common = lhs
        .iter()
        .enumerate()
        .filter_map(|(left, field)| {
            rhs.iter()
                .position(|candidate| candidate.name == field.name)
                .map(|right| (left, right))
        })
        .collect::<Vec<_>>();
    if common
        .iter()
        .any(|(left, right)| lhs[*left].schema != rhs[*right].schema)
    {
        return Err(table_join_error(
            "common join columns must have identical schemas",
        ));
    }
    Ok(common)
}

fn common_columns_with_construction(
    lhs: &[SchemaField],
    rhs: &[SchemaField],
    construction: &mut FrozenSnapshotConstruction,
) -> MResult<Vec<(usize, usize)>> {
    let count = lhs
        .iter()
        .filter(|field| rhs.iter().any(|candidate| candidate.name == field.name))
        .count();
    let mut common = construction.try_vec_with_capacity(count)?;
    for (left, field) in lhs.iter().enumerate() {
        let Some(right) = rhs
            .iter()
            .position(|candidate| candidate.name == field.name)
        else {
            continue;
        };
        if field.schema != rhs[right].schema {
            return Err(table_join_error(
                "common join columns must have identical schemas",
            ));
        }
        common.push((left, right));
    }
    Ok(common)
}

pub(crate) fn joined_table_fields(
    lhs: &[SchemaField],
    rhs: &[SchemaField],
    mode: JoinMode,
) -> MResult<Box<[SchemaField]>> {
    let common = common_columns(lhs, rhs)?;
    let common_rhs = common
        .iter()
        .map(|(_, right)| *right)
        .collect::<std::collections::BTreeSet<_>>();
    let lhs_outer = matches!(mode, JoinMode::RightOuter | JoinMode::FullOuter);
    let rhs_outer = matches!(mode, JoinMode::LeftOuter | JoinMode::FullOuter);
    let lhs_only = matches!(mode, JoinMode::LeftSemi | JoinMode::LeftAnti);
    let mut fields = lhs
        .iter()
        .enumerate()
        .map(|(index, field)| SchemaField {
            name: field.name.clone(),
            schema: if lhs_outer && !common.iter().any(|(left, _)| *left == index) {
                optional_schema(&field.schema)
            } else {
                field.schema.clone()
            },
        })
        .collect::<Vec<_>>();
    if !lhs_only {
        fields.extend(
            rhs.iter()
                .enumerate()
                .filter(|(index, _)| !common_rhs.contains(index))
                .map(|(_, field)| SchemaField {
                    name: field.name.clone(),
                    schema: if rhs_outer {
                        optional_schema(&field.schema)
                    } else {
                        field.schema.clone()
                    },
                }),
        );
    }
    Ok(fields.into_boxed_slice())
}

pub(crate) fn joined_table(lhs: &ValueCell, rhs: &ValueCell, mode: JoinMode) -> MResult<ValueCell> {
    let lhs = CanonicalTable::from_cell(lhs)?;
    let rhs = CanonicalTable::from_cell(rhs)?;
    let (schema, data) = joined_table_data(&lhs, &rhs, mode)?;
    ValueCell::from_schema_data(schema, data)
}

fn joined_table_data(
    lhs: &CanonicalTable,
    rhs: &CanonicalTable,
    mode: JoinMode,
) -> MResult<(SchemaBody, ValueDataDraft)> {
    let common = common_columns(&lhs.fields, &rhs.fields)?;
    let common_rhs = common
        .iter()
        .map(|(_, right)| *right)
        .collect::<std::collections::BTreeSet<_>>();

    let lhs_only = matches!(mode, JoinMode::LeftSemi | JoinMode::LeftAnti);
    let fields = joined_table_fields(&lhs.fields, &rhs.fields, mode)?;

    let mut row_pairs = Vec::new();
    let mut rhs_matched = vec![false; rhs.rows];
    for lhs_row in 0..lhs.rows {
        let matches = (0..rhs.rows)
            .filter(|rhs_row| rows_match(&lhs, lhs_row, &rhs, *rhs_row, &common))
            .collect::<Vec<_>>();
        match mode {
            JoinMode::Inner | JoinMode::RightOuter => {
                for rhs_row in matches {
                    rhs_matched[rhs_row] = true;
                    row_pairs.push((Some(lhs_row), Some(rhs_row)));
                }
            }
            JoinMode::LeftOuter | JoinMode::FullOuter => {
                if matches.is_empty() {
                    row_pairs.push((Some(lhs_row), None));
                } else {
                    for rhs_row in matches {
                        rhs_matched[rhs_row] = true;
                        row_pairs.push((Some(lhs_row), Some(rhs_row)));
                    }
                }
            }
            JoinMode::LeftSemi if !matches.is_empty() => row_pairs.push((Some(lhs_row), None)),
            JoinMode::LeftAnti if matches.is_empty() => row_pairs.push((Some(lhs_row), None)),
            JoinMode::LeftSemi | JoinMode::LeftAnti => {}
        }
    }
    if matches!(mode, JoinMode::RightOuter | JoinMode::FullOuter) {
        row_pairs.extend(
            rhs_matched
                .iter()
                .enumerate()
                .filter(|(_, matched)| !**matched)
                .map(|(row, _)| (None, Some(row))),
        );
    }

    let mut output_columns = fields
        .iter()
        .map(|field| TableColumnDraft {
            name: field.name.clone(),
            values: Box::new([]),
        })
        .collect::<Vec<_>>();
    let mut values = vec![Vec::with_capacity(row_pairs.len()); fields.len()];
    for (lhs_row, rhs_row) in row_pairs {
        for (index, field) in lhs.fields.iter().enumerate() {
            let target = &fields[index].schema;
            let value = if let Some(row) = lhs_row {
                present_for_schema(target, &field.schema, lhs.value(index, row))
            } else if let Some((_, rhs_index)) = common.iter().find(|(left, _)| *left == index) {
                let row = rhs_row.expect("right outer row has a right source");
                present_for_schema(
                    target,
                    &rhs.fields[*rhs_index].schema,
                    rhs.value(*rhs_index, row),
                )
            } else {
                absent_for_schema(target)?
            };
            values[index].push(value);
        }
        if !lhs_only {
            let mut output = lhs.fields.len();
            for (index, field) in rhs.fields.iter().enumerate() {
                if common_rhs.contains(&index) {
                    continue;
                }
                let target = &fields[output].schema;
                let value = if let Some(row) = rhs_row {
                    present_for_schema(target, &field.schema, rhs.value(index, row))
                } else {
                    absent_for_schema(target)?
                };
                values[output].push(value);
                output += 1;
            }
        }
    }
    for (column, values) in output_columns.iter_mut().zip(values) {
        column.values = values.into_boxed_slice();
    }
    Ok((
        SchemaBody::Table {
            columns: fields,
            rows: CardinalitySpec::Dynamic { upper_bound: None },
        },
        ValueDataDraft::Table(output_columns.into_boxed_slice()),
    ))
}

fn visit_join_row_pairs(
    lhs: &CanonicalTable,
    rhs: &CanonicalTable,
    mode: JoinMode,
    common: &[(usize, usize)],
    rhs_matched: &mut [bool],
    mut visit: impl FnMut(Option<usize>, Option<usize>) -> MResult<()>,
) -> MResult<()> {
    rhs_matched.fill(false);
    for lhs_row in 0..lhs.rows {
        let mut any_match = false;
        for rhs_row in 0..rhs.rows {
            if !rows_match(lhs, lhs_row, rhs, rhs_row, common) {
                continue;
            }
            any_match = true;
            rhs_matched[rhs_row] = true;
            match mode {
                JoinMode::Inner
                | JoinMode::LeftOuter
                | JoinMode::RightOuter
                | JoinMode::FullOuter => visit(Some(lhs_row), Some(rhs_row))?,
                JoinMode::LeftSemi => break,
                JoinMode::LeftAnti => {}
            }
        }
        match mode {
            JoinMode::LeftOuter | JoinMode::FullOuter if !any_match => visit(Some(lhs_row), None)?,
            JoinMode::LeftSemi if any_match => visit(Some(lhs_row), None)?,
            JoinMode::LeftAnti if !any_match => visit(Some(lhs_row), None)?,
            _ => {}
        }
    }
    if matches!(mode, JoinMode::RightOuter | JoinMode::FullOuter) {
        for (rhs_row, matched) in rhs_matched.iter().enumerate() {
            if !matched {
                visit(None, Some(rhs_row))?;
            }
        }
    }
    Ok(())
}

fn joined_table_data_with_construction(
    lhs: &CanonicalTable,
    rhs: &CanonicalTable,
    mode: JoinMode,
    construction: &mut FrozenSnapshotConstruction,
) -> MResult<(SchemaBody, ValueDataDraft)> {
    let common = common_columns_with_construction(&lhs.fields, &rhs.fields, construction)?;
    let lhs_only = matches!(mode, JoinMode::LeftSemi | JoinMode::LeftAnti);
    let rhs_output_count = if lhs_only {
        0
    } else {
        rhs.fields
            .iter()
            .enumerate()
            .filter(|(right, _)| !common.iter().any(|(_, candidate)| candidate == right))
            .count()
    };
    let field_count = lhs
        .fields
        .len()
        .checked_add(rhs_output_count)
        .ok_or_else(|| table_join_error("table join column count overflows"))?;

    let mut rhs_matched = construction.try_vec_with_capacity(rhs.rows)?;
    rhs_matched.resize(rhs.rows, false);
    let mut pair_count = 0usize;
    visit_join_row_pairs(lhs, rhs, mode, &common, &mut rhs_matched, |_, _| {
        pair_count = pair_count
            .checked_add(1)
            .ok_or_else(|| table_join_error("table join row count overflows"))?;
        Ok(())
    })?;
    let mut row_pairs = construction.try_vec_with_capacity(pair_count)?;
    visit_join_row_pairs(lhs, rhs, mode, &common, &mut rhs_matched, |left, right| {
        row_pairs.push((left, right));
        Ok(())
    })?;

    let mut fields = construction.try_vec_with_capacity(field_count)?;
    let mut output_columns = construction.try_vec_with_capacity(field_count)?;
    let mut values = construction.try_vec_with_capacity(field_count)?;
    for _ in 0..field_count {
        values.push(construction.try_vec_with_capacity(pair_count)?);
    }

    construction.try_finish_preallocated_with(move || {
        let lhs_outer = matches!(mode, JoinMode::RightOuter | JoinMode::FullOuter);
        let rhs_outer = matches!(mode, JoinMode::LeftOuter | JoinMode::FullOuter);
        for (index, field) in lhs.fields.iter().enumerate() {
            fields.push(SchemaField {
                name: field.name.clone(),
                schema: if lhs_outer && !common.iter().any(|(left, _)| *left == index) {
                    optional_schema(&field.schema)
                } else {
                    field.schema.clone()
                },
            });
        }
        if !lhs_only {
            for (right, field) in rhs.fields.iter().enumerate() {
                if common.iter().any(|(_, candidate)| *candidate == right) {
                    continue;
                }
                fields.push(SchemaField {
                    name: field.name.clone(),
                    schema: if rhs_outer {
                        optional_schema(&field.schema)
                    } else {
                        field.schema.clone()
                    },
                });
            }
        }

        for field in &fields {
            output_columns.push(TableColumnDraft {
                name: field.name.clone(),
                values: Box::new([]),
            });
        }
        for (lhs_row, rhs_row) in row_pairs {
            for (index, field) in lhs.fields.iter().enumerate() {
                let target = &fields[index].schema;
                let value = if let Some(row) = lhs_row {
                    present_for_schema(target, &field.schema, lhs.value(index, row))
                } else if let Some((_, rhs_index)) = common.iter().find(|(left, _)| *left == index)
                {
                    let row = rhs_row.expect("right outer row has a right source");
                    present_for_schema(
                        target,
                        &rhs.fields[*rhs_index].schema,
                        rhs.value(*rhs_index, row),
                    )
                } else {
                    absent_for_schema(target)?
                };
                values[index].push(value);
            }
            if !lhs_only {
                let mut output = lhs.fields.len();
                for (index, field) in rhs.fields.iter().enumerate() {
                    if common.iter().any(|(_, right)| *right == index) {
                        continue;
                    }
                    let target = &fields[output].schema;
                    let value = if let Some(row) = rhs_row {
                        present_for_schema(target, &field.schema, rhs.value(index, row))
                    } else {
                        absent_for_schema(target)?
                    };
                    values[output].push(value);
                    output += 1;
                }
            }
        }
        for (column, values) in output_columns.iter_mut().zip(values) {
            column.values = values.into_boxed_slice();
        }
        Ok((
            SchemaBody::Table {
                columns: fields.into_boxed_slice(),
                rows: CardinalitySpec::Dynamic { upper_bound: None },
            },
            ValueDataDraft::Table(output_columns.into_boxed_slice()),
        ))
    })
}

#[derive(Debug)]
struct TableJoinFxn {
    lhs: FunctionValueInput,
    rhs: FunctionValueInput,
    out: FunctionValueOutput,
    mode: JoinMode,
}

impl TableJoinFxn {
    fn from_invocation(
        invocation: FunctionInvocation,
        mode: JoinMode,
    ) -> MResult<Box<dyn MechFunction>> {
        let (out, lhs, rhs) = invocation.expect_binary()?;
        Ok(Box::new(Self {
            lhs: lhs.value(),
            rhs: rhs.value(),
            out: out.value(),
            mode,
        }))
    }

    fn prospective_output_footprint(&self) -> MResult<CurrentMemoryFootprint> {
        let row_count = |input: &ValueCell| -> MResult<u64> {
            let value = input.snapshot()?;
            let ValueData::Table(table) = value.data() else {
                return Err(table_join_error("input must be a canonical table"));
            };
            if table.is_empty() {
                return Ok(0);
            }
            table
                .column(0)
                .and_then(|values| u64::try_from(values.len()).ok())
                .ok_or_else(|| table_join_error("table row count exceeds memory-plan limits"))
        };
        let lhs_rows = row_count(self.lhs.cell())?;
        let rhs_rows = row_count(self.rhs.cell())?;
        self.out.cell().prospective_aggregate_memory_footprint([
            (self.lhs.cell(), rhs_rows.max(1)),
            (self.rhs.cell(), lhs_rows.max(1)),
        ])
    }
}

impl MechFunctionImpl for TableJoinFxn {
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(Some(
            vec![self.prospective_output_footprint()?].into_boxed_slice(),
        ))
    }

    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        let footprint = self.prospective_output_footprint()?;
        frame.with_admitted_canonical_output(
            self.out.cell(),
            footprint,
            |frame, construction| {
                let lhs = frame.snapshot_input_cell_with_construction(
                    self.lhs.cell(),
                    0,
                    construction,
                )?;
                let rhs = frame.snapshot_input_cell_with_construction(
                    self.rhs.cell(),
                    1,
                    construction,
                )?;
                let lhs = CanonicalTable::from_value(self.lhs.cell(), &lhs)?;
                let rhs = CanonicalTable::from_value(self.rhs.cell(), &rhs)?;
                let (_, draft) =
                    joined_table_data_with_construction(&lhs, &rhs, self.mode, construction)?;
                let next = construction.try_rebuild_data_draft(self.out.cell(), draft)?;
                Ok(((), next))
            },
        )?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_TABLE_JOIN_CONTRACT)
    }

    fn to_string(&self) -> String {
        format!("TableJoinFxn::{:?}", self.mode)
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for TableJoinFxn {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![
            self.out.cell().clone(),
            self.lhs.cell().clone(),
            self.rhs.cell().clone(),
        ]
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = self.out.compile_register(context)?;
        let lhs = self.lhs.compile_register(context)?;
        let rhs = self.rhs.compile_register(context)?;
        context.emit_binop(hash_str(&self.to_string()), output, lhs, rhs);
        Ok(output)
    }
}

macro_rules! table_join_factory {
    ($factory:ident, $mode:ident) => {
        #[derive(Debug)]
        struct $factory;

        impl MechFunctionFactory for $factory {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::CanonicalFinalize
            }

            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                FunctionValueRepresentation::Table,
                FunctionValueRepresentation::Table,
                FunctionValueRepresentation::Table,
            );

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                TableJoinFxn::from_invocation(invocation, JoinMode::$mode)
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_TABLE_JOIN_CONTRACT)
            }
        }
    };
}

table_join_factory!(TableInnerJoinFxn, Inner);
table_join_factory!(TableLeftOuterJoinFxn, LeftOuter);
table_join_factory!(TableRightOuterJoinFxn, RightOuter);
table_join_factory!(TableFullOuterJoinFxn, FullOuter);
table_join_factory!(TableLeftSemiJoinFxn, LeftSemi);
table_join_factory!(TableLeftAntiJoinFxn, LeftAnti);

macro_rules! table_join_native_factory {
    ($registration:ident, $installer:ident, $name:literal, $factory:ty) => {
        mech_core::declare_native_runtime_factory! {
            cfg: feature = "table",
            registration: $registration,
            installer: $installer,
            name: $name,
            factory_type: $factory,
            contract: RuntimeFunctionContract::no_matrix(
                RuntimeOutputAliasPolicy::DisallowInputAlias,
            ),
            compiler_family: mech_core::RuntimeFamilyId::from_name($name),
            package: "mech-engine",
            crate_name: "mech_engine",
            installer_path: concat!("mech_engine::__mech_native::", stringify!($installer)),
            extra_cargo_features: [],
        }
    };
}

table_join_native_factory!(
    register_table_inner_join,
    install_table_inner_join,
    "TableJoinFxn::Inner",
    TableInnerJoinFxn
);
table_join_native_factory!(
    register_table_left_outer_join,
    install_table_left_outer_join,
    "TableJoinFxn::LeftOuter",
    TableLeftOuterJoinFxn
);
table_join_native_factory!(
    register_table_right_outer_join,
    install_table_right_outer_join,
    "TableJoinFxn::RightOuter",
    TableRightOuterJoinFxn
);
table_join_native_factory!(
    register_table_full_outer_join,
    install_table_full_outer_join,
    "TableJoinFxn::FullOuter",
    TableFullOuterJoinFxn
);
table_join_native_factory!(
    register_table_left_semi_join,
    install_table_left_semi_join,
    "TableJoinFxn::LeftSemi",
    TableLeftSemiJoinFxn
);
table_join_native_factory!(
    register_table_left_anti_join,
    install_table_left_anti_join,
    "TableJoinFxn::LeftAnti",
    TableLeftAntiJoinFxn
);

pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register_table_inner_join(builder)?;
    register_table_left_outer_join(builder)?;
    register_table_right_outer_join(builder)?;
    register_table_full_outer_join(builder)?;
    register_table_left_semi_join(builder)?;
    register_table_left_anti_join(builder)?;
    Ok(())
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    pub use super::{
        install_table_full_outer_join, install_table_inner_join, install_table_left_anti_join,
        install_table_left_outer_join, install_table_left_semi_join,
        install_table_right_outer_join,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str, schema: SchemaBody) -> SchemaField {
        SchemaField {
            name: name.to_owned(),
            schema,
        }
    }

    #[test]
    fn join_schema_planning_is_shared_and_rejects_incompatible_common_columns() {
        let left = [
            field("id", SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            field("left", SchemaBody::String),
        ];
        let right = [
            field("id", SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            field("right", SchemaBody::Bool),
        ];
        assert_eq!(
            joined_table_fields(&left, &right, JoinMode::Inner).unwrap(),
            vec![left[0].clone(), left[1].clone(), right[1].clone()].into_boxed_slice(),
        );
        assert_eq!(
            joined_table_fields(&left, &right, JoinMode::FullOuter).unwrap(),
            vec![
                left[0].clone(),
                field("left", SchemaBody::Option(Box::new(SchemaBody::String))),
                field("right", SchemaBody::Option(Box::new(SchemaBody::Bool))),
            ]
            .into_boxed_slice(),
        );

        let incompatible = [field("id", SchemaBody::String)];
        assert!(joined_table_fields(&left, &incompatible, JoinMode::Inner).is_err());
    }

    #[test]
    fn join_row_matching_uses_schema_directed_language_equality() {
        let table = |bits| {
            ValueCell::from_schema_data(
                SchemaBody::Table {
                    columns: vec![field("id", SchemaBody::FloatingPoint(FloatWidth::W64))]
                        .into_boxed_slice(),
                    rows: CardinalitySpec::Dynamic { upper_bound: None },
                },
                ValueDataDraft::Table(
                    vec![TableColumnDraft {
                        name: "id".to_owned(),
                        values: vec![ValueDataDraft::F64(
                            mech_core::snapshot::F64Bits::from_bits(bits),
                        )]
                        .into_boxed_slice(),
                    }]
                    .into_boxed_slice(),
                ),
            )
            .unwrap()
        };
        let negative_zero = CanonicalTable::from_cell(&table((-0.0_f64).to_bits())).unwrap();
        let positive_zero = CanonicalTable::from_cell(&table(0.0_f64.to_bits())).unwrap();
        assert!(rows_match(&negative_zero, 0, &positive_zero, 0, &[(0, 0)]));

        let nan = CanonicalTable::from_cell(&table(f64::NAN.to_bits())).unwrap();
        assert!(!rows_match(&nan, 0, &nan, 0, &[(0, 0)]));
    }

    #[test]
    fn reactive_table_join_publishes_through_its_managed_candidate() {
        let table = |id| {
            ValueCell::from_schema_data(
                SchemaBody::Table {
                    columns: vec![field("id", SchemaBody::UnsignedInteger(IntegerWidth::W64))]
                        .into_boxed_slice(),
                    rows: CardinalitySpec::Dynamic { upper_bound: None },
                },
                ValueDataDraft::Table(
                    vec![TableColumnDraft {
                        name: "id".to_owned(),
                        values: vec![ValueDataDraft::U64(id)].into_boxed_slice(),
                    }]
                    .into_boxed_slice(),
                ),
            )
            .unwrap()
        };
        let lhs = table(1);
        let rhs = table(2);
        let output = joined_table(&lhs, &rhs, JoinMode::Inner).unwrap();
        let invocation = FunctionInvocation::binary(output.clone(), lhs.clone(), rhs.clone());
        let specialized = SpecializedFunction::syntax_directed(
            (
                TableJoinFxn::from_invocation(invocation.clone(), JoinMode::Inner).unwrap(),
                invocation,
            ),
            ResolvedOperationDescriptor::from_name(
                "table/inner-join",
                PURE_TABLE_JOIN_CONTRACT.clone(),
            )
            .unwrap(),
            RuntimeFunctionId::from_name("TableJoinInner"),
            ExecutionTarget::DirectRuntime,
            ImplementationMemoryClass::CanonicalFinalize,
        )
        .unwrap();

        rhs.replace(&table(1).snapshot().unwrap()).unwrap();
        specialized.instance().solve_result().unwrap();

        let value = output.snapshot().unwrap();
        let ValueData::Table(table) = value.data() else {
            panic!("join output must remain a table")
        };
        let mech_core::snapshot::SequenceView::U64(values) = table.column(0).unwrap() else {
            panic!("join key column must remain packed u64")
        };
        assert_eq!(values, &[1]);
    }
}

macro_rules! table_join_specializer {
    ($specializer:ident, $mode:ident) => {
        pub struct $specializer;

        impl CanonicalFunctionSpecializer for $specializer {
            fn specialize_invocation(
                &self,
                invocation: &SpecializationInvocation,
                context: &mut SpecializationContext<'_>,
            ) -> MResult<SpecializedFunction> {
                if invocation.len() != 2 {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 2,
                            found: invocation.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let lhs = invocation.input(0).expect("validated lhs").cell()?.clone();
                let rhs = invocation.input(1).expect("validated rhs").cell()?.clone();
                let output = joined_table(&lhs, &rhs, JoinMode::$mode)?;
                let bound = FunctionInvocation::binary(output, lhs, rhs);
                context.certify_instance(
                    (
                        TableJoinFxn::from_invocation(bound.clone(), JoinMode::$mode)?,
                        bound,
                    ),
                    mech_core::RuntimeFunctionId::from_name(concat!(
                        "TableJoin",
                        stringify!($mode)
                    )),
                    mech_core::ExecutionTarget::DirectRuntime,
                    mech_core::ImplementationMemoryClass::CanonicalFinalize,
                )
            }
        }
    };
}

table_join_specializer!(TableInnerJoin, Inner);
table_join_specializer!(TableLeftOuterJoin, LeftOuter);
table_join_specializer!(TableRightOuterJoin, RightOuter);
table_join_specializer!(TableFullOuterJoin, FullOuter);
table_join_specializer!(TableLeftSemiJoin, LeftSemi);
table_join_specializer!(TableLeftAntiJoin, LeftAnti);
