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

#[cfg(feature = "range")]
pub(crate) fn canonical_range_drafts<T>(
    from: T,
    step: Option<T>,
    to: T,
    inclusive: bool,
) -> mech_core::MResult<Box<[mech_core::ValueDataDraft]>>
where
    T: mech_core::CanonicalMatrixElementBacking + mech_core::CanonicalRangeScalar,
{
    let output_len = mech_core::canonical_range_size(from, step, to, inclusive).map_err(|error| {
        mech_core::function_shape_contract_violation(
            "range_construction",
            format!("canonical typed range cardinality failed: {error:?}"),
        )
    })?;
    let mut elements = Vec::with_capacity(output_len);
    mech_core::visit_canonical_range(
        from,
        step,
        to,
        inclusive,
        output_len,
        |value| {
            elements.push(value.data_draft());
            Ok::<(), core::convert::Infallible>(())
        },
    )
    .map_err(|error| {
        mech_core::function_shape_contract_violation(
            "range_construction",
            format!("canonical typed range evaluation failed: {error:?}"),
        )
    })?;
    Ok(elements.into_boxed_slice())
}

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
    ($factory:ident, $scalar:ty, $first:expr, $second:expr, $inclusive:expr, $context:expr) => {{
        let inputs = vec![$first.cell()?.clone(), $second.cell()?.clone()].into_boxed_slice();
        let size = $crate::catalog::canonical_range_size(&inputs, $inclusive, false)?;
        let semantic_inputs = [$first, $second];
        return $context.bind_resolved_runtime(
            mech_core::RuntimeBindingSelector::Operation(
                $context.resolved_call()?.operation.id,
            ),
            mech_core::ExecutionTarget::DirectRuntime,
            vec![vec![1, size as u64].into_boxed_slice()].into_boxed_slice(),
            &semantic_inputs,
        )
    }};
}

#[doc(hidden)]
#[macro_export]
#[cfg(feature = "source")]
macro_rules! bind_dynamic_ternary_range {
    ($factory:ident, $scalar:ty, $first:expr, $step:expr, $last:expr, $inclusive:expr, $context:expr) => {{
        let inputs = vec![
            $first.cell()?.clone(),
            $step.cell()?.clone(),
            $last.cell()?.clone(),
        ]
        .into_boxed_slice();
        let size = $crate::catalog::canonical_range_size(&inputs, $inclusive, true)?;
        let semantic_inputs = [$first, $step, $last];
        return $context.bind_resolved_runtime(
            mech_core::RuntimeBindingSelector::Operation(
                $context.resolved_call()?.operation.id,
            ),
            mech_core::ExecutionTarget::DirectRuntime,
            vec![vec![1, size as u64].into_boxed_slice()].into_boxed_slice(),
            &semantic_inputs,
        )
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
