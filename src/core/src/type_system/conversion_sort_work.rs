//! Borrowed comparison facts for the SAME conversion normalizer, not execution.
use super::{ConversionPopulationError as Error, ConversionStep as Step};
use crate::snapshot::SequenceView;
use crate::{SchemaBody as Body, ValueData};

type Result<T, E> = core::result::Result<T, Error<E>>;

/// SAME maintained heapsort/dedup bound shared by source and resident owners.
/// Compound keys are bounded separately by complete current payload bytes.
pub fn conversion_sort_comparison_units(
    source: &Body,
    target: &Body,
    count: usize,
    longest_string: u64,
) -> Option<(u64, u64)> {
    let units = u64::try_from(count)
        .ok()?
        .checked_mul(3)?
        .checked_mul(u64::from(count.max(1).ilog2()) + 1)?;
    let width = match target {
        Body::String => Some(if source == &Body::String {
            longest_string.max(1)
        } else {
            768
        }),
        Body::Bool
        | Body::Index
        | Body::Id
        | Body::UnsignedInteger(_)
        | Body::SignedInteger(_)
        | Body::FloatingPoint(_) => Some(1),
        _ => None,
    };
    Some(match width {
        Some(width) => (units.checked_mul(width)?, 0),
        None => (0, units),
    })
}

#[derive(Default)]
struct Work {
    fixed: u64,
    compound: u64,
}
impl Work {
    fn group<E>(
        &mut self,
        source: &Body,
        target: &Body,
        count: usize,
        longest: u64,
    ) -> Result<(), E> {
        let (fixed, compound) = conversion_sort_comparison_units(source, target, count, longest)
            .ok_or(Error::ArithmeticOverflow)?;
        self.fixed = self
            .fixed
            .checked_add(fixed)
            .ok_or(Error::ArithmeticOverflow)?;
        self.compound = self
            .compound
            .checked_add(compound)
            .ok_or(Error::ArithmeticOverflow)?;
        Ok(())
    }
    fn complete<E>(&self, payload: u64) -> Result<u64, E> {
        self.compound
            .checked_mul(payload)
            .and_then(|v| v.checked_add(self.fixed))
            .ok_or(Error::ArithmeticOverflow)
    }
}
fn observe<E>(
    charge: &mut impl FnMut(u64) -> core::result::Result<(), E>,
    units: u64,
) -> Result<(), E> {
    charge(units).map_err(Error::Observer)
}
fn strings<E>(
    values: SequenceView<'_>,
    charge: &mut impl FnMut(u64) -> core::result::Result<(), E>,
) -> Result<u64, E> {
    if let SequenceView::String(values) = values {
        observe(
            charge,
            u64::try_from(values.len()).map_err(|_| Error::ArithmeticOverflow)?,
        )?;
        Ok(values.iter().map(|s| s.len() as u64).max().unwrap_or(0))
    } else {
        Ok(0)
    }
}

fn converted<E>(
    source: &Body,
    target: &Body,
    step: &Step,
    data: &ValueData,
    work: &mut Work,
    charge: &mut impl FnMut(u64) -> core::result::Result<(), E>,
) -> Result<(), E> {
    observe(charge, 64)?;
    match step {
        Step::Identity | Step::Scalar(_) => Ok(()),
        Step::OptionPresent(inner) => {
            let Body::Option(target) = target else {
                return Err(Error::InvalidValue);
            };
            converted(source, target, &inner.step, data, work, charge)
        }
        Step::OptionPayload(inner) => {
            let (Body::Option(source), Body::Option(target), ValueData::Option(value)) =
                (source, target, data)
            else {
                return Err(Error::InvalidValue);
            };
            if let Some(value) = value {
                converted(source, target, &inner.step, value, work, charge)?;
            }
            Ok(())
        }
        Step::MatrixElements(inner) | Step::MatrixToSet(inner) => {
            let (
                Body::Matrix {
                    element: source, ..
                },
                ValueData::Matrix(matrix),
            ) = (source, data)
            else {
                return Err(Error::InvalidValue);
            };
            let target = match (step, target) {
                (Step::MatrixElements(_), Body::Matrix { element, .. })
                | (Step::MatrixToSet(_), Body::Set { element, .. }) => element.as_ref(),
                _ => return Err(Error::InvalidValue),
            };
            let values = matrix.elements();
            if matches!(step, Step::MatrixToSet(_)) {
                let longest = if target == &Body::String {
                    strings(values, charge)?
                } else {
                    0
                };
                work.group(source, target, values.len(), longest)?;
            }
            if let SequenceView::Values(values) = values {
                for value in values {
                    converted(source, target, &inner.step, value, work, charge)?;
                }
            }
            Ok(())
        }
    }
}

