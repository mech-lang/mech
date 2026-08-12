use mech_core::snapshot::SequenceView;
use mech_core::{CellSlotId, DimensionExpr, Value, ValueData};
use mech_engine::__resident::{
    ActivatedPlan, ReactiveInstance, ResidentStorageClass, ResidentValueBorrow,
};
use mech_engine::artifact::{ArtifactSource, BindingDeclaration, ProgramArtifact};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ElementType {
    F64,
    Index,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Shape {
    pub(super) rows: usize,
    pub(super) columns: usize,
}

impl Shape {
    pub(super) const SCALAR: Self = Self {
        rows: 1,
        columns: 1,
    };

    pub(super) fn len(self) -> usize {
        self.rows * self.columns
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ValueType {
    pub(super) element: ElementType,
    pub(super) shape: Shape,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct ValueId(u32);

impl ValueId {
    pub(super) fn get(self) -> u32 {
        self.0
    }
}

impl From<CellSlotId> for ValueId {
    fn from(value: CellSlotId) -> Self {
        Self(value.get())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ValueStorage {
    Temporary,
    Activation,
    State,
}

#[derive(Clone, Debug)]
pub(super) struct ValueDeclaration {
    pub(super) ty: ValueType,
    pub(super) storage: ValueStorage,
}

#[derive(Clone, Debug)]
pub(super) struct Constant {
    pub(super) ty: ValueType,
    pub(super) elements: Vec<u64>,
}

#[derive(Clone, Debug)]
pub(super) enum Source {
    Constant(Constant),
    Value(ValueId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum UnaryOperation {
    Sin,
    Cos,
    Negate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BinaryOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Operation {
    Input { ordinal: usize },
    Broadcast,
    HorizontalConcatenate,
    VerticalConcatenate,
    MatrixMultiply,
    Transpose,
    Dot,
    Assign,
    Unary(UnaryOperation),
    Atan2,
    Binary(BinaryOperation),
    MultiplyRows,
    SumColumns,
    Gather1D,
    RowsAllColumns,
    AddIndexedRows,
    SubtractIndexedRows,
}

#[derive(Clone, Debug)]
pub(super) struct Instruction {
    pub(super) node: u32,
    pub(super) operation_name: String,
    pub(super) operation: Operation,
    pub(super) inputs: Vec<Source>,
    pub(super) output: ValueId,
}

#[derive(Clone, Debug)]
pub(super) struct StateBinding {
    pub(super) value: ValueId,
    pub(super) offset: usize,
    pub(super) initial_elements: Vec<u64>,
}

#[derive(Clone, Debug)]
pub(super) struct ActivationBinding {
    pub(super) value: ValueId,
    pub(super) elements: Vec<u64>,
}

#[derive(Clone, Debug)]
pub(super) struct InputBinding {
    pub(super) value: ValueId,
    pub(super) ordinal: usize,
    pub(super) offset: usize,
    pub(super) len: usize,
    pub(super) per_lane: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BatchLayoutKind {
    MaterializedLaneVectors,
    OuterLift,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct BatchLayout {
    pub(super) len: usize,
    pub(super) kind: BatchLayoutKind,
}

#[derive(Clone, Debug)]
pub(super) struct KernelIr {
    pub(super) input_len: usize,
    pub(super) state_len: usize,
    pub(super) batch: Option<BatchLayout>,
    pub(super) values: BTreeMap<ValueId, ValueDeclaration>,
    pub(super) inputs: Vec<InputBinding>,
    pub(super) activations: Vec<ActivationBinding>,
    pub(super) states: Vec<StateBinding>,
    pub(super) instructions: Vec<Instruction>,
}

impl KernelIr {
    pub(super) fn lower(
        artifact: &ProgramArtifact,
        instance: &ReactiveInstance,
        input_slots: &[CellSlotId],
    ) -> Result<Self, KernelIrError> {
        let plan = &instance.plan;
        let activation = &instance.activation;
        let numeric = crate::analyze_activated_artifact(artifact, plan);
        let numeric_operations = numeric
            .regions
            .iter()
            .flat_map(|region| region.instructions.iter())
            .map(|instruction| (instruction.node, instruction.opcode))
            .collect::<BTreeMap<_, _>>();
        let turn_nodes = plan
            .nodes
            .iter()
            .map(|node| node.artifact_node.get())
            .collect::<BTreeSet<_>>();
        let rejection_by_node = numeric
            .rejections
            .iter()
            .map(|rejection| (rejection.node, rejection.reason.as_str()))
            .collect::<BTreeMap<_, _>>();
        let state_len = state_len(plan)?;
        let values = plan
            .slots
            .iter()
            .map(|slot| {
                let element = match slot.region.kind {
                    mech_core::ResidentValueKind::F64 => ElementType::F64,
                    mech_core::ResidentValueKind::Index => ElementType::Index,
                    mech_core::ResidentValueKind::Bool => {
                        return Err(KernelIrError::global(format!(
                            "slot {} has unsupported resident kind Bool",
                            slot.artifact_id.get(),
                        )));
                    }
                };
                Ok((
                    ValueId::from(slot.artifact_id),
                    ValueDeclaration {
                        ty: ValueType {
                            element,
                            shape: Shape {
                                rows: slot.region.shape.rows as usize,
                                columns: slot.region.shape.columns as usize,
                            },
                        },
                        storage: match slot.storage {
                            ResidentStorageClass::State => ValueStorage::State,
                            ResidentStorageClass::Constant => ValueStorage::Activation,
                            ResidentStorageClass::Input | ResidentStorageClass::Scratch => {
                                ValueStorage::Temporary
                            }
                        },
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        let mut input_len = 0usize;
        let inputs = input_slots
            .iter()
            .copied()
            .enumerate()
            .map(|(ordinal, slot)| {
                let value = ValueId::from(slot);
                let len = values
                    .get(&value)
                    .ok_or_else(|| {
                        KernelIrError::global(format!(
                            "input slot {} has no resolved resident type",
                            slot.get(),
                        ))
                    })?
                    .ty
                    .shape
                    .len();
                let offset = input_len;
                input_len = input_len
                    .checked_add(len)
                    .ok_or_else(|| KernelIrError::global("kernel input length overflow"))?;
                Ok(InputBinding {
                    value,
                    ordinal,
                    offset,
                    len,
                    per_lane: false,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let input_by_slot = inputs
            .iter()
            .map(|input| (CellSlotId::new(input.value.get()), input.ordinal))
            .collect::<BTreeMap<_, _>>();

        let activations = plan
            .slots
            .iter()
            .filter(|slot| slot.storage == ResidentStorageClass::Constant)
            .map(|slot| {
                let range = slot.region.offset..slot.region.offset + slot.region.len;
                let elements = match slot.region.kind {
                    mech_core::ResidentValueKind::F64 => activation.f64_storage()[range]
                        .iter()
                        .map(|value| value.to_bits())
                        .collect(),
                    mech_core::ResidentValueKind::Index => {
                        activation.index_storage()[range].to_vec()
                    }
                    mech_core::ResidentValueKind::Bool => {
                        return Err(KernelIrError::global(format!(
                            "activation slot {} has unsupported bool values",
                            slot.artifact_id.get(),
                        )));
                    }
                };
                Ok(ActivationBinding {
                    value: ValueId::from(slot.artifact_id),
                    elements,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let states = plan
            .slots
            .iter()
            .filter(|slot| slot.storage == ResidentStorageClass::State)
            .map(|slot| {
                let initial_elements = match instance.state_borrow(slot.artifact_id) {
                    Some(ResidentValueBorrow::F64 { values, .. }) => {
                        values.iter().map(|value| value.to_bits()).collect()
                    }
                    Some(ResidentValueBorrow::Index { values, .. }) => values.to_vec(),
                    Some(ResidentValueBorrow::Bool { .. }) => {
                        return Err(KernelIrError::global(format!(
                            "state slot {} has unsupported bool values",
                            slot.artifact_id.get(),
                        )));
                    }
                    None => {
                        return Err(KernelIrError::global(format!(
                            "state slot {} is not readable from the activated instance",
                            slot.artifact_id.get(),
                        )));
                    }
                };
                if initial_elements.len() != slot.region.len {
                    return Err(KernelIrError::global(format!(
                        "state slot {} initializer contains {} elements, expected {}",
                        slot.artifact_id.get(),
                        initial_elements.len(),
                        slot.region.len,
                    )));
                }
                Ok(StateBinding {
                    value: ValueId::from(slot.artifact_id),
                    offset: slot.region.offset,
                    initial_elements,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let activation_nodes = plan
            .activation_nodes
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let instructions = artifact
            .nodes()
            .iter()
            .filter(|node| !activation_nodes.contains(&node.node))
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
                } else if let Some(opcode) = numeric_operations.get(&node.node.get()) {
                    operation_from_numeric(*opcode)
                } else if turn_nodes.contains(&node.node.get()) {
                    return Err(KernelIrError::node(
                        node.node.get(),
                        operation_name.clone(),
                        rejection_by_node
                            .get(&node.node.get())
                            .copied()
                            .unwrap_or("turn node was not assigned to a native numeric region"),
                    ));
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

        let batch = infer_batch_layout(&values, &states, &instructions);
        let ir = Self {
            input_len,
            state_len,
            batch,
            values,
            inputs,
            activations,
            states,
            instructions,
        };
        ir.validate_input_ordinals()?;
        Ok(ir)
    }

    pub(super) fn value(&self, id: ValueId) -> &ValueDeclaration {
        &self.values[&id]
    }

    pub(super) fn state_is_written(&self, id: ValueId) -> bool {
        self.instructions.iter().any(|instruction| {
            instruction.output == id && self.value(id).storage == ValueStorage::State
        })
    }

    pub(super) fn input(&self, ordinal: usize) -> &InputBinding {
        &self.inputs[ordinal]
    }

    pub(super) fn lift_outer_batch(
        &mut self,
        len: usize,
        per_lane_inputs: &[usize],
    ) -> Result<(), KernelIrError> {
        if len == 0 {
            return Err(KernelIrError::global(
                "outer batch length must be greater than zero",
            ));
        }
        if self.batch.is_some() {
            return Err(KernelIrError::global(
                "cannot outer-lift a kernel that already has a batch layout",
            ));
        }
        let per_lane_inputs = per_lane_inputs.iter().copied().collect::<BTreeSet<_>>();
        if per_lane_inputs
            .iter()
            .any(|ordinal| *ordinal >= self.inputs.len())
        {
            return Err(KernelIrError::global(
                "outer batch references an unknown input ordinal",
            ));
        }

        let mut input_len = 0usize;
        for input in &mut self.inputs {
            input.offset = input_len;
            input.per_lane = per_lane_inputs.contains(&input.ordinal);
            let physical_len = if input.per_lane {
                input
                    .len
                    .checked_mul(len)
                    .ok_or_else(|| KernelIrError::global("outer batch input length overflow"))?
            } else {
                input.len
            };
            input_len = input_len
                .checked_add(physical_len)
                .ok_or_else(|| KernelIrError::global("outer batch input length overflow"))?;
        }

        let mut state_len = 0usize;
        for state in &mut self.states {
            state.offset = state_len;
            state_len =
                state_len
                    .checked_add(state.initial_elements.len().checked_mul(len).ok_or_else(
                        || KernelIrError::global("outer batch state length overflow"),
                    )?)
                    .ok_or_else(|| KernelIrError::global("outer batch state length overflow"))?;
        }

        self.input_len = input_len;
        self.state_len = state_len;
        self.batch = Some(BatchLayout {
            len,
            kind: BatchLayoutKind::OuterLift,
        });
        Ok(())
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
        let expected = (0..self.inputs.len()).collect::<BTreeSet<_>>();
        if ordinals != expected {
            return Err(KernelIrError::global(format!(
                "kernel input ordinals are {ordinals:?}, expected {expected:?}",
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(super) struct KernelIrError {
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
    } else if name.starts_with("ConvertScalarToMat2") {
        Some(Operation::Broadcast)
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

fn operation_from_numeric(opcode: crate::NativeNumericOpcode) -> Operation {
    use crate::NativeNumericOpcode;

    match opcode {
        NativeNumericOpcode::Broadcast => Operation::Broadcast,
        NativeNumericOpcode::HorizontalConcatenate => Operation::HorizontalConcatenate,
        NativeNumericOpcode::VerticalConcatenate => Operation::VerticalConcatenate,
        NativeNumericOpcode::MatrixMultiply => Operation::MatrixMultiply,
        NativeNumericOpcode::Transpose => Operation::Transpose,
        NativeNumericOpcode::Dot => Operation::Dot,
        NativeNumericOpcode::Assign => Operation::Assign,
        NativeNumericOpcode::Sin => Operation::Unary(UnaryOperation::Sin),
        NativeNumericOpcode::Cos => Operation::Unary(UnaryOperation::Cos),
        NativeNumericOpcode::Negate => Operation::Unary(UnaryOperation::Negate),
        NativeNumericOpcode::Atan2 => Operation::Atan2,
        NativeNumericOpcode::Add => Operation::Binary(BinaryOperation::Add),
        NativeNumericOpcode::Subtract => Operation::Binary(BinaryOperation::Subtract),
        NativeNumericOpcode::Multiply => Operation::Binary(BinaryOperation::Multiply),
        NativeNumericOpcode::MultiplyRows => Operation::MultiplyRows,
        NativeNumericOpcode::Divide => Operation::Binary(BinaryOperation::Divide),
        NativeNumericOpcode::Power => Operation::Binary(BinaryOperation::Power),
        NativeNumericOpcode::SumColumns => Operation::SumColumns,
        NativeNumericOpcode::Gather1D => Operation::Gather1D,
        NativeNumericOpcode::RowsAllColumns => Operation::RowsAllColumns,
        NativeNumericOpcode::AddAssign => Operation::Binary(BinaryOperation::Add),
        NativeNumericOpcode::AddIndexedRows => Operation::AddIndexedRows,
        NativeNumericOpcode::SubtractIndexedRows => Operation::SubtractIndexedRows,
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
            element: value_element(value)?,
            shape: value_shape(artifact, value)?,
        },
        elements: value_elements(value)?,
    })
}

fn value_element(value: &Value) -> Result<ElementType, String> {
    match value.data() {
        ValueData::F64(_) => Ok(ElementType::F64),
        ValueData::Index(_) => Ok(ElementType::Index),
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::F64(_) => Ok(ElementType::F64),
            SequenceView::Index(_) => Ok(ElementType::Index),
            other => Err(format!(
                "matrix constant elements {other:?} are not numeric"
            )),
        },
        other => Err(format!("constant {other:?} is not numeric")),
    }
}

fn value_shape(artifact: &ProgramArtifact, value: &Value) -> Result<Shape, String> {
    let schema = artifact
        .schemas()
        .entry(value.schema())
        .ok_or_else(|| "constant schema is missing".to_string())?
        .schema();
    match schema.body() {
        mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
        | mech_core::SchemaBody::Index => Ok(Shape::SCALAR),
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
        ValueData::Index(value) => Ok(vec![*value]),
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::F64(values) => Ok(values.iter().map(|value| value.bits()).collect()),
            SequenceView::Index(values) => Ok(values.to_vec()),
            other => Err(format!(
                "matrix constant elements {other:?} are not numeric"
            )),
        },
        other => Err(format!("constant {other:?} is not numeric")),
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
    let shape = output.shape;
    match instruction.operation {
        Operation::Input { .. } => {
            require_arity(&inputs, 0)?;
        }
        Operation::Broadcast => {
            require_arity(&inputs, 1)?;
            if inputs[0].shape != Shape::SCALAR {
                return Err("broadcast currently requires a scalar input".to_string());
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
            require_f64(&inputs)?;
            for input in inputs {
                if input.shape != Shape::SCALAR && input.shape != shape {
                    return Err("atan2 operands must match the output or be scalar".to_string());
                }
            }
        }
        Operation::Binary(_) => {
            require_arity(&inputs, 2)?;
            require_f64(&inputs)?;
            for input in inputs {
                if input.shape != Shape::SCALAR && input.shape != shape {
                    return Err("binary operands must match the output or be scalar".to_string());
                }
            }
        }
        Operation::MultiplyRows => {
            require_arity(&inputs, 2)?;
            require_f64(&inputs)?;
            if inputs[0].shape != shape
                || inputs[1].shape.len() != shape.rows
                || inputs[1].shape.columns != 1
            {
                return Err("row-wise multiplication shape mismatch".to_string());
            }
        }
        Operation::SumColumns => {
            require_arity(&inputs, 1)?;
            require_f64(&inputs)?;
            if shape.len() != inputs[0].shape.rows {
                return Err("column reduction shape mismatch".to_string());
            }
        }
        Operation::Gather1D => {
            require_arity(&inputs, 2)?;
            require_selection_types(output, &inputs)?;
            if shape.len() != inputs[1].shape.len() {
                return Err("one-dimensional gather shape mismatch".to_string());
            }
        }
        Operation::RowsAllColumns => {
            require_arity(&inputs, 2)?;
            require_selection_types(output, &inputs)?;
            if shape.rows != inputs[1].shape.len() || shape.columns != inputs[0].shape.columns {
                return Err("row selection shape mismatch".to_string());
            }
        }
        Operation::AddIndexedRows | Operation::SubtractIndexedRows => {
            require_arity(&inputs, 3)?;
            if inputs[0].element != ElementType::F64
                || inputs[1].element != ElementType::F64
                || inputs[2].element != ElementType::Index
                || inputs[0].shape != shape
                || inputs[1].shape.rows != inputs[2].shape.len()
                || inputs[1].shape.columns != shape.columns
            {
                return Err("indexed row update type or shape mismatch".to_string());
            }
        }
    }
    Ok(())
}

fn require_f64(inputs: &[ValueType]) -> Result<(), String> {
    if inputs.iter().any(|input| input.element != ElementType::F64) {
        return Err("one or more inputs are not f64".to_string());
    }
    Ok(())
}

fn require_selection_types(output: ValueType, inputs: &[ValueType]) -> Result<(), String> {
    if output.element != ElementType::F64
        || inputs[0].element != ElementType::F64
        || inputs[1].element != ElementType::Index
    {
        return Err("selection requires f64 data and Index selectors".to_string());
    }
    Ok(())
}

fn infer_batch_layout(
    values: &BTreeMap<ValueId, ValueDeclaration>,
    states: &[StateBinding],
    instructions: &[Instruction],
) -> Option<BatchLayout> {
    let first_state = states.first()?;
    let state_shape = values.get(&first_state.value)?.ty.shape;
    if state_shape.rows != 1 || state_shape.columns <= 1 {
        return None;
    }
    let len = state_shape.columns;
    let shape_is_lane = |shape: Shape| shape == Shape::SCALAR || shape == state_shape;
    if !values.values().all(|value| shape_is_lane(value.ty.shape))
        || !states
            .iter()
            .all(|state| values[&state.value].ty.shape == state_shape)
        || !instructions.iter().all(|instruction| {
            matches!(
                instruction.operation,
                Operation::Input { .. }
                    | Operation::Broadcast
                    | Operation::Assign
                    | Operation::Unary(_)
                    | Operation::Atan2
                    | Operation::Binary(_)
            )
        })
    {
        return None;
    }
    for instruction in instructions {
        for source in &instruction.inputs {
            let Source::Constant(constant) = source else {
                continue;
            };
            if constant.ty.shape != Shape::SCALAR
                && (constant.ty.shape != state_shape
                    || constant.elements.windows(2).any(|pair| pair[0] != pair[1]))
            {
                return None;
            }
        }
    }
    Some(BatchLayout {
        len,
        kind: BatchLayoutKind::MaterializedLaneVectors,
    })
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
