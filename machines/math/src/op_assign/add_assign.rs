use super::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use num_traits::*;

// Add Assign -----------------------------------------------------------------

#[cfg(feature = "source")]
#[macro_export]
macro_rules! impl_add_assign_match_arms {
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
macro_rules! impl_add_assign_range_fxn_s {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        impl_op_assign_range_fxn_s!($struct_name, $op, $ix);
    };
}

#[cfg(feature = "matrix")]
macro_rules! impl_add_assign_range_fxn_v {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        impl_op_assign_range_fxn_v!($struct_name, $op, $ix);
    };
}

// x += 1 ----------------------------------------------------------------------

impl_assign_scalar_scalar!(Add, checked_add_assign);
impl_assign_vector_vector!(Add, checked_add_assign);
impl_assign_vector_scalar!(Add, checked_add_assign);

#[cfg(feature = "source")]
pub fn add_assign_math_fxn(sink: LegacyValue, source: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_op_assign_value_match_arms!(
      Add,
      (sink, source),
      U8,  "u8";
      U16, "u16";
      U32, "u32";
      U64, "u64";
      I128, "i128";
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
pub struct AddAssignMath {}
#[cfg(feature = "source")]
impl FunctionSpecializer for AddAssignMath {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
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
        match add_assign_math_fxn(sink.clone(), source.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (sink, source) {
                (LegacyValue::MutableReference(sink), LegacyValue::MutableReference(source)) => {
                    add_assign_math_fxn(sink.borrow().clone(), source.borrow().clone())
                }
                (sink, LegacyValue::MutableReference(source)) => {
                    add_assign_math_fxn(sink.clone(), source.borrow().clone())
                }
                (LegacyValue::MutableReference(sink), source) => {
                    add_assign_math_fxn(sink.borrow().clone(), source.clone())
                }
                (arg1, arg2) => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (arg1.kind(), arg2.kind()),
                        fxn_name: "math/add-assign".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}

// x[1..3] += 1 ----------------------------------------------------------------

macro_rules! add_assign_1d_range {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            for &index in ($ix).iter() {
                let offset = checked_one_based_index(index, ($sink).len())?;
                ($sink)[offset] = checked_add_assign(($sink)[offset], *($source))?;
            }
            Ok::<(), MechError>(())
        }
    };
}

macro_rules! add_assign_1d_range_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            validate_mask_len(($ix).len(), ($sink).len())?;
            for (i, selected) in ($ix).iter().copied().enumerate() {
                if selected {
                    ($sink)[i] = checked_add_assign(($sink)[i], *($source))?;
                }
            }
            Ok::<(), MechError>(())
        }
    };
}

macro_rules! add_assign_1d_range_vec {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            validate_source_len(($source).len(), ($ix).len())?;
            for (i, &index) in ($ix).iter().enumerate() {
                let offset = checked_one_based_index(index, ($sink).len())?;
                ($sink)[offset] = checked_add_assign(($sink)[offset], ($source)[i])?;
            }
            Ok::<(), MechError>(())
        }
    };
}

macro_rules! add_assign_1d_range_vec_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            validate_mask_len(($ix).len(), ($sink).len())?;
            validate_source_len(($source).len(), ($ix).len())?;
            for (i, selected) in ($ix).iter().copied().enumerate() {
                if selected {
                    ($sink)[i] = checked_add_assign(($sink)[i], ($source)[i])?;
                }
            }
            Ok::<(), MechError>(())
        }
    };
}

#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_s!(AddAssign1DRS, add_assign_1d_range, usize);
#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_s!(AddAssign1DRB, add_assign_1d_range_b, bool);
#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_v!(AddAssign1DRV, add_assign_1d_range_vec, usize);
#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_v!(AddAssign1DRVB, add_assign_1d_range_vec_b, bool);

#[cfg(feature = "source")]
op_assign_range_fxn!(add_assign_range_fxn, AddAssign1DR);

