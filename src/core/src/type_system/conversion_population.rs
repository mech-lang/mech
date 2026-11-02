//! Exact expanded draft population over borrowed finalized data; no payload copy.
use crate::ValueData;
use crate::snapshot::SequenceView;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConversionPopulationError<E> {
    ArithmeticOverflow,
    InvalidValue,
    Observer(E),
}
type PopulationResult<E> = Result<u64, ConversionPopulationError<E>>;

fn add<E>(a: u64, b: u64) -> PopulationResult<E> {
    a.checked_add(b)
        .ok_or(ConversionPopulationError::ArithmeticOverflow)
}
fn observe<E>(
    observer: &mut impl FnMut(u64) -> Result<(), E>,
) -> Result<(), ConversionPopulationError<E>> {
    observer(64).map_err(ConversionPopulationError::Observer)
}
fn sequence<E>(
    values: SequenceView<'_>,
    observer: &mut impl FnMut(u64) -> Result<(), E>,
) -> PopulationResult<E> {
    observe(observer)?;
    if let SequenceView::Values(values) = values {
        values
            .iter()
            .try_fold(0, |n, value| add(n, walk(value, observer)?))
    } else {
        u64::try_from(values.len()).map_err(|_| ConversionPopulationError::ArithmeticOverflow)
    }
}
fn walk<E>(
    data: &ValueData,
    observer: &mut impl FnMut(u64) -> Result<(), E>,
) -> PopulationResult<E> {
    observe(observer)?;
    let mut nodes = 1;
    match data {
        ValueData::Option(Some(value)) => nodes = add(nodes, walk(value, observer)?)?,
        ValueData::Enum(value) => {
            if let Some(value) = value.payload() {
                nodes = add(nodes, walk(value, observer)?)?;
            }
        }
        ValueData::Dynamic(value) => {
            if let Some(value) = value.value() {
                nodes = add(add(nodes, 1)?, walk(value.data(), observer)?)?;
            }
        }
        ValueData::Matrix(value) => nodes = add(nodes, sequence(value.elements(), observer)?)?,
        ValueData::Table(value) => {
            for index in 0..value.len() {
                observe(observer)?;
                let values = value
                    .column(index)
                    .ok_or(ConversionPopulationError::InvalidValue)?;
                nodes = add(add(nodes, 1)?, sequence(values, observer)?)?;
            }
        }
        ValueData::Tuple(values) => {
            for value in values {
                nodes = add(nodes, walk(value, observer)?)?;
            }
        }
        ValueData::Record(value) => {
            for value in value.fields() {
                nodes = add(nodes, walk(value, observer)?)?;
            }
        }
        ValueData::Set(value) => {
            for value in value.elements() {
                nodes = add(nodes, walk(value.data(), observer)?)?;
            }
        }
        ValueData::Map(value) => {
            for entry in value.entries() {
                observe(observer)?;
                nodes = add(
                    add(add(nodes, 1)?, walk(entry.key().data(), observer)?)?,
                    walk(entry.value(), observer)?,
                )?;
            }
        }
        _ => {}
    }
    Ok(nodes)
}

/// The observer runs before descending or reading packed populations. Targets
/// can stop through their current meter, while source planning retains actual
/// observation work. Packed lengths (including Atom/Unit) are read in O(1).
pub fn conversion_draft_population<E>(
    data: &ValueData,
    mut observer: impl FnMut(u64) -> Result<(), E>,
) -> PopulationResult<E> {
    walk(data, &mut observer)
}
