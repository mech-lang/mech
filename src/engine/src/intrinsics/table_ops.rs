use crate::intrinsics::*;
use indexmap::map::IndexMap;
use mech_core::matrix::Matrix;
use na::DVector;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy)]
enum JoinMode {
    Inner,
    LeftOuter,
    RightOuter,
    FullOuter,
    LeftSemi,
    LeftAnti,
}

#[derive(Debug)]
struct TableJoinFxn {
    lhs: Ref<MechTable>,
    rhs: Ref<MechTable>,
    out: Ref<MechTable>,
    mode: JoinMode,
}

impl TableJoinFxn {
    fn build_joined_table(lhs: &MechTable, rhs: &MechTable, mode: JoinMode) -> MResult<MechTable> {
        let rhs_name_to_id: HashMap<String, u64> = rhs
            .col_names
            .iter()
            .map(|(id, name)| (name.clone(), *id))
            .collect();

        let mut common_cols: Vec<(u64, u64)> = vec![];
        for (lhs_id, lhs_name) in &lhs.col_names {
            if let Some(rhs_id) = rhs_name_to_id.get(lhs_name) {
                common_cols.push((*lhs_id, *rhs_id));
            }
        }

        let common_rhs: HashSet<u64> = common_cols.iter().map(|(_, rhs_id)| *rhs_id).collect();
        let common_lhs: HashSet<u64> = common_cols.iter().map(|(lhs_id, _)| *lhs_id).collect();

        let mut output_cols: Vec<(u64, ValueKind, String)> = vec![];
        for (lhs_id, (kind, _)) in lhs.data.iter() {
            let name = lhs
                .col_names
                .get(lhs_id)
                .cloned()
                .unwrap_or_else(|| lhs_id.to_string());
            let out_kind = if !common_lhs.contains(lhs_id)
                && matches!(mode, JoinMode::RightOuter | JoinMode::FullOuter)
            {
                make_optional_kind(kind)
            } else {
                kind.clone()
            };
            output_cols.push((*lhs_id, out_kind, name));
        }
        for (rhs_id, (kind, _)) in rhs.data.iter() {
            if common_rhs.contains(rhs_id) {
                continue;
            }
            let name = rhs
                .col_names
                .get(rhs_id)
                .cloned()
                .unwrap_or_else(|| rhs_id.to_string());
            let out_kind = if matches!(mode, JoinMode::LeftOuter | JoinMode::FullOuter) {
                make_optional_kind(kind)
            } else {
                kind.clone()
            };
            output_cols.push((*rhs_id, out_kind, name));
        }

        if matches!(mode, JoinMode::LeftSemi | JoinMode::LeftAnti) {
            output_cols = lhs
                .data
                .iter()
                .map(|(lhs_id, (kind, _))| {
                    let name = lhs
                        .col_names
                        .get(lhs_id)
                        .cloned()
                        .unwrap_or_else(|| lhs_id.to_string());
                    (*lhs_id, kind.clone(), name)
                })
                .collect();
        }

        let mut out_rows: Vec<HashMap<u64, LegacyValue>> = vec![];
        let mut rhs_matched: Vec<bool> = vec![false; rhs.rows];

        for lhs_row in 1..=lhs.rows {
            let mut matched_rhs: Vec<usize> = vec![];
            for rhs_row in 1..=rhs.rows {
                if rows_match(lhs, lhs_row, rhs, rhs_row, &common_cols) {
                    matched_rhs.push(rhs_row);
                }
            }

            match mode {
                JoinMode::Inner => {
                    for rhs_row in matched_rhs {
                        rhs_matched[rhs_row - 1] = true;
                        out_rows.push(merge_rows(lhs, lhs_row, rhs, rhs_row, &common_rhs, false));
                    }
                }
                JoinMode::LeftOuter => {
                    if matched_rhs.is_empty() {
                        out_rows.push(merge_rows(lhs, lhs_row, rhs, 0, &common_rhs, true));
                    } else {
                        for rhs_row in matched_rhs {
                            rhs_matched[rhs_row - 1] = true;
                            out_rows.push(merge_rows(
                                lhs,
                                lhs_row,
                                rhs,
                                rhs_row,
                                &common_rhs,
                                false,
                            ));
                        }
                    }
                }
                JoinMode::RightOuter => {
                    if matched_rhs.is_empty() {
                        // handled when iterating unmatched rhs rows below
                    } else {
                        for rhs_row in matched_rhs {
                            rhs_matched[rhs_row - 1] = true;
                            out_rows.push(merge_rows(
                                lhs,
                                lhs_row,
                                rhs,
                                rhs_row,
                                &common_rhs,
                                false,
                            ));
                        }
                    }
                }
                JoinMode::FullOuter => {
                    if matched_rhs.is_empty() {
                        out_rows.push(merge_rows(lhs, lhs_row, rhs, 0, &common_rhs, true));
                    } else {
                        for rhs_row in matched_rhs {
                            rhs_matched[rhs_row - 1] = true;
                            out_rows.push(merge_rows(
                                lhs,
                                lhs_row,
                                rhs,
                                rhs_row,
                                &common_rhs,
                                false,
                            ));
                        }
                    }
                }
                JoinMode::LeftSemi => {
                    if !matched_rhs.is_empty() {
                        out_rows.push(lhs_only_row(lhs, lhs_row));
                    }
                }
                JoinMode::LeftAnti => {
                    if matched_rhs.is_empty() {
                        out_rows.push(lhs_only_row(lhs, lhs_row));
                    }
                }
            }
        }

        if matches!(mode, JoinMode::RightOuter | JoinMode::FullOuter) {
            for rhs_row in 1..=rhs.rows {
                if rhs_matched[rhs_row - 1] {
                    continue;
                }
                let mut row = HashMap::new();

                if !matches!(mode, JoinMode::LeftSemi | JoinMode::LeftAnti) {
                    for (lhs_id, _) in lhs.data.iter() {
                        if let Some((_, rhs_id)) = common_cols.iter().find(|(l, _)| l == lhs_id) {
                            let value = rhs
                                .data
                                .get(rhs_id)
                                .map(|(_, col)| col.index1d(rhs_row))
                                .unwrap_or(LegacyValue::Empty);
                            row.insert(*lhs_id, value);
                        } else {
                            row.insert(*lhs_id, LegacyValue::Empty);
                        }
                    }
                    for (rhs_id, _) in rhs.data.iter() {
                        if common_rhs.contains(rhs_id) {
                            continue;
                        }
                        let value = rhs
                            .data
                            .get(rhs_id)
                            .map(|(_, col)| col.index1d(rhs_row))
                            .unwrap_or(LegacyValue::Empty);
                        row.insert(*rhs_id, value);
                    }
                }

                out_rows.push(row);
            }
        }

        let mut data: IndexMap<u64, (ValueKind, Matrix<LegacyValue>)> = IndexMap::new();
        let mut col_names: HashMap<u64, String> = HashMap::new();

        for (col_id, kind, name) in &output_cols {
            let mut values = Vec::with_capacity(out_rows.len());
            for row in &out_rows {
                values.push(row.get(col_id).cloned().unwrap_or(LegacyValue::Empty));
            }
            data.insert(
                *col_id,
                (
                    kind.clone(),
                    Matrix::DVector(Ref::new(DVector::from_vec(values))),
                ),
            );
            col_names.insert(*col_id, name.clone());
        }

        Ok(MechTable {
            rows: out_rows.len(),
            cols: output_cols.len(),
            data,
            col_names,
        })
    }
}

