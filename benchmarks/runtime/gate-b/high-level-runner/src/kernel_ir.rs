use mech_core::snapshot::SequenceView;
use mech_core::{CellSlotId, DimensionExpr, Value, ValueData};
use mech_engine::__resident::{ActivatedPlan, ResidentStorageClass};
use mech_engine::artifact::{
    ArtifactSource, BindingDeclaration, InitializerReference, ProgramArtifact,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ElementType {
    F64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Shape {
    pub(crate) rows: usize,
    pub(crate) columns: usize,
}

impl Shape {
    pub(crate) const SCALAR: Self = Self {
        rows: 1,
        columns: 1,
    };

    pub(crate) fn len(self) -> usize {
        self.rows * self.columns
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ValueType {
    pub(crate) element: ElementType,
    pub(crate) shape: Shape,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ValueId(u32);

impl ValueId {
    pub(crate) fn get(self) -> u32 {
        self.0
    }
}

impl From<CellSlotId> for ValueId {
    fn from(value: CellSlotId) -> Self {
        Self(value.get())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ValueStorage {
    Temporary,
    State,
}

#[derive(Clone, Debug)]
pub(crate) struct ValueDeclaration {
    pub(crate) ty: ValueType,
    pub(crate) storage: ValueStorage,
}

#[derive(Clone, Debug)]
pub(crate) struct Constant {
    pub(crate) ty: ValueType,
    pub(crate) elements: Vec<u64>,
}

#[derive(Clone, Debug)]
pub(crate) enum Source {
    Constant(Constant),
    Value(ValueId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnaryOperation {
    Sin,
    Cos,
    Negate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BinaryOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Operation {
    Input { ordinal: usize },
    HorizontalConcatenate,
    VerticalConcatenate,
    MatrixMultiply,
    Transpose,
    Dot,
    Assign,
    Unary(UnaryOperation),
    Atan2,
    Binary(BinaryOperation),
}

#[derive(Clone, Debug)]
pub(crate) struct Instruction {
    pub(crate) node: u32,
    pub(crate) operation_name: String,
    pub(crate) operation: Operation,
    pub(crate) inputs: Vec<Source>,
    pub(crate) output: ValueId,
}

#[derive(Clone, Debug)]
pub(crate) struct StateBinding {
    pub(crate) value: ValueId,
    pub(crate) offset: usize,
    pub(crate) initial_elements: Vec<u64>,
}

#[derive(Clone, Debug)]
pub(crate) struct KernelIr {
    pub(crate) input_count: usize,
    pub(crate) state_len: usize,
    pub(crate) values: BTreeMap<ValueId, ValueDeclaration>,
    pub(crate) states: Vec<StateBinding>,
    pub(crate) instructions: Vec<Instruction>,
}

impl KernelIr {
    pub(crate) fn lower(
        artifact: &ProgramArtifact,
        plan: &ActivatedPlan,
        input_slots: &[CellSlotId],
    ) -> Result<Self, KernelIrError> {
        let state_len = state_len(plan)?;
        let input_by_slot = input_slots
            .iter()
            .copied()
            .enumerate()
            .map(|(ordinal, slot)| (slot, ordinal))
            .collect::<BTreeMap<_, _>>();
        let values = plan
            .slots
            .iter()
            .map(|slot| {
                if slot.region.kind != mech_core::ResidentValueKind::F64 {
                    return Err(KernelIrError::global(format!(
                        "slot {} has unsupported resident kind {:?}",
                        slot.artifact_id.get(),
                        slot.region.kind,
                    )));
                }
                Ok((
                    ValueId::from(slot.artifact_id),
                    ValueDeclaration {
                        ty: ValueType {
                            element: ElementType::F64,
                            shape: Shape {
                                rows: slot.region.shape.rows as usize,
                                columns: slot.region.shape.columns as usize,
                            },
                        },
                        storage: if slot.storage == ResidentStorageClass::State {
                            ValueStorage::State
                        } else {
                            ValueStorage::Temporary
                        },
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        let states = plan
            .slots
            .iter()
            .filter(|slot| slot.storage == ResidentStorageClass::State)
            .map(|slot| {
                let declaration = artifact
                    .slots()
                    .get(slot.artifact_id.get() as usize)
                    .ok_or_else(|| {
                        KernelIrError::global(format!(
                            "missing declaration for state slot {}",
                            slot.artifact_id.get(),
                        ))
                    })?;
                let Some(InitializerReference::Constant(constant_id)) = declaration.initializer
                else {
                    return Err(KernelIrError::global(format!(
                        "state slot {} has no constant initializer",
                        slot.artifact_id.get(),
                    )));
                };
                let constant = constant(artifact, constant_id).map_err(KernelIrError::global)?;
                if constant.elements.len() != slot.region.len {
                    return Err(KernelIrError::global(format!(
                        "state slot {} initializer contains {} elements, expected {}",
                        slot.artifact_id.get(),
                        constant.elements.len(),
                        slot.region.len,
                    )));
                }
                Ok(StateBinding {
                    value: ValueId::from(slot.artifact_id),
                    offset: slot.region.offset,
                    initial_elements: constant.elements,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let instructions = artifact
            .nodes()
            .iter()
            .map(|node| {
                let operation_name = qualified_operation_name(node);
                let output = node_output(artifact, node).map_err(|reason| {
                    KernelIrError::node(node.node.get(), operation_name.clone(), reason)
                })?;
                let output_id = ValueId::from(output);
                let output_type =
                    values
                        .get(&output_id)
                        .map(|value| value.ty)
                        .ok_or_else(|| {
                            KernelIrError::node(
                                node.node.get(),
                                operation_name.clone(),
                                format!(
                                    "output slot {} has no resolved resident type",
                                    output.get()
                                ),
                            )
                        })?;

                let operation = if node.operation.module_path.as_ref() == ["resource", "read"] {
                    Operation::Input {
                        ordinal: *input_by_slot.get(&output).ok_or_else(|| {
                            KernelIrError::node(
                                node.node.get(),
                                operation_name.clone(),
                                "resource output is not one of the bound kernel inputs",
                            )
                        })?,
                    }
                } else {
                    lower_operation(&node.operation.operation_name).ok_or_else(|| {
                        KernelIrError::node(
                            node.node.get(),
                            operation_name.clone(),
                            "operation has no native numeric lowering",
                        )
                    })?
                };
                let inputs = if matches!(operation, Operation::Input { .. }) {
                    Vec::new()
                } else {
                    node_inputs(artifact, node).map_err(|reason| {
                        KernelIrError::node(node.node.get(), operation_name.clone(), reason)
                    })?
                };
                let instruction = Instruction {
                    node: node.node.get(),
                    operation_name,
                    operation,
                    inputs,
                    output: output_id,
                };
                validate_instruction(&instruction, output_type, &values).map_err(|reason| {
                    KernelIrError::node(
                        instruction.node,
                        instruction.operation_name.clone(),
                        reason,
                    )
                })?;
                Ok(instruction)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let ir = Self {
            input_count: input_slots.len(),
            state_len,
            values,
            states,
            instructions,
        };
        ir.validate_input_ordinals()?;
        Ok(ir)
    }

    pub(crate) fn value(&self, id: ValueId) -> &ValueDeclaration {
        &self.values[&id]
    }

    pub(crate) fn state_is_written(&self, id: ValueId) -> bool {
        self.instructions.iter().any(|instruction| {
            instruction.output == id && self.value(id).storage == ValueStorage::State
        })
    }

    fn validate_input_ordinals(&self) -> Result<(), KernelIrError> {
        let ordinals = self
            .instructions
            .iter()
            .filter_map(|instruction| match instruction.operation {
                Operation::Input { ordinal } => Some(ordinal),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let expected = (0..self.input_count).collect::<BTreeSet<_>>();
        if ordinals != expected {
            return Err(KernelIrError::global(format!(
                "kernel input ordinals are {ordinals:?}, expected {expected:?}",
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct KernelIrError {
    node: Option<u32>,
    operation: Option<String>,
    reason: String,
}

impl KernelIrError {
    fn global(reason: impl Into<String>) -> Self {
        Self {
            node: None,
            operation: None,
            reason: reason.into(),
        }
    }

    fn node(node: u32, operation: String, reason: impl Into<String>) -> Self {
        Self {
            node: Some(node),
            operation: Some(operation),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for KernelIrError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.node, self.operation.as_deref()) {
            (Some(node), Some(operation)) => write!(
                formatter,
                "node {node} ({operation}) is not eligible for a native numeric kernel: {}",
                self.reason,
            ),
            _ => write!(
                formatter,
                "program is not eligible for a native numeric kernel: {}",
                self.reason,
            ),
        }
    }
}

fn qualified_operation_name(node: &mech_engine::artifact::NodeDeclaration) -> String {
    if node.operation.module_path.is_empty() {
        node.operation.operation_name.clone()
    } else {
        format!(
            "{}/{}",
            node.operation.module_path.join("/"),
            node.operation.operation_name,
        )
    }
}

fn lower_operation(name: &str) -> Option<Operation> {
    if name.starts_with("HorizontalConcatenate") {
        Some(Operation::HorizontalConcatenate)
    } else if name.starts_with("VerticalConcatenate") {
        Some(Operation::VerticalConcatenate)
    } else if name.starts_with("MatMul") {
        Some(Operation::MatrixMultiply)
    } else if name.starts_with("Transpose") {
        Some(Operation::Transpose)
    } else if name.starts_with("Dot") {
        Some(Operation::Dot)
    } else if name.starts_with("Assign") {
        Some(Operation::Assign)
    } else if name.starts_with("MathSin") {
        Some(Operation::Unary(UnaryOperation::Sin))
    } else if name.starts_with("MathCos") {
        Some(Operation::Unary(UnaryOperation::Cos))
    } else if name.starts_with("Negate") {
        Some(Operation::Unary(UnaryOperation::Negate))
    } else if name.starts_with("Atan2") {
        Some(Operation::Atan2)
    } else if name.starts_with("Add") {
        Some(Operation::Binary(BinaryOperation::Add))
    } else if name.starts_with("Sub") {
        Some(Operation::Binary(BinaryOperation::Subtract))
    } else if name.starts_with("Mul") {
        Some(Operation::Binary(BinaryOperation::Multiply))
    } else if name.starts_with("Div") {
        Some(Operation::Binary(BinaryOperation::Divide))
    } else {
        None
    }
}

fn state_len(plan: &ActivatedPlan) -> Result<usize, KernelIrError> {
    plan.slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::State)
        .try_fold(0usize, |len, slot| {
            if slot.region.kind != mech_core::ResidentValueKind::F64 {
                return Err(KernelIrError::global(format!(
                    "state slot {} is {:?}, not f64",
                    slot.artifact_id.get(),
                    slot.region.kind,
                )));
            }
            Ok(len.max(slot.region.offset + slot.region.len))
        })
}

fn node_output(
    artifact: &ProgramArtifact,
    node: &mech_engine::artifact::NodeDeclaration,
) -> Result<CellSlotId, String> {
    let outputs = &artifact.bindings()
        [node.output_bindings.start as usize..node.output_bindings.end as usize];
    let [BindingDeclaration::Output { target, .. }] = outputs else {
        return Err("native numeric nodes must have exactly one output".to_string());
    };
    Ok(*target)
}

fn node_inputs(
    artifact: &ProgramArtifact,
    node: &mech_engine::artifact::NodeDeclaration,
) -> Result<Vec<Source>, String> {
    let mut inputs = artifact.bindings()
        [node.input_bindings.start as usize..node.input_bindings.end as usize]
        .iter()
        .map(|binding| match binding {
            BindingDeclaration::Input {
                port_ordinal,
                source,
                ..
            } => Ok((*port_ordinal, source_from_artifact(artifact, *source)?)),
            BindingDeclaration::Output { .. } => {
                Err("output binding appeared in the input range".to_string())
            }
        })
        .collect::<Result<Vec<_>, String>>()?;
    inputs.sort_by_key(|(ordinal, _)| *ordinal);
    Ok(inputs.into_iter().map(|(_, source)| source).collect())
}

fn source_from_artifact(
    artifact: &ProgramArtifact,
    source: ArtifactSource,
) -> Result<Source, String> {
    match source {
        ArtifactSource::Constant(id) => Ok(Source::Constant(constant(artifact, id)?)),
        ArtifactSource::Slot(slot) => Ok(Source::Value(ValueId::from(slot))),
    }
}

fn constant(artifact: &ProgramArtifact, id: mech_core::ConstantId) -> Result<Constant, String> {
    let value = artifact
        .constants()
        .get(id)
        .ok_or_else(|| format!("missing constant {}", id.get()))?;
    Ok(Constant {
        ty: ValueType {
            element: ElementType::F64,
            shape: value_shape(artifact, value)?,
        },
        elements: value_elements(value)?,
    })
}

fn value_shape(artifact: &ProgramArtifact, value: &Value) -> Result<Shape, String> {
    let schema = artifact
        .schemas()
        .entry(value.schema())
        .ok_or_else(|| "constant schema is missing".to_string())?
        .schema();
    match schema.body() {
        mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => Ok(Shape::SCALAR),
        mech_core::SchemaBody::Matrix { dimensions, .. } if dimensions.len() == 2 => Ok(Shape {
            rows: evaluate_dimension(&dimensions[0], value.shape().parameter_values())? as usize,
            columns: evaluate_dimension(&dimensions[1], value.shape().parameter_values())? as usize,
        }),
        body => Err(format!("constant schema {body:?} is not fixed-shape f64")),
    }
}

fn evaluate_dimension(expression: &DimensionExpr, values: &[u64]) -> Result<u64, String> {
    match expression {
        DimensionExpr::Hole => Err("constant contains an unresolved dimension".to_string()),
        DimensionExpr::Constant(value) => Ok(*value),
        DimensionExpr::Parameter(id) => values
            .get(id.get() as usize)
            .copied()
            .ok_or_else(|| "constant dimension parameter is missing".to_string()),
        DimensionExpr::Add(terms) => terms.iter().try_fold(0_u64, |sum, term| {
            sum.checked_add(evaluate_dimension(term, values)?)
                .ok_or_else(|| "constant dimension overflow".to_string())
        }),
        DimensionExpr::Multiply(terms) => terms.iter().try_fold(1_u64, |product, term| {
            product
                .checked_mul(evaluate_dimension(term, values)?)
                .ok_or_else(|| "constant dimension overflow".to_string())
        }),
        DimensionExpr::Min(terms) => terms
            .iter()
            .map(|term| evaluate_dimension(term, values))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .min()
            .ok_or_else(|| "constant has an empty minimum dimension".to_string()),
        DimensionExpr::Max(terms) => terms
            .iter()
            .map(|term| evaluate_dimension(term, values))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .ok_or_else(|| "constant has an empty maximum dimension".to_string()),
    }
}

fn value_elements(value: &Value) -> Result<Vec<u64>, String> {
    match value.data() {
        ValueData::F64(value) => Ok(vec![value.bits()]),
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::F64(values) => Ok(values.iter().map(|value| value.bits()).collect()),
            other => Err(format!("matrix constant elements {other:?} are not f64")),
        },
        other => Err(format!("constant {other:?} is not f64")),
    }
}

fn source_type(
    source: &Source,
    values: &BTreeMap<ValueId, ValueDeclaration>,
) -> Result<ValueType, String> {
    match source {
        Source::Constant(constant) => Ok(constant.ty),
        Source::Value(id) => values
            .get(id)
            .map(|value| value.ty)
            .ok_or_else(|| format!("input slot {} has no resolved resident type", id.get())),
    }
}

fn validate_instruction(
    instruction: &Instruction,
    output: ValueType,
    values: &BTreeMap<ValueId, ValueDeclaration>,
) -> Result<(), String> {
    if output.element != ElementType::F64 {
        return Err("output element type is not f64".to_string());
    }
    let inputs = instruction
        .inputs
        .iter()
        .map(|source| source_type(source, values))
        .collect::<Result<Vec<_>, _>>()?;
    if inputs.iter().any(|input| input.element != ElementType::F64) {
        return Err("one or more input element types are not f64".to_string());
    }
    let shape = output.shape;
    match instruction.operation {
        Operation::Input { .. } => {
            require_arity(&inputs, 0)?;
            if shape != Shape::SCALAR {
                return Err("resource inputs currently require a scalar output".to_string());
            }
        }
        Operation::HorizontalConcatenate => {
            if inputs.is_empty()
                || inputs.iter().any(|input| input.shape.rows != shape.rows)
                || inputs
                    .iter()
                    .map(|input| input.shape.columns)
                    .sum::<usize>()
                    != shape.columns
            {
                return Err("horizontal concatenation shape mismatch".to_string());
            }
        }
        Operation::VerticalConcatenate => {
            if inputs.is_empty()
                || inputs
                    .iter()
                    .any(|input| input.shape.columns != shape.columns)
                || inputs.iter().map(|input| input.shape.rows).sum::<usize>() != shape.rows
            {
                return Err("vertical concatenation shape mismatch".to_string());
            }
        }
        Operation::MatrixMultiply => {
            require_arity(&inputs, 2)?;
            if inputs[0].shape.columns != inputs[1].shape.rows
                || shape.rows != inputs[0].shape.rows
                || shape.columns != inputs[1].shape.columns
            {
                return Err("matrix multiplication shape mismatch".to_string());
            }
        }
        Operation::Transpose => {
            require_arity(&inputs, 1)?;
            if shape.rows != inputs[0].shape.columns || shape.columns != inputs[0].shape.rows {
                return Err("transpose shape mismatch".to_string());
            }
        }
        Operation::Dot => {
            require_arity(&inputs, 2)?;
            if inputs[0].shape.len() != inputs[1].shape.len() || shape != Shape::SCALAR {
                return Err("dot-product shape mismatch".to_string());
            }
        }
        Operation::Assign => {
            require_arity(&inputs, 1)?;
            if inputs[0] != output {
                return Err("assignment input and output types differ".to_string());
            }
        }
        Operation::Unary(_) => {
            require_arity(&inputs, 1)?;
            if inputs[0] != output {
                return Err("unary input and output types differ".to_string());
            }
        }
        Operation::Atan2 => {
            require_arity(&inputs, 2)?;
            if inputs.iter().any(|input| input.shape != Shape::SCALAR) || shape != Shape::SCALAR {
                return Err("atan2 currently requires scalar inputs and output".to_string());
            }
        }
        Operation::Binary(_) => {
            require_arity(&inputs, 2)?;
            for input in inputs {
                if input.shape != Shape::SCALAR && input.shape != shape {
                    return Err("binary operands must match the output or be scalar".to_string());
                }
            }
        }
    }
    Ok(())
}

fn require_arity(inputs: &[ValueType], expected: usize) -> Result<(), String> {
    if inputs.len() != expected {
        return Err(format!(
            "operation has {} inputs, expected {expected}",
            inputs.len(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_operation_diagnostic_identifies_node_and_operation() {
        let error = KernelIrError::node(
            17,
            "example/Unsupported".to_string(),
            "operation has no native numeric lowering",
        );
        assert_eq!(
            error.to_string(),
            "node 17 (example/Unsupported) is not eligible for a native numeric kernel: operation has no native numeric lowering",
        );
    }
}