#[cfg(feature = "source")]
pub struct AddAssignRange {}
#[cfg(feature = "source")]
impl FunctionSpecializer for AddAssignRange {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
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
        let sink: LegacyValue = arguments[0].clone();
        let source: LegacyValue = arguments[1].clone();
        let ixes = arguments[2..].to_vec();
        match add_assign_range_fxn(sink.clone(), source.clone(), ixes.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (&sink, &ixes, &source) {
                (LegacyValue::MutableReference(sink), ixes, LegacyValue::MutableReference(source)) => {
                    add_assign_range_fxn(
                        sink.borrow().clone(),
                        source.borrow().clone(),
                        ixes.clone(),
                    )
                }
                (sink, ixes, LegacyValue::MutableReference(source)) => {
                    add_assign_range_fxn(sink.clone(), source.borrow().clone(), ixes.clone())
                }
                (LegacyValue::MutableReference(sink), ixes, source) => {
                    add_assign_range_fxn(sink.borrow().clone(), source.clone(), ixes.clone())
                }
                (sink, ixes, source) => Err(MechError::new(
                    UnhandledFunctionArgumentIxes {
                        arg: (
                            sink.kind(),
                            ixes.iter().map(|x| x.kind()).collect(),
                            source.kind(),
                        ),
                        fxn_name: "math/add-assign/range".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}

// x[1..3,:] += 1 ------------------------------------------------------------------

macro_rules! add_assign_2d_vector_all {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            for &index in ($ix).iter() {
                checked_one_based_index(index, ($sink).nrows())?;
            }
            for cix in 0..($sink).ncols() {
                for &index in ($ix).iter() {
                    let row = index - 1;
                    let value = ($sink).column(cix)[row];
                    ($sink).column_mut(cix)[row] = checked_add_assign(value, *($source))?;
                }
            }
            Ok::<(), MechError>(())
        }
    };
}

macro_rules! add_assign_2d_vector_all_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            validate_mask_len(($ix).len(), ($sink).nrows())?;
            for cix in 0..($sink).ncols() {
                for (row, selected) in ($ix).iter().copied().enumerate() {
                    if selected {
                        let value = ($sink).column(cix)[row];
                        ($sink).column_mut(cix)[row] = checked_add_assign(value, *($source))?;
                    }
                }
            }
            Ok::<(), MechError>(())
        }
    };
}

macro_rules! add_assign_2d_vector_all_mat {
    ($source:expr, $ix:expr, $sink:expr) => {{
        let nsrc = $source.nrows();
        validate_source_len(nsrc, if ($ix).is_empty() { 0 } else { 1 })?;
        for (i, &rix) in $ix.iter().enumerate() {
            let row_index = checked_one_based_index(rix, ($sink).nrows())?;
            let mut sink_row = $sink.row_mut(row_index);
            let src_row = $source.row(i % nsrc); // wrap around!
            for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
                *dst = checked_add_assign(*dst, *src)?;
            }
        }
        Ok::<(), MechError>(())
    }};
}

macro_rules! add_assign_2d_vector_all_mat_b {
    ($source:expr, $ix:expr, $sink:expr) => {{
        validate_mask_len(($ix).len(), ($sink).nrows())?;
        validate_source_len(($source).nrows(), ($ix).iter().filter(|selected| **selected).count())?;
        let mut src_i = 0;
        for (i, rix) in (&$ix).iter().enumerate() {
            if *rix == true {
                let mut sink_row = ($sink).row_mut(i);
                let src_row = ($source).row(src_i);
                for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
                    *dst = checked_add_assign(*dst, *src)?;
                }
                src_i += 1;
            }
        }
        Ok::<(), MechError>(())
    }};
}

#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_s!(AddAssign2DRAS, add_assign_2d_vector_all, usize);
#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_s!(AddAssign2DRASB, add_assign_2d_vector_all_b, bool);
#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_v!(AddAssign2DRAV, add_assign_2d_vector_all_mat, usize);
#[cfg(feature = "matrix")]
impl_add_assign_range_fxn_v!(AddAssign2DRAVB, add_assign_2d_vector_all_mat_b, bool);

#[cfg(feature = "source")]
op_assign_range_all_fxn!(add_assign_range_all_fxn, AddAssign2DRA);

#[cfg(feature = "source")]
pub struct AddAssignRangeAll {}
#[cfg(feature = "source")]
impl FunctionSpecializer for AddAssignRangeAll {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
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
        let sink: LegacyValue = arguments[0].clone();
        let source: LegacyValue = arguments[1].clone();
        let ixes = arguments[2..].to_vec();
        match add_assign_range_all_fxn(sink.clone(), source.clone(), ixes.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (sink, ixes, source) {
                (LegacyValue::MutableReference(sink), ixes, LegacyValue::MutableReference(source)) => {
                    add_assign_range_all_fxn(
                        sink.borrow().clone(),
                        source.borrow().clone(),
                        ixes.clone(),
                    )
                }
                (sink, ixes, LegacyValue::MutableReference(source)) => {
                    add_assign_range_all_fxn(sink.clone(), source.borrow().clone(), ixes.clone())
                }
                (LegacyValue::MutableReference(sink), ixes, source) => {
                    add_assign_range_all_fxn(sink.borrow().clone(), source.clone(), ixes.clone())
                }
                (sink, ixes, source) => Err(MechError::new(
                    UnhandledFunctionArgumentIxes {
                        arg: (
                            sink.kind(),
                            ixes.iter().map(|x| x.kind()).collect(),
                            source.kind(),
                        ),
                        fxn_name: "math/add-assign/range-all".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
