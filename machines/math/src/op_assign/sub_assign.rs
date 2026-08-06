#[macro_use]
use crate::*;
use super::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use num_traits::*;

// Sub Assign -----------------------------------------------------------------

#[cfg(feature = "source")]
#[macro_export]
macro_rules! impl_sub_assign_match_arms {
    ($fxn_name:ident,$macro_name:ident, $arg:expr) => {
        paste! {
          [<impl_set_ $macro_name _match_arms>]!(
            $fxn_name,
            $arg,
            U8, "u8";
            U16, "u16";
            U32, "u32";
            U64, "u64";
            U128, "u128";
            I8, "i8";
            I16, "i16";
            I32, "i32";
            I64, "i64";
            U128, "u128";
            F32, "f32";
            F64, "f64" ;
            C64, "complex";
            R64, "rational";
          )
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! impl_sub_assign_range_fxn_s {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        impl_op_assign_range_fxn_s!($struct_name, $op, $ix);
    };
}

#[cfg(feature = "matrix")]
macro_rules! impl_sub_assign_range_fxn_v {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        impl_op_assign_range_fxn_v!($struct_name, $op, $ix);
    };
}

// x = 1 ----------------------------------------------------------------------

impl_assign_scalar_scalar!(Sub, checked_sub_assign);
impl_assign_vector_vector!(Sub, checked_sub_assign);
impl_assign_vector_scalar!(Sub, checked_sub_assign);

#[cfg(feature = "source")]
fn sub_assign_value_fxn(sink: Value, source: Value) -> MResult<Box<dyn MechFunction>> {
    impl_op_assign_value_match_arms!(
      Sub,
      (sink, source),
      U8,  "u8";
      U16, "u16";
      U32, "u32";
      U64, "u64";
      U128, "u128";
      I8,  "i8";
      I16, "i16";
      I32, "i32";
      I64, "i64";
      U128, "u128";
      F32, "f32";
      F64, "f64";
      R64, "rational";
      C64, "complex";
    )
}

#[cfg(feature = "source")]
pub struct SubAssignValue {}
#[cfg(feature = "source")]
impl FunctionSpecializer for SubAssignValue {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let sink = arguments[0].clone();
        let source = arguments[1].clone();
        match sub_assign_value_fxn(sink.clone(), source.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (sink, source) {
                (Value::MutableReference(sink), Value::MutableReference(source)) => {
                    sub_assign_value_fxn(sink.borrow().clone(), source.borrow().clone())
                }
                (sink, Value::MutableReference(source)) => {
                    sub_assign_value_fxn(sink.clone(), source.borrow().clone())
                }
                (Value::MutableReference(sink), source) => {
                    sub_assign_value_fxn(sink.borrow().clone(), source.clone())
                }
                (arg1, arg2) => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (arg1.kind(), arg2.kind()),
                        fxn_name: "math/sub-assign".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}

// x[1..3] -= 1 ----------------------------------------------------------------

macro_rules! sub_assign_1d_range {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            for &index in ($ix).iter() {
                let offset = checked_one_based_index(index, ($sink).len())?;
                ($sink)[offset] = checked_sub_assign(($sink)[offset], *($source))?;
            }
            Ok::<(), MechError>(())
        }
    };
}

macro_rules! sub_assign_1d_range_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            validate_mask_len(($ix).len(), ($sink).len())?;
            for (i, selected) in ($ix).iter().copied().enumerate() {
                if selected {
                    ($sink)[i] = checked_sub_assign(($sink)[i], *($source))?;
                }
            }
            Ok::<(), MechError>(())
        }
    };
}

macro_rules! sub_assign_1d_range_vec {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            validate_source_len(($source).len(), ($ix).len())?;
            for (i, &index) in ($ix).iter().enumerate() {
                let offset = checked_one_based_index(index, ($sink).len())?;
                ($sink)[offset] = checked_sub_assign(($sink)[offset], ($source)[i])?;
            }
            Ok::<(), MechError>(())
        }
    };
}

macro_rules! sub_assign_1d_range_vec_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            validate_mask_len(($ix).len(), ($sink).len())?;
            validate_source_len(($source).len(), ($ix).len())?;
            for (i, selected) in ($ix).iter().copied().enumerate() {
                if selected {
                    ($sink)[i] = checked_sub_assign(($sink)[i], ($source)[i])?;
                }
            }
            Ok::<(), MechError>(())
        }
    };
}

#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_s!(SubAssign1DRS, sub_assign_1d_range, usize);
#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_s!(SubAssign1DRB, sub_assign_1d_range_b, bool);
#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_v!(SubAssign1DRV, sub_assign_1d_range_vec, usize);
#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_v!(SubAssign1DRVB, sub_assign_1d_range_vec_b, bool);

#[cfg(feature = "source")]
op_assign_range_fxn!(sub_assign_range_fxn, SubAssign1DR);