fn make_optional_kind(kind: &ValueKind) -> ValueKind {
    match kind {
        ValueKind::Option(_) => kind.clone(),
        _ => ValueKind::Option(Box::new(kind.clone())),
    }
}

impl MechFunctionFactory for TableJoinFxn {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Table,
        FunctionValueRepresentation::Table,
        FunctionValueRepresentation::Table,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        table_join_from_args(args, JoinMode::Inner)
    }
}

impl MechFunctionImpl for TableJoinFxn {
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let lhs = &*self.lhs.as_ptr();
            let rhs = &*self.rhs.as_ptr();
            if let Ok(joined) = Self::build_joined_table(lhs, rhs, self.mode) {
                *self.out.as_mut_ptr() = joined;
            }
        };
        Ok(())
    }

    fn out(&self) -> LegacyValue {
        LegacyValue::Table(self.out.clone())
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for TableJoinFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("TableJoinFxn::{:?}", self.mode);
        compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
    }
}

fn table_join_from_args(args: FunctionArgs, mode: JoinMode) -> MResult<Box<dyn MechFunction>> {
    match args {
        FunctionArgs::Binary(out, arg1, arg2) => {
            let lhs: Ref<MechTable> = arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
            let rhs: Ref<MechTable> = arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
            let out: Ref<MechTable> = out.try_function_ref(FunctionArgumentRole::Output)?;
            Ok(Box::new(TableJoinFxn {
                lhs,
                rhs,
                out,
                mode,
            }))
        }
        _ => Err(MechError::new(
            IncorrectNumberOfArguments {
                expected: 2,
                found: args.len(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

macro_rules! table_join_factory {
    ($factory:ident, $mode:ident) => {
        #[derive(Debug)]
        struct $factory;

        impl MechFunctionFactory for $factory {
            const SIGNATURE: RuntimeFunctionSignature = TableJoinFxn::SIGNATURE;

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                table_join_from_args(args, JoinMode::$mode)
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
            installer_path: concat!(
                "mech_engine::__mech_native::",
                stringify!($installer),
            ),
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

fn rows_match(
    lhs: &MechTable,
    lhs_row: usize,
    rhs: &MechTable,
    rhs_row: usize,
    common_cols: &[(u64, u64)],
) -> bool {
    common_cols.iter().all(|(lhs_col, rhs_col)| {
        let lhs_val = lhs.data.get(lhs_col).map(|(_, col)| col.index1d(lhs_row));
        let rhs_val = rhs.data.get(rhs_col).map(|(_, col)| col.index1d(rhs_row));
        lhs_val == rhs_val
    })
}

fn merge_rows(
    lhs: &MechTable,
    lhs_row: usize,
    rhs: &MechTable,
    rhs_row: usize,
    common_rhs: &HashSet<u64>,
    rhs_empty: bool,
) -> HashMap<u64, LegacyValue> {
    let mut row = HashMap::new();
    for (lhs_id, _) in lhs.data.iter() {
        let value = lhs
            .data
            .get(lhs_id)
            .map(|(_, col)| col.index1d(lhs_row))
            .unwrap_or(LegacyValue::Empty);
        row.insert(*lhs_id, value);
    }
    for (rhs_id, _) in rhs.data.iter() {
        if common_rhs.contains(rhs_id) {
            continue;
        }
        let value = if rhs_empty || rhs_row == 0 {
            LegacyValue::Empty
        } else {
            rhs.data
                .get(rhs_id)
                .map(|(_, col)| col.index1d(rhs_row))
                .unwrap_or(LegacyValue::Empty)
        };
        row.insert(*rhs_id, value);
    }
    row
}

fn lhs_only_row(lhs: &MechTable, lhs_row: usize) -> HashMap<u64, LegacyValue> {
    lhs.data
        .iter()
        .map(|(lhs_id, _)| {
            let value = lhs
                .data
                .get(lhs_id)
                .map(|(_, col)| col.index1d(lhs_row))
                .unwrap_or(LegacyValue::Empty);
            (*lhs_id, value)
        })
        .collect()
}

fn compile_table_join(arguments: &[LegacyValue], mode: JoinMode) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
        return Err(MechError::new(
            IncorrectNumberOfArguments {
                expected: 2,
                found: arguments.len(),
            },
            None,
        )
        .with_compiler_loc());
    }

    let resolve = |v: &LegacyValue| -> Option<Ref<MechTable>> {
        match v {
            LegacyValue::Table(t) => Some(t.clone()),
            LegacyValue::MutableReference(r) => match &*r.borrow() {
                LegacyValue::Table(t) => Some(t.clone()),
                _ => None,
            },
            _ => None,
        }
    };

    let lhs = resolve(&arguments[0]);
    let rhs = resolve(&arguments[1]);

    match (lhs, rhs) {
        (Some(lhs), Some(rhs)) => {
            let out = Ref::new(TableJoinFxn::build_joined_table(
                &lhs.borrow(),
                &rhs.borrow(),
                mode,
            )?);
            Ok(Box::new(TableJoinFxn {
                lhs,
                rhs,
                out,
                mode,
            }))
        }
        _ => Err(MechError::new(
            UnhandledFunctionArgumentKind2 {
                arg: (arguments[0].kind(), arguments[1].kind()),
                fxn_name: "table/join".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

pub struct TableInnerJoin {}
impl FunctionSpecializer for TableInnerJoin {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        compile_table_join(arguments, JoinMode::Inner)
    }
}

pub struct TableLeftOuterJoin {}
impl FunctionSpecializer for TableLeftOuterJoin {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        compile_table_join(arguments, JoinMode::LeftOuter)
    }
}

pub struct TableRightOuterJoin {}
impl FunctionSpecializer for TableRightOuterJoin {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        compile_table_join(arguments, JoinMode::RightOuter)
    }
}

pub struct TableFullOuterJoin {}
impl FunctionSpecializer for TableFullOuterJoin {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        compile_table_join(arguments, JoinMode::FullOuter)
    }
}

pub struct TableLeftSemiJoin {}
impl FunctionSpecializer for TableLeftSemiJoin {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        compile_table_join(arguments, JoinMode::LeftSemi)
    }
}

pub struct TableLeftAntiJoin {}
impl FunctionSpecializer for TableLeftAntiJoin {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        compile_table_join(arguments, JoinMode::LeftAnti)
    }
}