fn sequence<E>(
    body: &Body,
    values: SequenceView<'_>,
    work: &mut Work,
    charge: &mut impl FnMut(u64) -> core::result::Result<(), E>,
) -> Result<(), E> {
    observe(charge, 64)?;
    if let SequenceView::Values(values) = values {
        for value in values {
            existing(body, value, work, charge)?;
        }
    }
    Ok(())
}

// Already canonical source Sets/Maps are rebuilt by key normalization. Bound
// those ordered insertions without executing a comparator before admission.
fn existing<E>(
    body: &Body,
    data: &ValueData,
    work: &mut Work,
    charge: &mut impl FnMut(u64) -> core::result::Result<(), E>,
) -> Result<(), E> {
    observe(charge, 64)?;
    match (body, data) {
        (Body::Option(body), ValueData::Option(Some(value))) => {
            existing(body, value, work, charge)?
        }
        (Body::Enum { variants, .. }, ValueData::Enum(value)) => {
            let variant = variants
                .get(value.ordinal() as usize)
                .ok_or(Error::InvalidValue)?;
            if let (Some(body), Some(value)) = (&variant.payload, value.payload()) {
                existing(body, value, work, charge)?;
            }
        }
        (Body::Matrix { element, .. }, ValueData::Matrix(matrix)) => {
            sequence(element, matrix.elements(), work, charge)?
        }
        (Body::Tuple(bodies), ValueData::Tuple(values)) => {
            for (body, value) in bodies.iter().zip(values) {
                existing(body, value, work, charge)?;
            }
        }
        (Body::Record(fields), ValueData::Record(values)) => {
            for (field, value) in fields.iter().zip(values.fields()) {
                existing(&field.schema, value, work, charge)?;
            }
        }
        (Body::Table { columns, .. }, ValueData::Table(values)) => {
            for (index, field) in columns.iter().enumerate() {
                sequence(
                    &field.schema,
                    values.column(index).ok_or(Error::InvalidValue)?,
                    work,
                    charge,
                )?;
            }
        }
        (Body::Set { element, .. }, ValueData::Set(values)) => {
            let mut longest = 0;
            for value in values.elements() {
                if let ValueData::String(value) = value.data() {
                    longest = longest.max(value.len() as u64);
                }
                existing(element, value.data(), work, charge)?;
            }
            work.group(element, element, values.elements().len(), longest)?;
        }
        (Body::Map { key, value, .. }, ValueData::Map(values)) => {
            let mut longest = 0;
            for entry in values.entries() {
                if let ValueData::String(value) = entry.key().data() {
                    longest = longest.max(value.len() as u64);
                }
                existing(key, entry.key().data(), work, charge)?;
                existing(value, entry.value(), work, charge)?;
            }
            work.group(key, key, values.entries().len(), longest)?;
        }
        _ => {}
    }
    Ok(())
}

/// All actual converted groups plus existing key roundtrips. The caller supplies
/// the complete converted-payload bound and real materialization phase count.
/// Absent Options are not visited; no draft, key or payload is allocated.
pub fn conversion_comparison_work<E>(
    source: &Body,
    target: &Body,
    step: &Step,
    data: &ValueData,
    payload: u64,
    phases: u64,
    mut charge: impl FnMut(u64) -> core::result::Result<(), E>,
) -> Result<u64, E> {
    let mut sorts = Work::default();
    converted(source, target, step, data, &mut sorts, &mut charge)?;
    let mut keys = Work::default();
    existing(source, data, &mut keys, &mut charge)?;
    let sorts = sorts
        .complete::<E>(payload)?
        .checked_mul(phases)
        .ok_or(Error::ArithmeticOverflow)?;
    let keys = keys
        .complete::<E>(payload)?
        .checked_mul(phases.checked_add(1).ok_or(Error::ArithmeticOverflow)?)
        .ok_or(Error::ArithmeticOverflow)?;
    sorts.checked_add(keys).ok_or(Error::ArithmeticOverflow)
}

/// Legacy dense lanes have no nested canonical keys. Their root population is
/// current descriptor geometry; borrowed payload bounds a String lane's length.
pub fn conversion_dense_comparison_work(
    source: &Body,
    target: &Body,
    step: &Step,
    count: usize,
    longest_string: u64,
    payload: u64,
    phases: u64,
) -> Option<u64> {
    match (step, target) {
        (Step::OptionPresent(inner), Body::Option(target)) => conversion_dense_comparison_work(
            source,
            target,
            &inner.step,
            count,
            longest_string,
            payload,
            phases,
        ),
        (
            Step::MatrixToSet(_),
            Body::Set {
                element: target, ..
            },
        ) => {
            let Body::Matrix {
                element: source, ..
            } = source
            else {
                return None;
            };
            let (fixed, compound) =
                conversion_sort_comparison_units(source, target, count, longest_string)?;
            compound
                .checked_mul(payload)?
                .checked_add(fixed)?
                .checked_mul(phases)
        }
        _ => None,
    }
}
