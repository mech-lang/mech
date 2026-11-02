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

/// Whether an axis is fixed for the active program, independently of its
/// current physical shape. A live axis that happens to be one is not fixed.
#[cfg(all(feature = "semantic-compiler", feature = "access"))]
pub(crate) fn canonical_fixed_matrix_axes(cell: &ValueCell) -> MResult<Vec<bool>> {
    fn fixed(expression: &crate::DimensionExpr, schema: &crate::Schema) -> bool {
        use crate::DimensionExpr;
        match expression {
            DimensionExpr::Constant(_) => true,
            DimensionExpr::Parameter(id) => schema
                .dimension_parameters()
                .get(id.get() as usize)
                .is_some_and(|parameter| parameter.lifetime() != crate::DimensionLifetime::Turn),
            DimensionExpr::Add(children)
            | DimensionExpr::Multiply(children)
            | DimensionExpr::Min(children)
            | DimensionExpr::Max(children) => children.iter().all(|child| fixed(child, schema)),
            DimensionExpr::Hole => false,
        }
    }
    let descriptor = cell.resolved_descriptor()?;
    let crate::SchemaBody::Matrix { dimensions, .. } = descriptor.schema().body() else {
        return Ok(Vec::new());
    };
    Ok(dimensions
        .iter()
        .map(|dimension| fixed(dimension, descriptor.schema()))
        .collect())
}

/// Canonical indexing has independent axis guarantees: linear gathers and
/// index conversion always produce columns, while a scalar axis selector
/// always contributes one. Retain those guarantees even when another axis
/// can grow, and retain the source element schema for empty selections.
#[cfg(all(feature = "semantic-compiler", feature = "access"))]
pub(crate) fn canonical_matrix_result_with_fixed_axes(
    source: &ValueCell,
    element: crate::SchemaBody,
    rows: usize,
    columns: usize,
    fixed_axes: [bool; 2],
    elements: Box<[ValueDataDraft]>,
) -> MResult<ValueCell> {
    let mut parameters = Vec::new();
    let dimensions = [rows, columns]
        .into_iter()
        .zip(fixed_axes)
        .map(|(extent, fixed)| {
            if fixed {
                crate::DimensionExpr::Constant(extent as u64)
            } else {
                let id = crate::DimensionParameterId::new(parameters.len() as u32);
                parameters.push(crate::DimensionParameterDeclaration {
                    id,
                    origin: crate::DimensionParameterOrigin::Inferred,
                    lifetime: crate::DimensionLifetime::Turn,
                    lower_bound: crate::DimensionExpr::Constant(0),
                    upper_bound: None,
                });
                crate::DimensionExpr::Parameter(id)
            }
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let resolved = crate::ResolvedType::from_schema_body(
        &crate::SchemaBody::Matrix {
            element: Box::new(element.clone()),
            dimensions,
        },
        &parameters,
    )
    .map_err(MechError::from)?;
    // As with the existing standalone source-result constructors, this
    // candidate is detached until managed specialization/publication adopts
    // it. The source contributes schemas, not a different ownership lifecycle.
    let owner = crate::MemoryDomain::new().map_err(MechError::from)?;
    ValueCell::matrix_from_resolved_type_drafts_in(
        &owner,
        &resolved,
        rows,
        columns,
        element,
        elements,
        std::slice::from_ref(source),
    )
}

#[cfg(feature = "semantic-compiler")]
fn canonical_index(cell: &ValueCell) -> MResult<usize> {
    let snapshot = cell.snapshot()?;
    let value = mech_core::canonical_positional_ordinal(snapshot.data()).map_err(|error| {
        MechError::new(
            GenericError {
                msg: format!("invalid positional access selector: {error:?}"),
            },
            None,
        )
        .with_compiler_loc()
    })?;
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
