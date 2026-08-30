use super::*;
use num_traits::*;
use std::ops::DivAssign;

// Div Assign -----------------------------------------------------------------

// We will mostly use the assign macros for this

#[cfg(feature = "source")]
#[macro_export]
macro_rules! impl_div_assign_match_arms {
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
macro_rules! impl_div_assign_range_fxn_s {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        impl_op_assign_range_fxn_s!($struct_name, $op, $ix);
    };
}

#[cfg(feature = "matrix")]
macro_rules! impl_div_assign_range_fxn_v {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        impl_op_assign_range_fxn_v!($struct_name, $op, $ix);
    };
}

// x /= 1 ----------------------------------------------------------------------

impl_assign_scalar_scalar!(Div, checked_div_assign);
impl_assign_vector_vector!(Div, checked_div_assign);
impl_assign_vector_scalar!(Div, checked_div_assign);

// x[1..3] /= 1 ----------------------------------------------------------------

macro_rules! div_assign_1d_range {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            for &index in ($ix).iter() {
                let offset = checked_one_based_index(index, ($sink).len())?;
                ($sink)[offset] = checked_div_assign(($sink)[offset], *($source))?;
            }
            Ok::<(), MechError>(())
        }
    };
}

macro_rules! div_assign_1d_range_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            validate_mask_len(($ix).len(), ($sink).len())?;
            for (i, selected) in ($ix).iter().copied().enumerate() {
                if selected {
                    ($sink)[i] = checked_div_assign(($sink)[i], *($source))?;
                }
            }
            Ok::<(), MechError>(())
        }
    };
}

macro_rules! div_assign_1d_range_vec {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            validate_source_len(($source).len(), ($ix).len())?;
            for (i, &index) in ($ix).iter().enumerate() {
                let offset = checked_one_based_index(index, ($sink).len())?;
                ($sink)[offset] = checked_div_assign(($sink)[offset], ($source)[i])?;
            }
            Ok::<(), MechError>(())
        }
    };
}

macro_rules! div_assign_1d_range_vec_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            validate_mask_len(($ix).len(), ($sink).len())?;
            validate_source_len(($source).len(), ($ix).len())?;
            for (i, selected) in ($ix).iter().copied().enumerate() {
                if selected {
                    ($sink)[i] = checked_div_assign(($sink)[i], ($source)[i])?;
                }
            }
            Ok::<(), MechError>(())
        }
    };
}

#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_s!(DivAssign1DRS, div_assign_1d_range, usize);
#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_s!(DivAssign1DRB, div_assign_1d_range_b, bool);
#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_v!(DivAssign1DRV, div_assign_1d_range_vec, usize);
#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_v!(DivAssign1DRVB, div_assign_1d_range_vec_b, bool);

// x[1..3,:] /= 1 ------------------------------------------------------------------

macro_rules! div_assign_2d_vector_all {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            for &index in ($ix).iter() {
                checked_one_based_index(index, ($sink).nrows())?;
            }
            for &index in ($ix).iter() {
                let row = index - 1;
                for column in 0..($sink).ncols() {
                    let value = ($sink)[(row, column)];
                    ($sink)[(row, column)] = checked_div_assign(value, *($source))?;
                }
            }
            Ok::<(), MechError>(())
        }
    };
}

macro_rules! div_assign_2d_vector_all_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        {
            validate_mask_len(($ix).len(), ($sink).nrows())?;
            for (row, selected) in ($ix).iter().copied().enumerate() {
                if selected {
                    for column in 0..($sink).ncols() {
                        let value = ($sink)[(row, column)];
                        ($sink)[(row, column)] = checked_div_assign(value, *($source))?;
                    }
                }
            }
            Ok::<(), MechError>(())
        }
    };
}

macro_rules! div_assign_2d_vector_all_mat {
    ($source:expr, $ix:expr, $sink:expr) => {{
        let nsrc = $source.nrows();
        validate_source_len(nsrc, if ($ix).is_empty() { 0 } else { 1 })?;
        for (i, &rix) in $ix.iter().enumerate() {
            let row_index = checked_one_based_index(rix, ($sink).nrows())?;
            let mut sink_row = $sink.row_mut(row_index);
            let src_row = $source.row(i % nsrc); // wrap around!
            for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
                *dst = checked_div_assign(*dst, *src)?;
            }
        }
        Ok::<(), MechError>(())
    }};
}

macro_rules! div_assign_2d_vector_all_mat_b {
    ($source:expr, $ix:expr, $sink:expr) => {{
        validate_mask_len(($ix).len(), ($sink).nrows())?;
        validate_source_len(($source).nrows(), ($ix).iter().filter(|selected| **selected).count())?;
        let mut src_i = 0;
        for (i, rix) in (&$ix).iter().enumerate() {
            if *rix == true {
                let mut sink_row = ($sink).row_mut(i);
                let src_row = ($source).row(src_i);
                for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
                    *dst = checked_div_assign(*dst, *src)?;
                }
                src_i += 1;
            }
        }
        Ok::<(), MechError>(())
    }};
}

#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_s!(DivAssign2DRAS, div_assign_2d_vector_all, usize);
#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_s!(DivAssign2DRASB, div_assign_2d_vector_all_b, bool);
#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_v!(DivAssign2DRAV, div_assign_2d_vector_all_mat, usize);
#[cfg(feature = "matrix")]
impl_div_assign_range_fxn_v!(DivAssign2DRAVB, div_assign_2d_vector_all_mat_b, bool);
#[cfg(feature = "source")]
crate::impl_canonical_op_assign_specializers!(
    DivAssignValue,
    DivAssignRange,
    DivAssignRangeAll,
    Div,
    "DivAssign",
    "DivAssign1DR",
    "DivAssign2DRA"
);
