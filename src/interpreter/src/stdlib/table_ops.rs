use crate::stdlib::*;
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

        let mut output_cols: Vec<(u64, ValueKind, String)> = vec![];
        for (lhs_id, (kind, _)) in lhs.data.iter() {
            let name = lhs
                .col_names
                .get(lhs_id)
                .cloned()
                .unwrap_or_else(|| lhs_id.to_string());
            output_cols.push((*lhs_id, kind.clone(), name));
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
            output_cols.push((*rhs_id, kind.clone(), name));
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

        let mut out_rows: Vec<HashMap<u64, Value>> = vec![];
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
                                .unwrap_or(Value::Empty);
                            row.insert(*lhs_id, value);
                        } else {
                            row.insert(*lhs_id, Value::Empty);
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
                            .unwrap_or(Value::Empty);
                        row.insert(*rhs_id, value);
                    }
                }

                out_rows.push(row);
            }
        }

        let mut data: IndexMap<u64, (ValueKind, Matrix<Value>)> = IndexMap::new();
        let mut col_names: HashMap<u64, String> = HashMap::new();

        for (col_id, kind, name) in &output_cols {
            let mut values = Vec::with_capacity(out_rows.len());
            for row in &out_rows {
                values.push(row.get(col_id).cloned().unwrap_or(Value::Empty));
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

impl MechFunctionFactory for TableJoinFxn {
    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Binary(out, arg1, arg2) => {
                let lhs: Ref<MechTable> = unsafe { arg1.as_unchecked() }.clone();
                let rhs: Ref<MechTable> = unsafe { arg2.as_unchecked() }.clone();
                let out: Ref<MechTable> = unsafe { out.as_unchecked() }.clone();
                Ok(Box::new(TableJoinFxn {
                    lhs,
                    rhs,
                    out,
                    mode: JoinMode::Inner,
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
}

impl MechFunctionImpl for TableJoinFxn {
    fn solve(&self) {
        unsafe {
            let lhs = &*self.lhs.as_ptr();
            let rhs = &*self.rhs.as_ptr();
            if let Ok(joined) = Self::build_joined_table(lhs, rhs, self.mode) {
                *self.out.as_mut_ptr() = joined;
            }
        }
    }

    fn out(&self) -> Value {
        Value::Table(self.out.clone())
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for TableJoinFxn {
    fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("TableJoinFxn::{:?}", self.mode);
        compile_binop!(
            name,
            self.out,
            self.lhs,
            self.rhs,
            ctx,
            FeatureFlag::Builtin(FeatureKind::Functions)
        );
    }
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
) -> HashMap<u64, Value> {
    let mut row = HashMap::new();
    for (lhs_id, _) in lhs.data.iter() {
        let value = lhs
            .data
            .get(lhs_id)
            .map(|(_, col)| col.index1d(lhs_row))
            .unwrap_or(Value::Empty);
        row.insert(*lhs_id, value);
    }
    for (rhs_id, _) in rhs.data.iter() {
        if common_rhs.contains(rhs_id) {
            continue;
        }
        let value = if rhs_empty || rhs_row == 0 {
            Value::Empty
        } else {
            rhs.data
                .get(rhs_id)
                .map(|(_, col)| col.index1d(rhs_row))
                .unwrap_or(Value::Empty)
        };
        row.insert(*rhs_id, value);
    }
    row
}

fn lhs_only_row(lhs: &MechTable, lhs_row: usize) -> HashMap<u64, Value> {
    lhs.data
        .iter()
        .map(|(lhs_id, _)| {
            let value = lhs
                .data
                .get(lhs_id)
                .map(|(_, col)| col.index1d(lhs_row))
                .unwrap_or(Value::Empty);
            (*lhs_id, value)
        })
        .collect()
}

fn compile_table_join(arguments: &Vec<Value>, mode: JoinMode) -> MResult<Box<dyn MechFunction>> {
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

    let resolve = |v: &Value| -> Option<Ref<MechTable>> {
        match v {
            Value::Table(t) => Some(t.clone()),
            Value::MutableReference(r) => match &*r.borrow() {
                Value::Table(t) => Some(t.clone()),
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
impl NativeFunctionCompiler for TableInnerJoin {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        compile_table_join(arguments, JoinMode::Inner)
    }
}

pub struct TableLeftOuterJoin {}
impl NativeFunctionCompiler for TableLeftOuterJoin {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        compile_table_join(arguments, JoinMode::LeftOuter)
    }
}

pub struct TableRightOuterJoin {}
impl NativeFunctionCompiler for TableRightOuterJoin {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        compile_table_join(arguments, JoinMode::RightOuter)
    }
}

pub struct TableFullOuterJoin {}
impl NativeFunctionCompiler for TableFullOuterJoin {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        compile_table_join(arguments, JoinMode::FullOuter)
    }
}

pub struct TableLeftSemiJoin {}
impl NativeFunctionCompiler for TableLeftSemiJoin {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        compile_table_join(arguments, JoinMode::LeftSemi)
    }
}

pub struct TableLeftAntiJoin {}
impl NativeFunctionCompiler for TableLeftAntiJoin {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        compile_table_join(arguments, JoinMode::LeftAnti)
    }
}
