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
}

impl UnaryOperation {
    pub fn apply(self, input: f32) -> f32 {
        match self {
            Self::Sin => input.sin(),
            Self::Cos => input.cos(),
            Self::Sqrt => input.sqrt(),
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
    Concat2,
}

impl ElementwiseLowering {
    pub const fn arity(self) -> usize {
        match self {
            Self::Apply(operation) => operation.arity(),
            Self::Concat2 => 2,
        }
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
    Concat2 {
        left: ArtifactSource,
        right: ArtifactSource,
        output: CellSlotId,
        left_elements: u64,
        right_elements: u64,
    },
}

impl ElementwiseInstruction {
    pub const fn output(&self) -> CellSlotId {
        match self {
            Self::Apply { output, .. } | Self::Concat2 { output, .. } => *output,
        }
    }

    pub fn elements(&self) -> u64 {
        match self {
            Self::Apply { elements, .. } => *elements,
            Self::Concat2 {
                left_elements,
                right_elements,
                ..
            } => left_elements
                .checked_add(*right_elements)
                .expect("validated concatenation element count"),
        }
    }

    pub fn concat_source_at(&self, index: u64) -> Option<(ArtifactSource, u64, u64)> {
        match self {
            Self::Apply { .. } => None,
            Self::Concat2 {
                left,
                right,
                left_elements,
                right_elements,
                ..
            } => {
                if index < *left_elements {
                    Some((*left, index, *left_elements))
                } else {
                    let right_index = index - *left_elements;
                    (right_index < *right_elements).then_some((
                        *right,
                        right_index,
                        *right_elements,
                    ))
                }
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
        "math/atan2" => Some(ElementwiseLowering::Apply(ElementwiseOperation::Atan2)),
        "matrix/horzcat" => Some(ElementwiseLowering::Apply(ElementwiseOperation::Identity)),
        "matrix/vertcat" => Some(ElementwiseLowering::Concat2),
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
            Some(ElementwiseLowering::Concat2)
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
        let concatenation = ElementwiseInstruction::Concat2 {
            left: ArtifactSource::Slot(CellSlotId::new(0)),
            right: ArtifactSource::Slot(CellSlotId::new(1)),
            output: CellSlotId::new(2),
            left_elements: 2,
            right_elements: 3,
        };
        assert_eq!(concatenation.elements(), 5);
        assert_eq!(
            concatenation.concat_source_at(1),
            Some((ArtifactSource::Slot(CellSlotId::new(0)), 1, 2))
        );
        assert_eq!(
            concatenation.concat_source_at(2),
            Some((ArtifactSource::Slot(CellSlotId::new(1)), 0, 3))
        );
        assert_eq!(
            concatenation.concat_source_at(4),
            Some((ArtifactSource::Slot(CellSlotId::new(1)), 2, 3))
        );
        assert_eq!(concatenation.concat_source_at(5), None);
    }

    #[test]
    fn equal_concatenation_still_uses_its_declared_split() {
        let concatenation = ElementwiseInstruction::Concat2 {
            left: ArtifactSource::Slot(CellSlotId::new(0)),
            right: ArtifactSource::Slot(CellSlotId::new(1)),
            output: CellSlotId::new(2),
            left_elements: 2,
            right_elements: 2,
        };
        assert_eq!(
            concatenation.concat_source_at(2),
            Some((ArtifactSource::Slot(CellSlotId::new(1)), 0, 2))
        );
    }
}
