#[cfg(feature = "semantic-compiler")]
use crate::{FunctionValueRepresentation, GenericError, SpecializationInput, ValueData};
use crate::{MResult, MechError, ValueCell, ValueCellSnapshotFailure, ValueDataDraft};

#[cfg(feature = "semantic-compiler")]
#[derive(Clone, Debug)]
pub(crate) enum CanonicalAccessSelector {
    All,
    Cell(ValueCell),
}

#[cfg(feature = "semantic-compiler")]
impl CanonicalAccessSelector {
    pub(crate) fn from_input(input: &SpecializationInput) -> MResult<Self> {
        match input {
            SpecializationInput::MatrixAllSelection => Ok(Self::All),
            SpecializationInput::Cell(cell) => Ok(Self::Cell(cell.clone())),
            SpecializationInput::Absent => Err(MechError::new(
                GenericError {
                    msg: "source absence is not an access selector".to_owned(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }

    #[cfg(feature = "access")]
    pub(crate) fn is_scalar(&self) -> bool {
        matches!(self, Self::Cell(cell) if !matches!(cell.representation(), FunctionValueRepresentation::Matrix { .. }))
    }
}

pub(crate) fn canonical_draft(cell: &ValueCell) -> MResult<ValueDataDraft> {
    cell.snapshot()?.canonical_data_draft().map_err(|error| {
        MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
    })
}

#[cfg(feature = "semantic-compiler")]
fn canonical_index(cell: &ValueCell) -> MResult<usize> {
    let snapshot = cell.snapshot()?;
    let value = match snapshot.data() {
        ValueData::Index(value) => *value as u128,
        ValueData::U8(value) => u128::from(*value),
        ValueData::U16(value) => u128::from(*value),
        ValueData::U32(value) => u128::from(*value),
        ValueData::U64(value) => u128::from(*value),
        ValueData::U128(value) => *value,
        ValueData::I8(value) if *value >= 0 => *value as u128,
        ValueData::I16(value) if *value >= 0 => *value as u128,
        ValueData::I32(value) if *value >= 0 => *value as u128,
        ValueData::I64(value) if *value >= 0 => *value as u128,
        ValueData::I128(value) if *value >= 0 => *value as u128,
        ValueData::F32(value) => value.to_f32().trunc().max(0.0) as u128,
        ValueData::F64(value) => value.to_f64().trunc().max(0.0) as u128,
        _ => {
            return Err(MechError::new(
                GenericError {
                    msg: "access selector must contain indices or booleans".to_owned(),
                },
                None,
            )
            .with_compiler_loc());
        }
    };
    usize::try_from(value).map_err(|_| {
        MechError::new(
            GenericError {
                msg: "access selector exceeds the host-independent index range".to_owned(),
            },
            None,
        )
        .with_compiler_loc()
    })
}

#[cfg(feature = "semantic-compiler")]
pub(crate) fn canonical_indices(
    selector: &CanonicalAccessSelector,
    upper: usize,
) -> MResult<Vec<usize>> {
    match selector {
        CanonicalAccessSelector::All => Ok((0..upper).collect()),
        CanonicalAccessSelector::Cell(cell)
            if matches!(
                cell.representation(),
                FunctionValueRepresentation::Matrix { .. }
            ) =>
        {
            let elements = cell.matrix_elements()?.ok_or_else(|| {
                MechError::new(
                    GenericError {
                        msg: "matrix selector does not expose canonical elements".to_owned(),
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
            if elements.first().is_some_and(|element| {
                matches!(
                    element.snapshot().map(|value| value.data().clone()),
                    Ok(ValueData::Bool(_))
                )
            }) {
                if elements.len() != upper {
                    return Err(MechError::new(
                        GenericError {
                            msg: format!(
                                "logical selector length {} does not match extent {upper}",
                                elements.len()
                            ),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                return elements
                    .iter()
                    .enumerate()
                    .filter_map(|(index, element)| match element.snapshot() {
                        Ok(value) if matches!(value.data(), ValueData::Bool(true)) => {
                            Some(Ok(index))
                        }
                        Ok(value) if matches!(value.data(), ValueData::Bool(false)) => None,
                        Ok(_) => Some(Err(MechError::new(
                            GenericError {
                                msg: "logical selector mixes boolean and non-boolean values"
                                    .to_owned(),
                            },
                            None,
                        )
                        .with_compiler_loc())),
                        Err(error) => Some(Err(error)),
                    })
                    .collect();
            }
            elements
                .iter()
                .map(canonical_index)
                .map(|index| {
                    index.and_then(|index| {
                        index
                            .checked_sub(1)
                            .filter(|index| *index < upper)
                            .ok_or_else(|| {
                                MechError::new(
                                    GenericError {
                                        msg: format!("access index {index} is outside 1..={upper}"),
                                    },
                                    None,
                                )
                                .with_compiler_loc()
                            })
                    })
                })
                .collect()
        }
        CanonicalAccessSelector::Cell(cell) => {
            let index = canonical_index(cell)?;
            Ok(vec![
                index
                    .checked_sub(1)
                    .filter(|index| *index < upper)
                    .ok_or_else(|| {
                        MechError::new(
                            GenericError {
                                msg: format!("access index {index} is outside 1..={upper}"),
                            },
                            None,
                        )
                        .with_compiler_loc()
                    })?,
            ])
        }
    }
}
