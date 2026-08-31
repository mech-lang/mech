use std::collections::BTreeSet;

use crate::FixedShapeIr;
use mech_core::{CellSlotId, NodeId};
use mech_engine::{
    ArtifactSource, BindingDeclaration, OperationReference, ProducerReference, ProgramArtifact,
    SlotRole,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl BinaryOperation {
    pub fn apply(self, left: f32, right: f32) -> f32 {
        match self {
            Self::Add => left + right,
            Self::Subtract => left - right,
            Self::Multiply => left * right,
            Self::Divide => left / right,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOperation {
    Sin,
    Cos,
    Sqrt,
    Ceil,
}

impl UnaryOperation {
    pub fn apply(self, input: f32) -> f32 {
        match self {
            Self::Sin => input.sin(),
            Self::Cos => input.cos(),
            Self::Sqrt => input.sqrt(),
            Self::Ceil => input.ceil(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElementwiseOperation {
    Binary(BinaryOperation),
    Unary(UnaryOperation),
    Atan2,
    Identity,
}

impl ElementwiseOperation {
    pub const fn arity(self) -> usize {
        match self {
            Self::Unary(_) | Self::Identity => 1,
            Self::Binary(_) | Self::Atan2 => 2,
        }
    }

    pub fn apply(self, inputs: &[f32]) -> f32 {
        match self {
            Self::Binary(operation) => operation.apply(inputs[0], inputs[1]),
            Self::Unary(operation) => operation.apply(inputs[0]),
            Self::Atan2 => inputs[0].atan2(inputs[1]),
            Self::Identity => inputs[0],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElementwiseLowering {
    Apply(ElementwiseOperation),
    Concatenate(ConcatenationAxis),
}

impl ElementwiseLowering {
    pub const fn fixed_arity(self) -> Option<usize> {
        match self {
            Self::Apply(operation) => Some(operation.arity()),
            Self::Concatenate(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConcatenationAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConcatenationInput {
    pub source: ArtifactSource,
    pub rows: u64,
    pub columns: u64,
}

impl ConcatenationInput {
    pub fn elements(self) -> u64 {
        self.rows
            .checked_mul(self.columns)
            .expect("validated concatenation input element count")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ElementwiseInstruction {
    Apply {
        operation: ElementwiseOperation,
        inputs: Box<[ArtifactSource]>,
        output: CellSlotId,
        elements: u64,
    },
    Concatenate {
        axis: ConcatenationAxis,
        inputs: Box<[ConcatenationInput]>,
        output: CellSlotId,
        rows: u64,
        columns: u64,
    },
}

impl ElementwiseInstruction {
    pub const fn output(&self) -> CellSlotId {
        match self {
            Self::Apply { output, .. } | Self::Concatenate { output, .. } => *output,
        }
    }

    pub fn elements(&self) -> u64 {
        match self {
            Self::Apply { elements, .. } => *elements,
            Self::Concatenate { rows, columns, .. } => rows
                .checked_mul(*columns)
                .expect("validated concatenation element count"),
        }
    }

    pub fn concat_source_at(&self, index: u64) -> Option<(ArtifactSource, u64, u64)> {
        match self {
            Self::Apply { .. } => None,
            Self::Concatenate {
                axis,
                inputs,
                rows,
                columns,
                ..
            } => {
                if index >= rows.checked_mul(*columns)? {
                    return None;
                }
                let output_row = index / columns;
                let output_column = index % columns;
                let mut row_offset = 0;
                let mut column_offset = 0;
                for input in inputs {
                    let selected = match axis {
                        ConcatenationAxis::Horizontal => {
                            let selected = output_column < column_offset + input.columns;
                            if selected {
                                let local_column = output_column - column_offset;
                                let local_index = output_row
                                    .checked_mul(input.columns)?
                                    .checked_add(local_column)?;
                                return Some((input.source, local_index, input.elements()));
                            }
                            column_offset += input.columns;
                            selected
                        }
                        ConcatenationAxis::Vertical => {
                            let selected = output_row < row_offset + input.rows;
                            if selected {
                                let local_row = output_row - row_offset;
                                let local_index = local_row
                                    .checked_mul(input.columns)?
                                    .checked_add(output_column)?;
                                return Some((input.source, local_index, input.elements()));
                            }
                            row_offset += input.rows;
                            selected
                        }
                    };
                    debug_assert!(!selected);
                }
                None
            }
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ElementwiseIr {
    pub instructions: Box<[ElementwiseInstruction]>,
}

#[derive(Clone, Debug)]
pub enum ComputeKernel {
    Elementwise(ElementwiseIr),
    FixedShape(FixedShapeIr),
}

pub fn display_operation(operation: &OperationReference) -> String {
    operation
        .module_path
        .iter()
        .chain(std::iter::once(&operation.operation_name))
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("/")
}

pub fn elementwise_lowering(operation: &OperationReference) -> Option<ElementwiseLowering> {
    match display_operation(operation).as_str() {
        "math/add" => Some(ElementwiseLowering::Apply(ElementwiseOperation::Binary(
            BinaryOperation::Add,
        ))),
        "math/sub" => Some(ElementwiseLowering::Apply(ElementwiseOperation::Binary(
            BinaryOperation::Subtract,
        ))),
        "math/mul" => Some(ElementwiseLowering::Apply(ElementwiseOperation::Binary(
            BinaryOperation::Multiply,
        ))),
        "math/div" => Some(ElementwiseLowering::Apply(ElementwiseOperation::Binary(
            BinaryOperation::Divide,
        ))),
        "math/sin" => Some(ElementwiseLowering::Apply(ElementwiseOperation::Unary(
            UnaryOperation::Sin,
        ))),
        "math/cos" => Some(ElementwiseLowering::Apply(ElementwiseOperation::Unary(
            UnaryOperation::Cos,
        ))),
        "math/sqrt" => Some(ElementwiseLowering::Apply(ElementwiseOperation::Unary(
            UnaryOperation::Sqrt,
        ))),
        "math/ceil" => Some(ElementwiseLowering::Apply(ElementwiseOperation::Unary(
            UnaryOperation::Ceil,
        ))),
        "math/atan2" => Some(ElementwiseLowering::Apply(ElementwiseOperation::Atan2)),
        "matrix/horzcat" => Some(ElementwiseLowering::Concatenate(
            ConcatenationAxis::Horizontal,
        )),
        "matrix/vertcat" => Some(ElementwiseLowering::Concatenate(
            ConcatenationAxis::Vertical,
        )),
        _ => None,
    }
}

pub fn turn_required_nodes(artifact: &ProgramArtifact) -> BTreeSet<NodeId> {
    let mut required = BTreeSet::new();
    for node in artifact.nodes() {
        let writes_state = node.output_bindings.clone().any(|index| {
            matches!(
                artifact.bindings().get(index as usize),
                Some(BindingDeclaration::Output { target, .. })
                    if artifact.slots()[target.get() as usize].role == SlotRole::State
            )
        });
        if writes_state {
            visit_turn_node(artifact, node.node, &mut required);
        }
    }
    for output in artifact.outputs() {
        visit_turn_source(artifact, ArtifactSource::Slot(output.source), &mut required);
    }
    for constraint in artifact.constraints() {
        for source in &constraint.inputs {
            visit_turn_source(artifact, *source, &mut required);
        }
    }
    required
}

fn visit_turn_node(artifact: &ProgramArtifact, node: NodeId, required: &mut BTreeSet<NodeId>) {
    if !required.insert(node) {
        return;
    }
    let Some(declaration) = artifact.nodes().get(node.get() as usize) else {
        return;
    };
    for binding in declaration.input_bindings.clone() {
        if let Some(BindingDeclaration::Input { source, .. }) =
            artifact.bindings().get(binding as usize)
        {
            visit_turn_source(artifact, *source, required);
        }
    }
}

fn visit_turn_source(
    artifact: &ProgramArtifact,
    source: ArtifactSource,
    required: &mut BTreeSet<NodeId>,
) {
    let ArtifactSource::Slot(slot) = source else {
        return;
    };
    let Some(declaration) = artifact.slots().get(slot.get() as usize) else {
        return;
    };
    if declaration.role == SlotRole::State {
        return;
    }
    match declaration.producer {
        ProducerReference::Input(_) => {}
        ProducerReference::NodeOutput { node, .. } => visit_turn_node(artifact, node, required),
        ProducerReference::Output { source, .. } => visit_turn_source(artifact, source, required),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operation(name: &str) -> OperationReference {
        let mut segments = name.split('/').map(str::to_owned).collect::<Vec<_>>();
        let operation_name = segments.pop().unwrap();
        OperationReference {
            module_path: segments.into_boxed_slice(),
            operation_name,
        }
    }

    #[test]
    fn semantic_operations_select_portable_ir_instructions() {
        assert_eq!(
            elementwise_lowering(&operation("math/mul")),
            Some(ElementwiseLowering::Apply(ElementwiseOperation::Binary(
                BinaryOperation::Multiply
            )))
        );
        assert_eq!(
            elementwise_lowering(&operation("matrix/vertcat")),
            Some(ElementwiseLowering::Concatenate(
                ConcatenationAxis::Vertical
            ))
        );
        assert_eq!(
            elementwise_lowering(&operation("matrix/horzcat")),
            Some(ElementwiseLowering::Concatenate(
                ConcatenationAxis::Horizontal
            ))
        );
        assert_eq!(
            elementwise_lowering(&operation("math/ceil")),
            Some(ElementwiseLowering::Apply(ElementwiseOperation::Unary(
                UnaryOperation::Ceil
            )))
        );
        assert_eq!(
            elementwise_lowering(&operation("runtime/MulMDS<f32>")),
            None
        );
    }

    #[test]
    fn elementwise_ir_has_backend_independent_scalar_semantics() {
        let multiply = ElementwiseOperation::Binary(BinaryOperation::Multiply);
        assert_eq!(multiply.apply(&[6.0, 7.0]), 42.0);
        let ceil = ElementwiseOperation::Unary(UnaryOperation::Ceil);
        assert_eq!(ceil.apply(&[0.25]), 1.0);
        assert_eq!(ceil.apply(&[0.0]), 0.0);
        let concatenation = ElementwiseInstruction::Concatenate {
            axis: ConcatenationAxis::Horizontal,
            inputs: vec![
                ConcatenationInput {
                    source: ArtifactSource::Slot(CellSlotId::new(0)),
                    rows: 2,
                    columns: 1,
                },
                ConcatenationInput {
                    source: ArtifactSource::Slot(CellSlotId::new(1)),
                    rows: 2,
                    columns: 2,
                },
            ]
            .into_boxed_slice(),
            output: CellSlotId::new(2),
            rows: 2,
            columns: 3,
        };
        assert_eq!(concatenation.elements(), 6);
        assert_eq!(
            concatenation.concat_source_at(1),
            Some((ArtifactSource::Slot(CellSlotId::new(1)), 0, 4))
        );
        assert_eq!(
            concatenation.concat_source_at(3),
            Some((ArtifactSource::Slot(CellSlotId::new(0)), 1, 2))
        );
        assert_eq!(
            concatenation.concat_source_at(5),
            Some((ArtifactSource::Slot(CellSlotId::new(1)), 3, 4))
        );
        assert_eq!(concatenation.concat_source_at(6), None);
    }

    #[test]
    fn vertical_concatenation_uses_row_offsets() {
        let concatenation = ElementwiseInstruction::Concatenate {
            axis: ConcatenationAxis::Vertical,
            inputs: vec![
                ConcatenationInput {
                    source: ArtifactSource::Slot(CellSlotId::new(0)),
                    rows: 1,
                    columns: 2,
                },
                ConcatenationInput {
                    source: ArtifactSource::Slot(CellSlotId::new(1)),
                    rows: 2,
                    columns: 2,
                },
            ]
            .into_boxed_slice(),
            output: CellSlotId::new(2),
            rows: 3,
            columns: 2,
        };
        assert_eq!(
            concatenation.concat_source_at(2),
            Some((ArtifactSource::Slot(CellSlotId::new(1)), 0, 4))
        );
        assert_eq!(
            concatenation.concat_source_at(5),
            Some((ArtifactSource::Slot(CellSlotId::new(1)), 3, 4))
        );
    }
}
