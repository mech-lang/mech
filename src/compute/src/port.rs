use std::{collections::BTreeSet, error::Error, fmt, sync::Arc};

use mech_core::{CellSlotId, DimensionExpr, FloatWidth, NodeId, SchemaBody, SchemaId};
use mech_engine::{
    ArtifactSource, BindingDeclaration, ComputeRegionDeclaration, ProducerReference,
    ProgramArtifact, SlotRole,
};

use crate::{ComputeAdmissionError, ComputeDiagnostic, ComputeDiagnosticCode, turn_required_nodes};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ComputePortId(u32);

impl ComputePortId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComputeElementType {
    F32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TensorLayout {
    Scalar,
    RowMajor,
    ColumnMajor,
}

/// The single host-to-compute numeric narrowing boundary.
///
/// Existing non-finite values retain IEEE semantics. A finite f64 that would
/// become an infinity is rejected rather than silently changing meaning.
pub fn narrow_compute_f64(value: f64) -> Result<f32, f64> {
    let narrowed = value as f32;
    if value.is_finite() && !narrowed.is_finite() {
        Err(value)
    } else {
        Ok(narrowed)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputePort {
    pub id: ComputePortId,
    pub name: Box<str>,
    pub slot: CellSlotId,
    pub schema: SchemaId,
    pub element: ComputeElementType,
    pub dimensions: Box<[u64]>,
}

impl ComputePort {
    pub fn layout(&self) -> TensorLayout {
        if self.dimensions.is_empty() {
            TensorLayout::Scalar
        } else {
            TensorLayout::RowMajor
        }
    }

    pub fn elements(&self) -> Result<usize, ComputeValueError> {
        dimensions_elements(&self.dimensions)
    }

    pub fn normalize_value(&self, value: ComputeValue) -> Result<ComputeValue, ComputeValueError> {
        match (self.dimensions.is_empty(), value) {
            (true, ComputeValue::ScalarF32(value)) => Ok(ComputeValue::ScalarF32(value)),
            (true, value) => Err(ComputeValueError::KindMismatch {
                port: self.name.clone(),
                expected: "scalar f32",
                actual: value.kind_name(),
            }),
            (
                false,
                ComputeValue::TensorF32 {
                    dimensions,
                    layout,
                    values,
                },
            ) => {
                if dimensions.as_ref() != self.dimensions.as_ref() {
                    return Err(ComputeValueError::DimensionMismatch {
                        port: self.name.clone(),
                        expected: self.dimensions.clone(),
                        actual: dimensions,
                    });
                }
                let expected = self.elements()?;
                if values.len() != expected {
                    return Err(ComputeValueError::ElementCountMismatch {
                        port: self.name.clone(),
                        expected,
                        actual: values.len(),
                    });
                }
                let values = match layout {
                    TensorLayout::RowMajor => values,
                    TensorLayout::ColumnMajor => Arc::from(column_major_to_row_major(
                        &self.dimensions,
                        values.as_ref(),
                    )?),
                    TensorLayout::Scalar => {
                        return Err(ComputeValueError::LayoutMismatch {
                            port: self.name.clone(),
                            expected: TensorLayout::RowMajor,
                            actual: TensorLayout::Scalar,
                        });
                    }
                };
                Ok(ComputeValue::TensorF32 {
                    dimensions: self.dimensions.clone(),
                    layout: TensorLayout::RowMajor,
                    values,
                })
            }
            (false, value) => Err(ComputeValueError::KindMismatch {
                port: self.name.clone(),
                expected: "tensor f32",
                actual: value.kind_name(),
            }),
        }
    }

    /// Normalizes either one inner value or an outer batch of inner values.
    ///
    /// The source language only owns ordinary scalar and matrix values, so a
    /// coordinator may express a scalar batch as a row/column matrix and a
    /// matrix batch as one row of flattened inner elements per instance. The
    /// outer lane axis is nevertheless structural: values with the same total
    /// element count but a transposed or unrelated shape are rejected before
    /// storage is relabeled as `[instances, ..inner_dimensions]`.
    pub fn normalize_broadcast_value(
        &self,
        value: ComputeValue,
        expected_instances: Option<u32>,
    ) -> Result<(ComputeValue, u32), ComputeValueError> {
        if let Ok(value) = self.normalize_value(value.clone()) {
            return Ok((value, 1));
        }

        let ComputeValue::TensorF32 {
            dimensions,
            layout,
            values,
        } = value
        else {
            return self.normalize_value(value).map(|value| (value, 1));
        };
        let inner_elements = self.elements()?;
        if inner_elements == 0 || values.is_empty() || values.len() % inner_elements != 0 {
            return Err(ComputeValueError::ElementCountMismatch {
                port: self.name.clone(),
                expected: inner_elements,
                actual: values.len(),
            });
        }
        let instances = values.len() / inner_elements;
        if let Some(expected) = expected_instances
            && instances != 1
            && instances != expected as usize
        {
            let mut expected_dimensions = Vec::from(self.dimensions.as_ref());
            expected_dimensions.insert(0, u64::from(expected));
            return Err(ComputeValueError::DimensionMismatch {
                port: self.name.clone(),
                expected: expected_dimensions.into_boxed_slice(),
                actual: dimensions,
            });
        }
        let mut canonical_dimensions = Vec::from(self.dimensions.as_ref());
        canonical_dimensions.insert(0, instances as u64);
        let shape_is_canonical = dimensions.as_ref() == canonical_dimensions.as_slice();
        let shape_is_matrix_projection = !self.dimensions.is_empty()
            && dimensions.as_ref() == [instances as u64, inner_elements as u64];
        let shape_is_scalar_batch = self.dimensions.is_empty()
            && matches!(
                dimensions.as_ref(),
                [extent] if *extent == instances as u64
            );
        let shape_is_scalar_row_or_column = self.dimensions.is_empty()
            && matches!(
                dimensions.as_ref(),
                [rows, columns]
                    if (*rows == instances as u64 && *columns == 1)
                        || (*rows == 1 && *columns == instances as u64)
            );
        if !shape_is_canonical
            && !shape_is_matrix_projection
            && !shape_is_scalar_batch
            && !shape_is_scalar_row_or_column
        {
            return Err(ComputeValueError::DimensionMismatch {
                port: self.name.clone(),
                expected: canonical_dimensions.into_boxed_slice(),
                actual: dimensions,
            });
        }
        let instances =
            u32::try_from(instances).map_err(|_| ComputeValueError::ElementCountMismatch {
                port: self.name.clone(),
                expected: usize::try_from(u32::MAX).unwrap_or(usize::MAX),
                actual: values.len(),
            })?;
        let values = match layout {
            TensorLayout::RowMajor => values,
            TensorLayout::ColumnMajor => {
                Arc::from(column_major_to_row_major(&dimensions, values.as_ref())?)
            }
            TensorLayout::Scalar => {
                return Err(ComputeValueError::LayoutMismatch {
                    port: self.name.clone(),
                    expected: TensorLayout::RowMajor,
                    actual: TensorLayout::Scalar,
                });
            }
        };
        if instances == 1 {
            if self.dimensions.is_empty() {
                return Ok((ComputeValue::ScalarF32(values[0]), 1));
            }
            return Ok((
                ComputeValue::TensorF32 {
                    dimensions: self.dimensions.clone(),
                    layout: TensorLayout::RowMajor,
                    values,
                },
                1,
            ));
        }
        let mut canonical_dimensions = Vec::from(self.dimensions.as_ref());
        canonical_dimensions.insert(0, u64::from(instances));
        Ok((
            ComputeValue::TensorF32 {
                dimensions: canonical_dimensions.into_boxed_slice(),
                layout: TensorLayout::RowMajor,
                values,
            },
            instances,
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeStatePort {
    pub id: ComputePortId,
    pub name: Box<str>,
    pub slot: CellSlotId,
    pub schema: SchemaId,
    pub element: ComputeElementType,
    pub dimensions: Box<[u64]>,
}

impl ComputeStatePort {
    pub fn as_port(&self) -> ComputePort {
        ComputePort {
            id: self.id,
            name: self.name.clone(),
            slot: self.slot,
            schema: self.schema,
            element: self.element,
            dimensions: self.dimensions.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComputeRegionInterface {
    pub inputs: Box<[ComputePort]>,
    pub states: Box<[ComputeStatePort]>,
    pub outputs: Box<[ComputePort]>,
}

impl ComputeRegionInterface {
    pub fn input(&self, id: ComputePortId) -> Option<&ComputePort> {
        self.inputs.iter().find(|port| port.id == id)
    }

    pub fn input_named(&self, name: &str) -> Option<&ComputePort> {
        self.inputs.iter().find(|port| port.name.as_ref() == name)
    }

    pub fn normalize_input_update(
        &self,
        update: ComputeInputUpdate,
    ) -> Result<ComputeInputUpdate, ComputeInputError> {
        let port = self
            .input(update.port)
            .ok_or(ComputeInputError::UnknownInputPort { port: update.port })?;
        let value = port
            .normalize_value(update.value)
            .map_err(ComputeInputError::InvalidValue)?;
        Ok(ComputeInputUpdate {
            port: update.port,
            value,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ComputeValue {
    ScalarF32(f32),
    TensorF32 {
        dimensions: Box<[u64]>,
        layout: TensorLayout,
        values: Arc<[f32]>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComputeInputUpdate {
    pub port: ComputePortId,
    pub value: ComputeValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComputeInputError {
    UnknownInputPort { port: ComputePortId },
    InvalidValue(ComputeValueError),
}

impl fmt::Display for ComputeInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for ComputeInputError {}

impl ComputeValue {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::ScalarF32(_) => "scalar f32",
            Self::TensorF32 { .. } => "tensor f32",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComputeValueError {
    KindMismatch {
        port: Box<str>,
        expected: &'static str,
        actual: &'static str,
    },
    DimensionMismatch {
        port: Box<str>,
        expected: Box<[u64]>,
        actual: Box<[u64]>,
    },
    ElementCountMismatch {
        port: Box<str>,
        expected: usize,
        actual: usize,
    },
    LayoutMismatch {
        port: Box<str>,
        expected: TensorLayout,
        actual: TensorLayout,
    },
    ElementCountOverflow,
    LayoutRankUnsupported {
        rank: usize,
    },
}

impl fmt::Display for ComputeValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for ComputeValueError {}

pub fn column_major_to_row_major<T: Copy + Default>(
    dimensions: &[u64],
    values: &[T],
) -> Result<Vec<T>, ComputeValueError> {
    let [rows, columns] = dimensions else {
        return Err(ComputeValueError::LayoutRankUnsupported {
            rank: dimensions.len(),
        });
    };
    let rows = usize::try_from(*rows).map_err(|_| ComputeValueError::ElementCountOverflow)?;
    let columns = usize::try_from(*columns).map_err(|_| ComputeValueError::ElementCountOverflow)?;
    let elements = rows
        .checked_mul(columns)
        .ok_or(ComputeValueError::ElementCountOverflow)?;
    if values.len() != elements {
        return Err(ComputeValueError::ElementCountMismatch {
            port: "<layout conversion>".into(),
            expected: elements,
            actual: values.len(),
        });
    }
    let mut row_major = vec![T::default(); elements];
    for row in 0..rows {
        for column in 0..columns {
            row_major[row * columns + column] = values[column * rows + row];
        }
    }
    Ok(row_major)
}

pub fn build_compute_region_interface(
    artifact: &ProgramArtifact,
    region: Option<&ComputeRegionDeclaration>,
) -> Result<ComputeRegionInterface, ComputeAdmissionError> {
    let nodes = region
        .map(|region| region.nodes.iter().copied().collect::<BTreeSet<_>>())
        .unwrap_or_else(|| turn_required_nodes(artifact));
    let input_slots = artifact
        .bindings()
        .iter()
        .filter_map(|binding| match binding {
            BindingDeclaration::Input {
                node,
                source: ArtifactSource::Slot(slot),
                ..
            } if nodes.contains(node) => Some(*slot),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let state_slots = artifact
        .slots()
        .iter()
        .filter(|slot| {
            slot.role == SlotRole::State
                && (input_slots.contains(&slot.slot)
                    || slot_produced_by_nodes(artifact, slot.slot, &nodes))
        })
        .map(|slot| slot.slot)
        .collect::<BTreeSet<_>>();

    let mut diagnostics = Vec::new();
    let mut next_id = 0_u32;
    let inputs = artifact
        .inputs()
        .iter()
        .filter(|input| input_slots.contains(&input.slot))
        .filter_map(|input| {
            let port = port_from_schema(
                artifact,
                ComputePortId::new(next_id),
                input.name.clone().into_boxed_str(),
                input.slot,
                artifact.slots()[input.slot.get() as usize].schema,
                &mut diagnostics,
            );
            next_id += 1;
            port
        })
        .collect::<Vec<_>>();
    let states = state_slots
        .iter()
        .filter_map(|slot| {
            let schema = artifact.slots()[slot.get() as usize].schema;
            let port = port_from_schema(
                artifact,
                ComputePortId::new(next_id),
                format!("state-{}", slot.get()).into_boxed_str(),
                *slot,
                schema,
                &mut diagnostics,
            );
            next_id += 1;
            port.map(|port| ComputeStatePort {
                id: port.id,
                name: port.name,
                slot: port.slot,
                schema: port.schema,
                element: port.element,
                dimensions: port.dimensions,
            })
        })
        .collect::<Vec<_>>();
    let mut output_sources = Vec::new();
    for output in artifact.outputs() {
        expand_output_source(
            artifact,
            output.name.clone(),
            ArtifactSource::Slot(output.source),
            &mut output_sources,
            &mut diagnostics,
        );
    }
    let outputs = output_sources
        .into_iter()
        .filter(|(_, slot)| {
            state_slots.contains(slot) || slot_produced_by_nodes(artifact, *slot, &nodes)
        })
        .filter_map(|(name, slot)| {
            let schema = artifact.slots()[slot.get() as usize].schema;
            let port = port_from_schema(
                artifact,
                ComputePortId::new(next_id),
                name.into_boxed_str(),
                slot,
                schema,
                &mut diagnostics,
            );
            next_id += 1;
            port
        })
        .collect::<Vec<_>>();
    if !diagnostics.is_empty() {
        return Err(ComputeAdmissionError { diagnostics });
    }
    Ok(ComputeRegionInterface {
        inputs: inputs.into_boxed_slice(),
        states: states.into_boxed_slice(),
        outputs: outputs.into_boxed_slice(),
    })
}

fn expand_output_source(
    artifact: &ProgramArtifact,
    name: String,
    source: ArtifactSource,
    outputs: &mut Vec<(String, CellSlotId)>,
    diagnostics: &mut Vec<ComputeDiagnostic>,
) {
    let ArtifactSource::Slot(slot) = source else {
        diagnostics.push(ComputeDiagnostic {
            code: ComputeDiagnosticCode::ConstantUnsupported,
            node: None,
            operation: None,
            detail: format!("output `{name}` is a constant"),
        });
        return;
    };
    let declaration = &artifact.slots()[slot.get() as usize];
    if let ProducerReference::Output { source, .. } = declaration.producer {
        expand_output_source(artifact, name, source, outputs, diagnostics);
        return;
    }
    let ProducerReference::NodeOutput { node, .. } = declaration.producer else {
        outputs.push((name, slot));
        return;
    };
    let Some(producer) = artifact.nodes().get(node.get() as usize) else {
        diagnostics.push(ComputeDiagnostic {
            code: ComputeDiagnosticCode::ArtifactMalformed,
            node: Some(node),
            operation: None,
            detail: format!("output `{name}` references a missing producer"),
        });
        return;
    };
    if producer.operation.module_path.as_ref() != ["core"]
        || producer.operation.operation_name != "composite-pack"
    {
        outputs.push((name, slot));
        return;
    }

    let mut sources = producer
        .input_bindings
        .clone()
        .filter_map(|index| match artifact.bindings().get(index as usize) {
            Some(BindingDeclaration::Input { source, .. }) => Some(*source),
            Some(BindingDeclaration::Output { .. }) | None => None,
        })
        .collect::<Vec<_>>();
    let has_template = sources.first().is_some_and(|source| match source {
        ArtifactSource::Constant(constant) => artifact
            .constants()
            .get(*constant)
            .is_some_and(|value| value.schema() == declaration.schema),
        ArtifactSource::Slot(_) => false,
    });
    if has_template {
        sources.remove(0);
    }
    if sources.is_empty() {
        diagnostics.push(ComputeDiagnostic {
            code: ComputeDiagnosticCode::ArtifactMalformed,
            node: Some(node),
            operation: Some("core/composite-pack".to_owned()),
            detail: format!("output `{name}` has no physical components"),
        });
        return;
    }
    for (index, source) in sources.into_iter().enumerate() {
        expand_output_source(
            artifact,
            format!("{name}.{index}"),
            source,
            outputs,
            diagnostics,
        );
    }
}

fn port_from_schema(
    artifact: &ProgramArtifact,
    id: ComputePortId,
    name: Box<str>,
    slot: CellSlotId,
    schema: SchemaId,
    diagnostics: &mut Vec<ComputeDiagnostic>,
) -> Option<ComputePort> {
    let body = artifact.schemas().get(schema).map(|schema| schema.body());
    let dimensions = match body {
        Some(SchemaBody::FloatingPoint(FloatWidth::W32)) => Vec::new(),
        Some(SchemaBody::Matrix {
            element,
            dimensions,
        }) if matches!(element.as_ref(), SchemaBody::FloatingPoint(FloatWidth::W32)) => {
            let dimensions = dimensions
                .iter()
                .map(|dimension| match dimension {
                    DimensionExpr::Constant(extent) => Ok(*extent),
                    _ => Err(()),
                })
                .collect::<Result<Vec<_>, _>>();
            let Ok(dimensions) = dimensions else {
                diagnostics.push(ComputeDiagnostic {
                    code: ComputeDiagnosticCode::DynamicShapeUnsupported,
                    node: producer_node(artifact, slot),
                    operation: None,
                    detail: format!("port `{name}` has a dynamic matrix dimension"),
                });
                return None;
            };
            dimensions
        }
        _ => {
            diagnostics.push(ComputeDiagnostic {
                code: ComputeDiagnosticCode::SchemaUnsupported,
                node: producer_node(artifact, slot),
                operation: None,
                detail: format!("port `{name}` is not scalar f32 or a fixed-shape f32 matrix"),
            });
            return None;
        }
    };
    Some(ComputePort {
        id,
        name,
        slot,
        schema,
        element: ComputeElementType::F32,
        dimensions: dimensions.into_boxed_slice(),
    })
}

fn dimensions_elements(dimensions: &[u64]) -> Result<usize, ComputeValueError> {
    dimensions.iter().try_fold(1_usize, |elements, extent| {
        let extent =
            usize::try_from(*extent).map_err(|_| ComputeValueError::ElementCountOverflow)?;
        elements
            .checked_mul(extent)
            .ok_or(ComputeValueError::ElementCountOverflow)
    })
}

#[cfg(test)]
mod broadcast_tests {
    use super::*;

    fn matrix_port() -> ComputePort {
        ComputePort {
            id: ComputePortId::new(0),
            name: "control".into(),
            slot: CellSlotId::new(0),
            schema: SchemaId::new(0),
            element: ComputeElementType::F32,
            dimensions: vec![3, 1].into_boxed_slice(),
        }
    }

    #[test]
    fn matrix_batch_requires_the_outer_lane_axis() {
        let port = matrix_port();
        let (value, instances) = port
            .normalize_broadcast_value(
                ComputeValue::TensorF32 {
                    dimensions: vec![2, 3].into_boxed_slice(),
                    layout: TensorLayout::ColumnMajor,
                    values: Arc::from([1.0, 4.0, 2.0, 5.0, 3.0, 6.0]),
                },
                Some(2),
            )
            .unwrap();
        assert_eq!(instances, 2);
        assert_eq!(
            value,
            ComputeValue::TensorF32 {
                dimensions: vec![2, 3, 1].into_boxed_slice(),
                layout: TensorLayout::RowMajor,
                values: Arc::from([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
            }
        );

        let transposed = port.normalize_broadcast_value(
            ComputeValue::TensorF32 {
                dimensions: vec![3, 2].into_boxed_slice(),
                layout: TensorLayout::RowMajor,
                values: Arc::from([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
            },
            Some(2),
        );
        assert!(matches!(
            transposed,
            Err(ComputeValueError::DimensionMismatch { .. })
        ));
    }

    #[test]
    fn canonical_ranked_batch_is_not_reinterpreted() {
        let port = matrix_port();
        let values: Arc<[f32]> = Arc::from([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let (value, instances) = port
            .normalize_broadcast_value(
                ComputeValue::TensorF32 {
                    dimensions: vec![2, 3, 1].into_boxed_slice(),
                    layout: TensorLayout::RowMajor,
                    values: Arc::clone(&values),
                },
                Some(2),
            )
            .unwrap();
        assert_eq!(instances, 2);
        assert_eq!(
            value,
            ComputeValue::TensorF32 {
                dimensions: vec![2, 3, 1].into_boxed_slice(),
                layout: TensorLayout::RowMajor,
                values,
            }
        );
    }

    #[test]
    fn thousand_lane_matrix_shapes_are_unambiguous() {
        let port = matrix_port();
        let values = Arc::<[f32]>::from(vec![0.0; 3_000]);
        for layout in [TensorLayout::RowMajor, TensorLayout::ColumnMajor] {
            let (_, instances) = port
                .normalize_broadcast_value(
                    ComputeValue::TensorF32 {
                        dimensions: vec![1_000, 3].into_boxed_slice(),
                        layout,
                        values: Arc::clone(&values),
                    },
                    Some(1_000),
                )
                .unwrap();
            assert_eq!(instances, 1_000);
        }
        for dimensions in [vec![3, 1_000], vec![30, 100], vec![1_000, 2]] {
            assert!(
                port.normalize_broadcast_value(
                    ComputeValue::TensorF32 {
                        dimensions: dimensions.into_boxed_slice(),
                        layout: TensorLayout::RowMajor,
                        values: Arc::clone(&values),
                    },
                    Some(1_000),
                )
                .is_err()
            );
        }
    }

    #[test]
    fn single_matrix_and_scalar_batches_keep_their_declared_axes() {
        let matrix = matrix_port();
        let (_, instances) = matrix
            .normalize_broadcast_value(
                ComputeValue::TensorF32 {
                    dimensions: vec![3, 1].into_boxed_slice(),
                    layout: TensorLayout::ColumnMajor,
                    values: Arc::from([1.0, 2.0, 3.0]),
                },
                Some(1_000),
            )
            .unwrap();
        assert_eq!(instances, 1);

        let scalar = ComputePort {
            dimensions: Box::new([]),
            ..matrix_port()
        };
        for dimensions in [vec![1_000], vec![1_000, 1], vec![1, 1_000]] {
            let (_, instances) = scalar
                .normalize_broadcast_value(
                    ComputeValue::TensorF32 {
                        dimensions: dimensions.into_boxed_slice(),
                        layout: TensorLayout::RowMajor,
                        values: Arc::from(vec![1.0; 1_000]),
                    },
                    Some(1_000),
                )
                .unwrap();
            assert_eq!(instances, 1_000);
        }
    }
}

fn slot_produced_by_nodes(
    artifact: &ProgramArtifact,
    slot: CellSlotId,
    nodes: &BTreeSet<NodeId>,
) -> bool {
    match artifact.slots()[slot.get() as usize].producer {
        ProducerReference::Input(_) => false,
        ProducerReference::NodeOutput { node, .. } => nodes.contains(&node),
        ProducerReference::Output { source, .. } => match source {
            ArtifactSource::Constant(_) => false,
            ArtifactSource::Slot(source) => slot_produced_by_nodes(artifact, source, nodes),
        },
    }
}

fn producer_node(artifact: &ProgramArtifact, slot: CellSlotId) -> Option<NodeId> {
    match artifact.slots().get(slot.get() as usize)?.producer {
        ProducerReference::Input(_) => None,
        ProducerReference::NodeOutput { node, .. } => Some(node),
        ProducerReference::Output { source, .. } => match source {
            ArtifactSource::Constant(_) => None,
            ArtifactSource::Slot(source) => producer_node(artifact, source),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn port(dimensions: &[u64]) -> ComputePort {
        ComputePort {
            id: ComputePortId::new(0),
            name: "value".into(),
            slot: CellSlotId::new(0),
            schema: SchemaId::new(0),
            element: ComputeElementType::F32,
            dimensions: dimensions.into(),
        }
    }

    #[test]
    fn scalar_and_one_by_one_matrix_are_distinct() {
        let scalar = port(&[]);
        let matrix = port(&[1, 1]);
        assert!(scalar.normalize_value(ComputeValue::ScalarF32(1.0)).is_ok());
        assert!(
            matrix
                .normalize_value(ComputeValue::ScalarF32(1.0))
                .is_err()
        );
        assert!(
            scalar
                .normalize_value(ComputeValue::TensorF32 {
                    dimensions: vec![1, 1].into_boxed_slice(),
                    layout: TensorLayout::RowMajor,
                    values: Arc::from([1.0]),
                })
                .is_err()
        );
    }

    #[test]
    fn rectangular_dimensions_are_validated_exactly() {
        let port = port(&[2, 3]);
        let error = port
            .normalize_value(ComputeValue::TensorF32 {
                dimensions: vec![3, 2].into_boxed_slice(),
                layout: TensorLayout::RowMajor,
                values: Arc::from([0.0; 6]),
            })
            .unwrap_err();
        assert!(matches!(error, ComputeValueError::DimensionMismatch { .. }));
    }

    #[test]
    fn column_major_ingress_converts_once_to_canonical_row_major() {
        let port = port(&[2, 3]);
        let value = port
            .normalize_value(ComputeValue::TensorF32 {
                dimensions: vec![2, 3].into_boxed_slice(),
                layout: TensorLayout::ColumnMajor,
                values: Arc::from([1.0, 4.0, 2.0, 5.0, 3.0, 6.0]),
            })
            .unwrap();
        let ComputeValue::TensorF32 { layout, values, .. } = value else {
            panic!("matrix ingress became a scalar")
        };
        assert_eq!(layout, TensorLayout::RowMajor);
        assert_eq!(values.as_ref(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn runtime_delivery_uses_the_planned_port_shape() {
        let interface = ComputeRegionInterface {
            inputs: vec![port(&[2, 3])].into_boxed_slice(),
            ..Default::default()
        };
        let error = interface
            .normalize_input_update(ComputeInputUpdate {
                port: ComputePortId::new(0),
                value: ComputeValue::TensorF32 {
                    dimensions: vec![3, 2].into_boxed_slice(),
                    layout: TensorLayout::RowMajor,
                    values: Arc::from([0.0; 6]),
                },
            })
            .unwrap_err();
        assert!(matches!(
            error,
            ComputeInputError::InvalidValue(ComputeValueError::DimensionMismatch { .. })
        ));
    }
}
