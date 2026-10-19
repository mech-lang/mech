//! Borrowed, incrementally metered populations for nested conversion staging.
use super::super::budget::{
    ResidentBudgetMeter, checked_cost_product as mul, checked_cost_sum as sum, checked_u64,
};
use mech_core::snapshot::SequenceView;
use mech_core::{
    ConversionStep as Step, FloatWidth, IntegerWidth, ResidentKernelError as Error,
    ResidentValueRef, SchemaBody as Body, ValueData,
};
use std::mem::size_of;

#[cfg(test)]
#[path = "set_conversion_storage_tests.rs"]
mod storage_tests;

// These are physical carrier facts from maintained SequenceStorage, not a
// second conversion evaluator. Existing Values already retain their inline
// ValueData headers; packed lanes retain only the indicated carrier width.
fn packed_width(values: SequenceView<'_>) -> Option<u64> {
    Some(
        (match values {
            SequenceView::U8(_) => size_of::<u8>(),
            SequenceView::U16(_) => size_of::<u16>(),
            SequenceView::U32(_) => size_of::<u32>(),
            SequenceView::U64(_) => size_of::<u64>(),
            SequenceView::U128(_) => size_of::<u128>(),
            SequenceView::I8(_) => size_of::<i8>(),
            SequenceView::I16(_) => size_of::<i16>(),
            SequenceView::I32(_) => size_of::<i32>(),
            SequenceView::I64(_) => size_of::<i64>(),
            SequenceView::I128(_) => size_of::<i128>(),
            SequenceView::F32(_) => size_of::<mech_core::snapshot::F32Bits>(),
            SequenceView::F64(_) => size_of::<mech_core::snapshot::F64Bits>(),
            SequenceView::Complex32(_) => size_of::<mech_core::snapshot::Complex32Bits>(),
            SequenceView::Complex64(_) => size_of::<mech_core::snapshot::Complex64Bits>(),
            SequenceView::Rational64(_) => size_of::<mech_core::snapshot::Rational64Value>(),
            SequenceView::Bool(_) => size_of::<bool>(),
            SequenceView::Id(_) | SequenceView::Index(_) => size_of::<u64>(),
            SequenceView::String(_) => size_of::<Box<str>>(),
            SequenceView::Unit(_) => 0,
            SequenceView::Values(_) => return None,
        }) as u64,
    )
}

fn target_matrix_width(target: &Body) -> u64 {
    (match target {
        Body::IntegerInterval(interval) => return target_matrix_width(&interval.base_body()),
        Body::UnsignedInteger(width) | Body::SignedInteger(width) => match width {
            IntegerWidth::W8 => size_of::<u8>(),
            IntegerWidth::W16 => size_of::<u16>(),
            IntegerWidth::W32 => size_of::<u32>(),
            IntegerWidth::W64 => size_of::<u64>(),
            IntegerWidth::W128 => size_of::<u128>(),
        },
        Body::FloatingPoint(FloatWidth::W32) => size_of::<mech_core::snapshot::F32Bits>(),
        Body::FloatingPoint(FloatWidth::W64) => size_of::<mech_core::snapshot::F64Bits>(),
        Body::Complex(FloatWidth::W32) => size_of::<mech_core::snapshot::Complex32Bits>(),
        Body::Complex(FloatWidth::W64) => size_of::<mech_core::snapshot::Complex64Bits>(),
        Body::Rational64 => size_of::<mech_core::snapshot::Rational64Value>(),
        Body::Bool => size_of::<bool>(),
        Body::Id | Body::Index => size_of::<u64>(),
        Body::String => size_of::<Box<str>>(),
        Body::Atom(_) => 0,
        _ => size_of::<ValueData>(),
    }) as u64
}

/// Counts EXPANDED drafts, not just physical retained nodes. In particular a
/// packed Atom matrix retains no element bytes but still expands N drafts.
pub(super) fn draft_nodes(data: &ValueData, meter: &mut ResidentBudgetMeter) -> Result<u64, Error> {
    mech_core::conversion_draft_population(data, |units| meter.charge_compute_work(units)).map_err(
        |error| match error {
            mech_core::ConversionPopulationError::ArithmeticOverflow => Error::InvalidShape,
            mech_core::ConversionPopulationError::InvalidValue => Error::InvalidInput,
            mech_core::ConversionPopulationError::Observer(error) => error,
        },
    )
}

#[derive(Default)]
pub(super) struct Stats {
    pub(super) sort_elements: u64,
    pub(super) fixed_comparison: u64,
    pub(super) compound_comparison_units: u64,
    pub(super) option_wrappers: u64,
    pub(super) string_casts: u64,
    pub(super) storage_growth: u64,
}

impl Stats {
    fn grow_storage(
        &mut self,
        source_width: u64,
        target_width: u64,
        count: u64,
    ) -> Result<(), Error> {
        self.storage_growth = sum(&[
            self.storage_growth,
            mul(&[count, target_width.saturating_sub(source_width)])?,
        ])?;
        Ok(())
    }

