#![cfg_attr(not(test), no_main)]
#![feature(where_clause_attrs)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    pub use crate::catalog::__mech_native::*;
}

#[cfg(feature = "matrix")]
extern crate nalgebra as na;
extern crate paste;

#[cfg(feature = "range")]
use num_traits::One;
use std::fmt::Debug;
use std::ops::*;

#[cfg(feature = "runtime")]
pub mod catalog;
#[cfg(feature = "runtime")]
pub use self::catalog::*;

#[cfg(feature = "exclusive")]
pub mod exclusive;
#[cfg(feature = "exclusive")]
pub mod exclusive_increment;
#[cfg(feature = "inclusive")]
pub mod inclusive;
#[cfg(feature = "inclusive")]
pub mod inclusive_increment;

#[cfg(feature = "exclusive")]
pub use self::exclusive::*;
#[cfg(feature = "exclusive")]
pub use self::exclusive_increment::*;
#[cfg(feature = "inclusive")]
pub use self::inclusive::*;
#[cfg(feature = "inclusive")]
pub use self::inclusive_increment::*;

#[cfg(test)]
mod port_tests;

use mech_core::MechErrorKind;

// ----------------------------------------------------------------------------
// Range Library
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EmptyRangeError;
impl MechErrorKind for EmptyRangeError {
    fn name(&self) -> &str {
        "EmptyRange"
    }
    fn message(&self) -> String {
        "Range size must be > 0".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct RangeSizeOverflowError;

impl MechErrorKind for RangeSizeOverflowError {
    fn name(&self) -> &str {
        "RangeSizeOverflow"
    }
    fn message(&self) -> String {
        "Range size overflow".to_string()
    }
}

#[doc(hidden)]
#[macro_export]
#[cfg(feature = "source")]
macro_rules! bind_dynamic_binary_range {
    ($factory:ident, $scalar:ty, $first:expr, $second:expr, $inclusive:expr) => {{
        let inputs = vec![$first.cell()?.clone(), $second.cell()?.clone()].into_boxed_slice();
        let size = $crate::catalog::canonical_range_size(&inputs, $inclusive, false)?;
        let initial = *$first.try_ref::<$scalar>()?.borrow();
        #[cfg(feature = "row_vectord")]
        {
            let output_ref = mech_core::Ref::new(nalgebra::RowDVector::<$scalar>::from_element(
                size, initial,
            ));
            let output = mech_core::ValueCell::from_exact_matrix_ref(
                output_ref,
                1,
                size,
            )?;
            return mech_core::SpecializedFunction::bind_factory::<
                $factory<$scalar, nalgebra::RowDVector<$scalar>>,
            >(output, inputs);
        }
        #[cfg(all(not(feature = "row_vectord"), feature = "matrixd"))]
        {
            let output_ref = mech_core::Ref::new(nalgebra::DMatrix::<$scalar>::from_element(
                1, size, initial,
            ));
            let output = mech_core::ValueCell::from_exact_matrix_ref(
                output_ref,
                1,
                size,
            )?;
            return mech_core::SpecializedFunction::bind_factory::<
                $factory<$scalar, nalgebra::DMatrix<$scalar>>,
            >(output, inputs);
        }
        #[cfg(all(not(feature = "matrixd"), not(feature = "row_vectord")))]
        return Err(mech_core::function_shape_contract_violation(
            "range_construction",
            "source range construction requires a dynamic row or matrix backing",
        ));
    }};
}

#[doc(hidden)]
#[macro_export]
#[cfg(feature = "source")]
macro_rules! bind_dynamic_ternary_range {
    ($factory:ident, $scalar:ty, $first:expr, $step:expr, $last:expr, $inclusive:expr) => {{
        let inputs = vec![
            $first.cell()?.clone(),
            $step.cell()?.clone(),
            $last.cell()?.clone(),
        ]
        .into_boxed_slice();
        let size = $crate::catalog::canonical_range_size(&inputs, $inclusive, true)?;
        let initial = *$first.try_ref::<$scalar>()?.borrow();
        #[cfg(feature = "row_vectord")]
        {
            let output_ref = mech_core::Ref::new(nalgebra::RowDVector::<$scalar>::from_element(
                size, initial,
            ));
            let output = mech_core::ValueCell::from_exact_matrix_ref(
                output_ref,
                1,
                size,
            )?;
            return mech_core::SpecializedFunction::bind_factory::<
                $factory<$scalar, nalgebra::RowDVector<$scalar>>,
            >(output, inputs);
        }
        #[cfg(all(not(feature = "row_vectord"), feature = "matrixd"))]
        {
            let output_ref = mech_core::Ref::new(nalgebra::DMatrix::<$scalar>::from_element(
                1, size, initial,
            ));
            let output = mech_core::ValueCell::from_exact_matrix_ref(
                output_ref,
                1,
                size,
            )?;
            return mech_core::SpecializedFunction::bind_factory::<
                $factory<$scalar, nalgebra::DMatrix<$scalar>>,
            >(output, inputs);
        }
        #[cfg(all(not(feature = "matrixd"), not(feature = "row_vectord")))]
        return Err(mech_core::function_shape_contract_violation(
            "range_construction",
            "source range construction requires a dynamic row or matrix backing",
        ));
    }};
}

#[macro_export]
macro_rules! range_size_to_usize {
    // Float f32 branch
    ($diff:expr, f32) => {{
        let v: f32 = $diff;
        if v < 0.0 {
            return Err(MechError::new(RangeSizeOverflowError {}, None).with_compiler_loc());
        }
        v as usize
    }};

    // Float f64 branch
    ($diff:expr, f64) => {{
        let v: f64 = $diff;
        if v < 0.0 {
            return Err(MechError::new(RangeSizeOverflowError {}, None).with_compiler_loc());
        }
        v as usize
    }};

    // Integer branch
    ($diff:expr, $ty:ty) => {{
        $diff
            .try_into()
            .map_err(|_| MechError::new(RangeSizeOverflowError {}, None).with_compiler_loc())?
    }};
}