#[cfg(feature = "source")]
pub struct SubAssignRange {}
#[cfg(feature = "source")]
impl FunctionSpecializer for SubAssignRange {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let sink: Value = arguments[0].clone();
        let source: Value = arguments[1].clone();
        let ixes = arguments[2..].to_vec();
        match sub_assign_range_fxn(sink.clone(), source.clone(), ixes.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(x) => match (&sink, &ixes, &source) {
                (Value::MutableReference(sink), ixes, Value::MutableReference(source)) => {
                    sub_assign_range_fxn(
                        sink.borrow().clone(),
                        source.borrow().clone(),
                        ixes.clone(),
                    )
                }
                (sink, ixes, Value::MutableReference(source)) => {
                    sub_assign_range_fxn(sink.clone(), source.borrow().clone(), ixes.clone())
                }
                (Value::MutableReference(sink), ixes, source) => {
                    sub_assign_range_fxn(sink.borrow().clone(), source.clone(), ixes.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentIxes {
                        arg: (
                            sink.kind(),
                            ixes.iter().map(|v| v.kind()).collect::<Vec<_>>(),
                            source.kind(),
                        ),
                        fxn_name: "math/sub-assign/range".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}

// x[1..3,:] -= 1 ------------------------------------------------------------------

macro_rules! sub_assign_2d_vector_all {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            for &index in ($ix).iter() {
                checked_one_based_index(index, ($sink).nrows())?;
            }
            for cix in 0..($sink).ncols() {
                for &index in ($ix).iter() {
                    let row = index - 1;
                    let value = ($sink).column(cix)[row];
                    ($sink).column_mut(cix)[row] = checked_sub_assign(value, *($source))?;
                }
            }
            Ok::<(), MechError>(())
        }
    };
}

macro_rules! sub_assign_2d_vector_all_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            validate_mask_len(($ix).len(), ($sink).nrows())?;
            for cix in 0..($sink).ncols() {
                for (row, selected) in ($ix).iter().copied().enumerate() {
                    if selected {
                        let value = ($sink).column(cix)[row];
                        ($sink).column_mut(cix)[row] = checked_sub_assign(value, *($source))?;
                    }
                }
            }
            Ok::<(), MechError>(())
        }
    };
}

macro_rules! sub_assign_2d_vector_all_mat {
    ($source:expr, $ix:expr, $sink:expr) => {{
        let nsrc = $source.nrows();
        validate_source_len(nsrc, if ($ix).is_empty() { 0 } else { 1 })?;
        for (i, &rix) in $ix.iter().enumerate() {
            let row_index = checked_one_based_index(rix, ($sink).nrows())?;
            let mut sink_row = $sink.row_mut(row_index);
            let src_row = $source.row(i % nsrc); // wrap around!
            for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
                *dst = checked_sub_assign(*dst, *src)?;
            }
        }
        Ok::<(), MechError>(())
    }};
}

macro_rules! sub_assign_2d_vector_all_mat_b {
    ($source:expr, $ix:expr, $sink:expr) => {{
        validate_mask_len(($ix).len(), ($sink).nrows())?;
        validate_source_len(($source).nrows(), ($ix).iter().filter(|selected| **selected).count())?;
        let mut src_i = 0;
        for (i, rix) in (&$ix).iter().enumerate() {
            if *rix == true {
                let mut sink_row = ($sink).row_mut(i);
                let src_row = ($source).row(src_i);
                for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
                    *dst = checked_sub_assign(*dst, *src)?;
                }
                src_i += 1;
            }
        }
        Ok::<(), MechError>(())
    }};
}

#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_s!(SubAssign2DRAS, sub_assign_2d_vector_all, usize);
#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_s!(SubAssign2DRASB, sub_assign_2d_vector_all_b, bool);
#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_v!(SubAssign2DRAV, sub_assign_2d_vector_all_mat, usize);
#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_v!(SubAssign2DRAVB, sub_assign_2d_vector_all_mat_b, bool);

#[cfg(feature = "source")]
op_assign_range_all_fxn!(sub_assign_range_all_fxn, SubAssign2DRA);

#[cfg(feature = "source")]
pub struct SubAssignRangeAll {}
#[cfg(feature = "source")]
impl FunctionSpecializer for SubAssignRangeAll {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let sink: Value = arguments[0].clone();
        let source: Value = arguments[1].clone();
        let ixes = arguments[2..].to_vec();
        match sub_assign_range_all_fxn(sink.clone(), source.clone(), ixes.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (&sink, &ixes, &source) {
                (Value::MutableReference(sink), ixes, Value::MutableReference(source)) => {
                    sub_assign_range_all_fxn(
                        sink.borrow().clone(),
                        source.borrow().clone(),
                        ixes.clone(),
                    )
                }
                (sink, ixes, Value::MutableReference(source)) => {
                    sub_assign_range_all_fxn(sink.clone(), source.borrow().clone(), ixes.clone())
                }
                (Value::MutableReference(sink), ixes, source) => {
                    sub_assign_range_all_fxn(sink.borrow().clone(), source.clone(), ixes.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentIxes {
                        arg: (
                            sink.kind(),
                            ixes.iter().map(|v| v.kind()).collect::<Vec<_>>(),
                            source.kind(),
                        ),
                        fxn_name: "math/sub-assign/range-all".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
