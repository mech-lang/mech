use crate::intrinsics::canonical_access::canonical_draft;
use crate::intrinsics::*;
use mech_core::snapshot::{OptionDraft, TableColumnDraft};

#[derive(Clone, Copy, Debug)]
enum JoinMode {
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
    rows: usize,
}

impl CanonicalTable {
    fn from_cell(cell: &ValueCell) -> MResult<Self> {
        let SchemaBody::Table { columns, .. } = cell.closed_schema_body()? else {
            return Err(table_join_error("input must be a canonical table"));
        };
        let ValueDataDraft::Table(values) = canonical_draft(cell)? else {
            return Err(table_join_error("table input has a non-table payload"));
        };
        let rows = values.first().map_or(0, |column| column.values.len());
        if values.iter().any(|column| column.values.len() != rows) {
            return Err(table_join_error(
                "table columns have inconsistent row counts",
            ));
        }
        Ok(Self {
            fields: columns,
            columns: values,
            rows,
        })
    }

    fn value(&self, column: usize, row: usize) -> &ValueDataDraft {
        &self.columns[column].values[row]
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
    common
        .iter()
        .all(|(left, right)| lhs.value(*left, lhs_row) == rhs.value(*right, rhs_row))
}

fn joined_table(lhs: &ValueCell, rhs: &ValueCell, mode: JoinMode) -> MResult<ValueCell> {
    let lhs = CanonicalTable::from_cell(lhs)?;
    let rhs = CanonicalTable::from_cell(rhs)?;
    let common = lhs
        .fields
        .iter()
        .enumerate()
        .filter_map(|(left, field)| {
            rhs.fields
                .iter()
                .position(|candidate| candidate.name == field.name)
                .map(|right| (left, right))
        })
        .collect::<Vec<_>>();
    let common_rhs = common
        .iter()
        .map(|(_, right)| *right)
        .collect::<std::collections::BTreeSet<_>>();

    let lhs_outer = matches!(mode, JoinMode::RightOuter | JoinMode::FullOuter);
    let rhs_outer = matches!(mode, JoinMode::LeftOuter | JoinMode::FullOuter);
    let lhs_only = matches!(mode, JoinMode::LeftSemi | JoinMode::LeftAnti);
    let mut fields = lhs
        .fields
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
            rhs.fields
                .iter()
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
    ValueCell::from_schema_data(
        SchemaBody::Table {
            columns: fields.into_boxed_slice(),
            rows: CardinalitySpec::Dynamic { upper_bound: None },
        },
        ValueDataDraft::Table(output_columns.into_boxed_slice()),
    )
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
}

impl MechFunctionImpl for TableJoinFxn {
    fn solve_result(&self) -> MResult<()> {
        let joined = joined_table(self.lhs.cell(), self.rhs.cell(), self.mode)?;
        self.out.replace(&joined.snapshot()?)
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        None
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
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                FunctionValueRepresentation::Table,
                FunctionValueRepresentation::Table,
                FunctionValueRepresentation::Table,
            );

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                TableJoinFxn::from_invocation(invocation, JoinMode::$mode)
            }

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                Self::new_invocation(args.into())
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

macro_rules! table_join_specializer {
    ($specializer:ident, $mode:ident) => {
        pub struct $specializer;

        impl CanonicalFunctionSpecializer for $specializer {
            fn specialize_invocation(
                &self,
                invocation: &SpecializationInvocation,
                _: &mut SpecializationContext<'_>,
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
                Ok(SpecializedFunction::new(FunctionInstance::new(
                    TableJoinFxn::from_invocation(bound.clone(), JoinMode::$mode)?,
                    bound,
                )))
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