    fn scalar(&mut self, step: &Step, count: u64) -> Result<(), Error> {
        match step {
            Step::Identity => Ok(()),
            Step::Scalar(mech_core::ScalarConversion::Builtin { source, target, .. }) => {
                if *target == mech_core::BuiltinScalarKind::String && source != target {
                    self.string_casts = sum(&[self.string_casts, count])?;
                }
                Ok(())
            }
            Step::OptionPresent(inner) => {
                self.option_wrappers = sum(&[self.option_wrappers, count])?;
                self.scalar(&inner.step, count)
            }
            _ => Err(Error::InvalidInput),
        }
    }

    fn sort(
        &mut self,
        source: &Body,
        target: &Body,
        count: usize,
        longest_string: u64,
    ) -> Result<(), Error> {
        let n = checked_u64(count)?;
        self.sort_elements = sum(&[self.sort_elements, n])?;
        let (fixed, compound) =
            mech_core::conversion_sort_comparison_units(source, target, count, longest_string)
                .ok_or(Error::InvalidShape)?;
        self.fixed_comparison = sum(&[self.fixed_comparison, fixed])?;
        self.compound_comparison_units = sum(&[self.compound_comparison_units, compound])?;
        Ok(())
    }

    pub(super) fn snapshot(
        &mut self,
        step: &Step,
        source: &Body,
        target: &Body,
        data: &ValueData,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<(), Error> {
        meter.charge_compute_work(64)?;
        match step {
            Step::Identity | Step::Scalar(_) => self.scalar(step, 1),
            Step::OptionPayload(inner) => {
                let (Body::Option(source), Body::Option(target), ValueData::Option(value)) =
                    (source, target, data)
                else {
                    return Err(Error::InvalidInput);
                };
                if let Some(value) = value {
                    self.snapshot(&inner.step, source, target, value, meter)?;
                }
                Ok(())
            }
            Step::OptionPresent(inner) => {
                let Body::Option(target) = target else {
                    return Err(Error::InvalidInput);
                };
                self.option_wrappers = sum(&[self.option_wrappers, 1])?;
                self.snapshot(&inner.step, source, target, data, meter)
            }
            Step::MatrixElements(inner) | Step::MatrixToSet(inner) => {
                let (
                    Body::Matrix {
                        element: source, ..
                    },
                    ValueData::Matrix(matrix),
                ) = (source, data)
                else {
                    return Err(Error::InvalidInput);
                };
                let target = match (step, target) {
                    (Step::MatrixElements(_), Body::Matrix { element, .. })
                    | (Step::MatrixToSet(_), Body::Set { element, .. }) => element.as_ref(),
                    _ => return Err(Error::InvalidInput),
                };
                let values = matrix.elements();
                if matches!(step, Step::MatrixToSet(_)) {
                    let longest = if let SequenceView::String(strings) = values {
                        meter.charge_compute_work(checked_u64(strings.len())?)?;
                        strings.iter().map(|s| s.len() as u64).max().unwrap_or(0)
                    } else {
                        0
                    };
                    self.sort(source, target, values.len(), longest)?;
                }
                if let SequenceView::Values(values) = values {
                    for value in values {
                        self.snapshot(&inner.step, source, target, value, meter)?;
                    }
                    Ok(())
                } else {
                    let source_width = packed_width(values).ok_or(Error::InvalidInput)?;
                    let target_width = if matches!(step, Step::MatrixToSet(_)) {
                        size_of::<ValueData>() as u64
                    } else {
                        target_matrix_width(target)
                    };
                    self.grow_storage(source_width, target_width, checked_u64(values.len())?)?;
                    self.scalar(&inner.step, checked_u64(values.len())?)
                }
            }
        }
    }

    pub(super) fn dense(
        &mut self,
        step: &Step,
        source: &Body,
        target: &Body,
        values: ResidentValueRef<'_>,
        longest_string: u64,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<(), Error> {
        meter.charge_compute_work(64)?;
        match (step, target) {
            (Step::OptionPresent(inner), Body::Option(target)) => {
                self.option_wrappers = sum(&[self.option_wrappers, 1])?;
                self.dense(&inner.step, source, target, values, longest_string, meter)
            }
            (
                Step::MatrixToSet(inner),
                Body::Set {
                    element: target, ..
                },
            ) => {
                let Body::Matrix {
                    element: source, ..
                } = source
                else {
                    return Err(Error::InvalidInput);
                };
                let source_width = (match (values, source.as_ref()) {
                    (ResidentValueRef::F64(_), Body::FloatingPoint(FloatWidth::W64)) => {
                        size_of::<f64>()
                    }
                    (ResidentValueRef::Bool(_), Body::Bool) => size_of::<u8>(),
                    (ResidentValueRef::Index(_), Body::Index) => size_of::<u64>(),
                    (ResidentValueRef::String(_), Body::String) => size_of::<String>(),
                    _ => return Err(Error::InvalidInput),
                }) as u64;
                let count = checked_u64(values.len())?;
                self.grow_storage(source_width, size_of::<ValueData>() as u64, count)?;
                self.sort(source, target, values.len(), longest_string)?;
                self.scalar(&inner.step, count)
            }
            _ => Err(Error::InvalidInput),
        }
    }
}
