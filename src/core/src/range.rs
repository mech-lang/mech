//! Canonical typed range semantics shared by source execution and targets.

use crate::{
    ValueData,
    snapshot::{F32Bits, F64Bits},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalRangeError {
    InvalidInputCount,
    UnsupportedSchema,
    InvalidValue,
    Empty,
    MaximumExceeded,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CanonicalRangeVisitError<E> {
    Range(CanonicalRangeError),
    Visitor(E),
}

/// Scalar behavior used by the canonical range visitor. The source runtime
/// and target-side evaluation both call this visitor, so cardinality and
/// repeated-addition behavior cannot diverge.
#[doc(hidden)]
pub trait CanonicalRangeScalar: Copy {
    fn one() -> Self;
    fn size(from: Self, step: Self, to: Self, inclusive: bool) -> Option<usize>;
    fn checked_step(self, step: Self) -> Option<Self>;
}

fn integer_range_size(magnitude: u128, step: u128, inclusive: bool) -> Option<usize> {
    let size = if inclusive {
        magnitude.checked_div(step)?.checked_add(1)?
    } else {
        let quotient = magnitude.checked_div(step)?;
        quotient.checked_add(u128::from(magnitude % step != 0))?
    };
    usize::try_from(size).ok()
}

macro_rules! unsigned_range_scalar {
    ($($scalar:ty),+ $(,)?) => {
        $(
            impl CanonicalRangeScalar for $scalar {
                fn one() -> Self {
                    1
                }

                fn size(from: Self, step: Self, to: Self, inclusive: bool) -> Option<usize> {
                    if step == 0 || to < from {
                        return None;
                    }
                    integer_range_size(
                        u128::from(to).checked_sub(u128::from(from))?,
                        u128::from(step),
                        inclusive,
                    )
                }

                fn checked_step(self, step: Self) -> Option<Self> {
                    self.checked_add(step)
                }
            }
        )+
    };
}

unsigned_range_scalar!(u8, u16, u32, u64, u128);

macro_rules! signed_range_scalar {
    ($($scalar:ty),+ $(,)?) => {
        $(
            impl CanonicalRangeScalar for $scalar {
                fn one() -> Self {
                    1
                }

                fn size(from: Self, step: Self, to: Self, inclusive: bool) -> Option<usize> {
                    let from = from as i128;
                    let step = step as i128;
                    let to = to as i128;
                    if step == 0 {
                        return None;
                    }
                    if from == to {
                        return Some(usize::from(inclusive));
                    }
                    if (to > from && step < 0) || (to < from && step > 0) {
                        return Some(0);
                    }
                    integer_range_size(from.abs_diff(to), step.unsigned_abs(), inclusive)
                }

                fn checked_step(self, step: Self) -> Option<Self> {
                    self.checked_add(step)
                }
            }
        )+
    };
}

signed_range_scalar!(i8, i16, i32, i64, i128);

fn float_range_size(from: f64, step: f64, to: f64, inclusive: bool) -> Option<usize> {
    if !from.is_finite() || !step.is_finite() || !to.is_finite() || step == 0.0 {
        return None;
    }
    let difference = to - from;
    let size = if difference == 0.0 {
        if inclusive { 1.0 } else { 0.0 }
    } else if (difference > 0.0 && step > 0.0) || (difference < 0.0 && step < 0.0) {
        let quotient = difference / step;
        if inclusive {
            quotient.floor() + 1.0
        } else {
            quotient.ceil()
        }
    } else {
        0.0
    };
    if !size.is_finite() || size < 0.0 || size >= usize::MAX as f64 {
        return None;
    }
    Some(size as usize)
}

impl CanonicalRangeScalar for f32 {
    fn one() -> Self {
        1.0
    }

    fn size(from: Self, step: Self, to: Self, inclusive: bool) -> Option<usize> {
        float_range_size(from as f64, step as f64, to as f64, inclusive)
    }

    fn checked_step(self, step: Self) -> Option<Self> {
        Some(self + step)
    }
}

impl CanonicalRangeScalar for f64 {
    fn one() -> Self {
        1.0
    }

    fn size(from: Self, step: Self, to: Self, inclusive: bool) -> Option<usize> {
        float_range_size(from, step, to, inclusive)
    }

    fn checked_step(self, step: Self) -> Option<Self> {
        Some(self + step)
    }
}

pub fn canonical_range_size<T: CanonicalRangeScalar>(
    from: T,
    step: Option<T>,
    to: T,
    inclusive: bool,
) -> Result<usize, CanonicalRangeError> {
    let size = T::size(from, step.unwrap_or_else(T::one), to, inclusive)
        .ok_or(CanonicalRangeError::InvalidValue)?;
    if size == 0 {
        return Err(CanonicalRangeError::Empty);
    }
    Ok(size)
}

pub fn visit_canonical_range<T, E>(
    from: T,
    step: Option<T>,
    to: T,
    inclusive: bool,
    maximum_elements: usize,
    mut visit: impl FnMut(T) -> Result<(), E>,
) -> Result<usize, CanonicalRangeVisitError<E>>
where
    T: CanonicalRangeScalar,
{
    let step = step.unwrap_or_else(T::one);
    let size = canonical_range_size(from, Some(step), to, inclusive)
        .map_err(CanonicalRangeVisitError::Range)?;
    if size > maximum_elements {
        return Err(CanonicalRangeVisitError::Range(
            CanonicalRangeError::MaximumExceeded,
        ));
    }
    let mut current = from;
    for index in 0..size {
        visit(current).map_err(CanonicalRangeVisitError::Visitor)?;
        if index + 1 < size {
            current = current
                .checked_step(step)
                .ok_or(CanonicalRangeVisitError::Range(
                    CanonicalRangeError::ArithmeticOverflow,
                ))?;
        }
    }
    Ok(size)
}

macro_rules! value_range_arm {
    ($inputs:expr, $incremented:expr, $inclusive:expr, $maximum:expr, $visit:expr;
     $variant:ident, $scalar:ty, $wrap:expr) => {
        match ($inputs, $incremented) {
            ([ValueData::$variant(from), ValueData::$variant(to)], false) => {
                visit_canonical_range(*from, None, *to, $inclusive, $maximum, |value: $scalar| {
                    $visit(($wrap)(value))
                })
            }
            (
                [
                    ValueData::$variant(from),
                    ValueData::$variant(step),
                    ValueData::$variant(to),
                ],
                true,
            ) => visit_canonical_range(
                *from,
                Some(*step),
                *to,
                $inclusive,
                $maximum,
                |value: $scalar| $visit(($wrap)(value)),
            ),
            _ => return None,
        }
    };
}

fn visit_value_range_variant<E>(
    inputs: &[ValueData],
    inclusive: bool,
    incremented: bool,
    maximum_elements: usize,
    visit: &mut impl FnMut(ValueData) -> Result<(), E>,
) -> Option<Result<usize, CanonicalRangeVisitError<E>>> {
    macro_rules! try_variant {
        ($variant:ident, $scalar:ty, $wrap:expr) => {
            if matches!(inputs.first(), Some(ValueData::$variant(_))) {
                return Some(value_range_arm!(
                    inputs,
                    incremented,
                    inclusive,
                    maximum_elements,
                    visit;
                    $variant,
                    $scalar,
                    $wrap
                ));
            }
        };
    }
    try_variant!(U8, u8, ValueData::U8);
    try_variant!(U16, u16, ValueData::U16);
    try_variant!(U32, u32, ValueData::U32);
    try_variant!(U64, u64, ValueData::U64);
    try_variant!(Index, u64, ValueData::Index);
    try_variant!(U128, u128, ValueData::U128);
    try_variant!(I8, i8, ValueData::I8);
    try_variant!(I16, i16, ValueData::I16);
    try_variant!(I32, i32, ValueData::I32);
    try_variant!(I64, i64, ValueData::I64);
    try_variant!(I128, i128, ValueData::I128);
    try_variant!(F32, F32Bits, ValueData::F32);
    try_variant!(F64, F64Bits, ValueData::F64);
    None
}

macro_rules! value_range_size_arm {
    ($inputs:expr, $incremented:expr, $inclusive:expr; $variant:ident) => {
        match ($inputs, $incremented) {
            ([ValueData::$variant(from), ValueData::$variant(to)], false) => {
                canonical_range_size(*from, None, *to, $inclusive)
            }
            (
                [
                    ValueData::$variant(from),
                    ValueData::$variant(step),
                    ValueData::$variant(to),
                ],
                true,
            ) => canonical_range_size(*from, Some(*step), *to, $inclusive),
            _ => return None,
        }
    };
}

fn value_range_size_variant(
    inputs: &[ValueData],
    inclusive: bool,
    incremented: bool,
) -> Option<Result<usize, CanonicalRangeError>> {
    macro_rules! try_variant {
        ($variant:ident) => {
            if matches!(inputs.first(), Some(ValueData::$variant(_))) {
                return Some(value_range_size_arm!(
                    inputs,
                    incremented,
                    inclusive;
                    $variant
                ));
            }
        };
    }
    try_variant!(U8);
    try_variant!(U16);
    try_variant!(U32);
    try_variant!(U64);
    try_variant!(Index);
    try_variant!(U128);
    try_variant!(I8);
    try_variant!(I16);
    try_variant!(I32);
    try_variant!(I64);
    try_variant!(I128);
    try_variant!(F32);
    try_variant!(F64);
    None
}

// Bit wrappers use native floating-point addition while preserving their
// canonical serialized representation at the API boundary.
impl CanonicalRangeScalar for F32Bits {
    fn one() -> Self {
        Self::from_f32(1.0)
    }

    fn size(from: Self, step: Self, to: Self, inclusive: bool) -> Option<usize> {
        float_range_size(
            from.to_f32() as f64,
            step.to_f32() as f64,
            to.to_f32() as f64,
            inclusive,
        )
    }

    fn checked_step(self, step: Self) -> Option<Self> {
        Some(Self::from_f32(self.to_f32() + step.to_f32()))
    }
}

impl CanonicalRangeScalar for F64Bits {
    fn one() -> Self {
        Self::from_f64(1.0)
    }

    fn size(from: Self, step: Self, to: Self, inclusive: bool) -> Option<usize> {
        float_range_size(from.to_f64(), step.to_f64(), to.to_f64(), inclusive)
    }

    fn checked_step(self, step: Self) -> Option<Self> {
        Some(Self::from_f64(self.to_f64() + step.to_f64()))
    }
}

/// Visits a homogeneous canonical numeric range without first allocating its
/// output. Cardinality is checked against `maximum_elements` before the first
/// value is visited.
pub fn visit_canonical_value_range<E>(
    inputs: &[ValueData],
    inclusive: bool,
    incremented: bool,
    maximum_elements: usize,
    mut visit: impl FnMut(ValueData) -> Result<(), E>,
) -> Result<usize, CanonicalRangeVisitError<E>> {
    let expected = if incremented { 3 } else { 2 };
    if inputs.len() != expected {
        return Err(CanonicalRangeVisitError::Range(
            CanonicalRangeError::InvalidInputCount,
        ));
    }
    visit_value_range_variant(inputs, inclusive, incremented, maximum_elements, &mut visit).ok_or(
        CanonicalRangeVisitError::Range(CanonicalRangeError::UnsupportedSchema),
    )?
}

pub fn canonical_value_range_size(
    inputs: &[ValueData],
    inclusive: bool,
    incremented: bool,
) -> Result<usize, CanonicalRangeError> {
    let expected = if incremented { 3 } else { 2 };
    if inputs.len() != expected {
        return Err(CanonicalRangeError::InvalidInputCount);
    }
    value_range_size_variant(inputs, inclusive, incremented)
        .ok_or(CanonicalRangeError::UnsupportedSchema)?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_float_range_is_visited_before_selector_conversion() {
        let inputs = [
            ValueData::F32(F32Bits::from_f32(1.9)),
            ValueData::F32(F32Bits::from_f32(2.1)),
        ];
        let mut values = Vec::new();
        assert_eq!(
            visit_canonical_value_range(&inputs, true, false, 4, |value| {
                values.push(value);
                Ok::<(), ()>(())
            }),
            Ok(1),
        );
        assert!(matches!(
            values.as_slice(),
            [ValueData::F32(value)] if value.to_f32() == 1.9_f32
        ));
    }

    #[test]
    fn converted_index_range_remains_canonically_visitable() {
        let inputs = [ValueData::Index(1), ValueData::Index(3)];
        let mut values = Vec::new();
        assert_eq!(
            visit_canonical_value_range(&inputs, true, false, 3, |value| {
                values.push(value);
                Ok::<(), ()>(())
            }),
            Ok(3),
        );
        assert!(matches!(
            values.as_slice(),
            [
                ValueData::Index(1),
                ValueData::Index(2),
                ValueData::Index(3)
            ]
        ));
    }

    #[test]
    fn cardinality_is_bounded_before_visiting() {
        let inputs = [ValueData::U64(1), ValueData::U64(u64::MAX)];
        let mut visited = false;
        assert_eq!(
            visit_canonical_value_range(&inputs, true, false, 65_536, |_| {
                visited = true;
                Ok::<(), ()>(())
            }),
            Err(CanonicalRangeVisitError::Range(
                CanonicalRangeError::MaximumExceeded
            )),
        );
        assert!(!visited);
    }
}
