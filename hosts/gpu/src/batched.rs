use std::collections::{BTreeMap, BTreeSet};

use mech_core::{
    CellSlotId, DimensionExpr, FloatWidth, NodeId, SchemaBody, ValueData, snapshot::SequenceView,
};
use mech_engine::{
    ArtifactSource, BindingDeclaration, ProducerReference, ProgramArtifact, SlotRole,
};

use super::{
    ElementwiseOperation, GpuAdmissionError, GpuDiagnostic, GpuDiagnosticCode, WORKGROUP_SIZE,
    display_operation, turn_required_nodes,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FixedShape {
    rows: usize,
    columns: usize,
}

impl FixedShape {
    const fn scalar() -> Self {
        Self {
            rows: 1,
            columns: 1,
        }
    }

    const fn elements(self) -> usize {
        self.rows * self.columns
    }

    const fn index(self, row: usize, column: usize) -> usize {
        row + column * self.rows
    }
}

#[derive(Clone, Copy, Debug)]
enum ScalarOperand {
    Register(usize),
    Constant(f32),
}

impl ScalarOperand {
    fn evaluate(self, registers: &[f32]) -> f32 {
        match self {
            Self::Register(register) => registers[register],
            Self::Constant(value) => value,
        }
    }

    fn wgsl(self) -> String {
        match self {
            Self::Register(register) => format!("r{register}"),
            Self::Constant(value) => super::format_wgsl_f32(value),
        }
    }
}

#[derive(Clone, Debug)]
enum ScalarComputation {
    Copy(ScalarOperand),
    Negate(ScalarOperand),
    Elementwise {
        operation: ElementwiseOperation,
        inputs: Vec<ScalarOperand>,
    },
    SumProducts(Vec<(ScalarOperand, ScalarOperand)>),
}

impl ScalarComputation {
    fn evaluate(&self, registers: &[f32]) -> f32 {
        match self {
            Self::Copy(input) => input.evaluate(registers),
            Self::Negate(input) => -input.evaluate(registers),
            Self::Elementwise { operation, inputs } => {
                let mut values = [0.0_f32; 2];
                for (index, input) in inputs.iter().enumerate() {
                    values[index] = input.evaluate(registers);
                }
                operation.apply(&values[..inputs.len()], 0, 1)
            }
            Self::SumProducts(terms) => terms.iter().fold(0.0, |sum, (left, right)| {
                left.evaluate(registers)
                    .mul_add(right.evaluate(registers), sum)
            }),
        }
    }

    fn wgsl(&self) -> String {
        match self {
            Self::Copy(input) => input.wgsl(),
            Self::Negate(input) => format!("-({})", input.wgsl()),
            Self::Elementwise { operation, inputs } => {
                let inputs = inputs.iter().map(|input| input.wgsl()).collect::<Vec<_>>();
                operation.wgsl_expression(&inputs, 1)
            }
            Self::SumProducts(terms) => terms
                .iter()
                .map(|(left, right)| format!("({} * {})", left.wgsl(), right.wgsl()))
                .collect::<Vec<_>>()
                .join(" + "),
        }
    }
}

#[derive(Clone, Debug)]
struct ScalarInstruction {
    output: usize,
    computation: ScalarComputation,
}

#[derive(Clone, Debug)]
struct BatchedInput {
    slot: CellSlotId,
    name: String,
    shape: FixedShape,
    binding: u32,
}

#[derive(Clone, Debug)]
struct BatchedState {
    slot: CellSlotId,
    shape: FixedShape,
    initializer: Vec<f32>,
    update: Vec<ScalarOperand>,
    read_binding: u32,
    write_binding: u32,
}

/// A generic fixed-shape numeric artifact scalarized once and mapped over an
/// outer batch. Each GPU invocation executes one complete source program.
#[derive(Clone, Debug)]
pub struct BatchedGpuProgram {
    instances: u32,
    register_count: usize,
    register_offsets: BTreeMap<CellSlotId, usize>,
    instructions: Vec<ScalarInstruction>,
    inputs: Vec<BatchedInput>,
    states: Vec<BatchedState>,
    wgsl: String,
}

#[derive(Debug)]
pub struct BatchedCpuSession<'a> {
    program: &'a BatchedGpuProgram,
    inputs: BTreeMap<CellSlotId, Vec<f32>>,
    state: BTreeMap<CellSlotId, Vec<f32>>,
    next_state: BTreeMap<CellSlotId, Vec<f32>>,
    registers: Vec<f32>,
}

impl super::GpuHost {
    /// Scalarizes generic fixed-shape f32 math and matrix operations, then maps
    /// the resulting kernel over `instances` independent program states.
    pub fn compile_batched(
        &self,
        artifact: &ProgramArtifact,
        instances: u32,
    ) -> Result<BatchedGpuProgram, GpuAdmissionError> {
        BatchCompiler::new(artifact, instances).compile()
    }
}

impl BatchedGpuProgram {
    pub const fn instances(&self) -> u32 {
        self.instances
    }

    pub const fn workgroup_count(&self) -> u32 {
        self.instances.div_ceil(WORKGROUP_SIZE)
    }

    pub fn wgsl(&self) -> &str {
        &self.wgsl
    }

    pub fn inputs(&self) -> impl Iterator<Item = (&str, usize)> {
        self.inputs
            .iter()
            .map(|input| (input.name.as_str(), input.shape.elements()))
    }

    pub fn state_layout(&self) -> impl Iterator<Item = (CellSlotId, usize)> + '_ {
        self.states
            .iter()
            .map(|state| (state.slot, state.shape.elements()))
    }

    pub fn prepare_cpu(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedCpuSession<'_>, BatchedExecutionError> {
        let inputs = self.expand_inputs(inputs)?;
        let state = self.initial_state();
        let next_state = state
            .iter()
            .map(|(slot, values)| (*slot, vec![0.0; values.len()]))
            .collect();
        Ok(BatchedCpuSession {
            program: self,
            inputs,
            state,
            next_state,
            registers: vec![0.0; self.register_count],
        })
    }

    fn expand_inputs(
        &self,
        provided: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BTreeMap<CellSlotId, Vec<f32>>, BatchedExecutionError> {
        self.inputs
            .iter()
            .map(|input| {
                let values = provided
                    .get(&input.name)
                    .ok_or_else(|| BatchedExecutionError::MissingInput(input.name.clone()))?;
                let elements = input.shape.elements();
                let batch_elements = elements * self.instances as usize;
                let expanded = if values.len() == elements {
                    values
                        .iter()
                        .copied()
                        .cycle()
                        .take(batch_elements)
                        .collect()
                } else if values.len() == batch_elements {
                    values.clone()
                } else {
                    return Err(BatchedExecutionError::InputLength {
                        name: input.name.clone(),
                        expected_single: elements,
                        expected_batch: batch_elements,
                        actual: values.len(),
                    });
                };
                Ok((input.slot, expanded))
            })
            .collect()
    }

    fn initial_state(&self) -> BTreeMap<CellSlotId, Vec<f32>> {
        self.states
            .iter()
            .map(|state| {
                let values = state
                    .initializer
                    .iter()
                    .copied()
                    .cycle()
                    .take(state.shape.elements() * self.instances as usize)
                    .collect();
                (state.slot, values)
            })
            .collect()
    }
}

impl BatchedCpuSession<'_> {
    pub fn dispatch_turns(&mut self, turns: u32) -> Result<(), BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        for _ in 0..turns {
            for instance in 0..self.program.instances as usize {
                for input in &self.program.inputs {
                    let offset = self.program.register_offsets[&input.slot];
                    let elements = input.shape.elements();
                    let values = &self.inputs[&input.slot];
                    self.registers[offset..offset + elements]
                        .copy_from_slice(&values[instance * elements..(instance + 1) * elements]);
                }
                for state in &self.program.states {
                    let offset = self.program.register_offsets[&state.slot];
                    let elements = state.shape.elements();
                    let values = &self.state[&state.slot];
                    self.registers[offset..offset + elements]
                        .copy_from_slice(&values[instance * elements..(instance + 1) * elements]);
                }
                for instruction in &self.program.instructions {
                    self.registers[instruction.output] =
                        instruction.computation.evaluate(&self.registers);
                }
                for state in &self.program.states {
                    let elements = state.shape.elements();
                    let destination = self.next_state.get_mut(&state.slot).unwrap();
                    for (component, source) in state.update.iter().enumerate() {
                        destination[instance * elements + component] =
                            source.evaluate(&self.registers);
                    }
                }
            }
            std::mem::swap(&mut self.state, &mut self.next_state);
        }
        Ok(())
    }

    pub fn state(&self) -> &BTreeMap<CellSlotId, Vec<f32>> {
        &self.state
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchedExecutionError {
    ZeroTurns,
    MissingInput(String),
    InputLength {
        name: String,
        expected_single: usize,
        expected_batch: usize,
        actual: usize,
    },
    Native(String),
}

impl std::fmt::Display for BatchedExecutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for BatchedExecutionError {}

struct PendingState {
    slot: CellSlotId,
    shape: FixedShape,
    initializer: Vec<f32>,
    update: Option<Vec<ScalarOperand>>,
}

struct BatchCompiler<'a> {
    artifact: &'a ProgramArtifact,
    instances: u32,
    shapes: BTreeMap<CellSlotId, FixedShape>,
    register_offsets: BTreeMap<CellSlotId, usize>,
    register_count: usize,
    instructions: Vec<ScalarInstruction>,
    inputs: Vec<(CellSlotId, String, FixedShape)>,
    states: BTreeMap<CellSlotId, PendingState>,
    diagnostics: Vec<GpuDiagnostic>,
}

impl<'a> BatchCompiler<'a> {
    fn new(artifact: &'a ProgramArtifact, instances: u32) -> Self {
        Self {
            artifact,
            instances,
            shapes: BTreeMap::new(),
            register_offsets: BTreeMap::new(),
            register_count: 0,
            instructions: Vec::new(),
            inputs: Vec::new(),
            states: BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }

    fn compile(mut self) -> Result<BatchedGpuProgram, GpuAdmissionError> {
        if self.instances == 0 {
            self.reject(
                None,
                None,
                "the outer batch must contain at least one instance",
            );
        }
        if !self.artifact.constraints().is_empty() {
            self.reject(
                None,
                None,
                "integrity constraints are not admitted by the fail-stop batch kernel",
            );
        }
        self.collect_slots();
        self.collect_inputs();
        self.lower_nodes();
        let missing_state_updates = self
            .states
            .values()
            .filter(|state| state.update.is_none())
            .map(|state| state.slot)
            .collect::<Vec<_>>();
        for slot in missing_state_updates {
            self.reject(
                None,
                None,
                format!("state slot {} has no whole-value update", slot.get()),
            );
        }
        if !self.diagnostics.is_empty() {
            return Err(GpuAdmissionError {
                diagnostics: self.diagnostics,
            });
        }

        let mut binding = 0_u32;
        let inputs = self
            .inputs
            .into_iter()
            .map(|(slot, name, shape)| {
                let input = BatchedInput {
                    slot,
                    name,
                    shape,
                    binding,
                };
                binding += 1;
                input
            })
            .collect::<Vec<_>>();
        let states = self
            .states
            .into_values()
            .map(|state| {
                let read_binding = binding;
                let write_binding = binding + 1;
                binding += 2;
                BatchedState {
                    slot: state.slot,
                    shape: state.shape,
                    initializer: state.initializer,
                    update: state.update.unwrap(),
                    read_binding,
                    write_binding,
                }
            })
            .collect::<Vec<_>>();
        let wgsl = generate_wgsl(
            self.instances,
            &self.register_offsets,
            &self.instructions,
            &inputs,
            &states,
        );
        Ok(BatchedGpuProgram {
            instances: self.instances,
            register_count: self.register_count,
            register_offsets: self.register_offsets,
            instructions: self.instructions,
            inputs,
            states,
            wgsl,
        })
    }

    fn collect_slots(&mut self) {
        for slot in self.artifact.slots() {
            if let ProducerReference::NodeOutput { node, .. } = slot.producer {
                let producer = &self.artifact.nodes()[node.get() as usize].operation;
                if producer.module_path.as_ref() == ["core"]
                    && producer.operation_name == "composite-pack"
                {
                    continue;
                }
            }
            let shape = match fixed_shape(self.artifact, slot.schema) {
                Ok(shape) => shape,
                Err(detail) => {
                    self.reject(None, None, format!("slot {}: {detail}", slot.slot.get()));
                    continue;
                }
            };
            self.shapes.insert(slot.slot, shape);
            self.register_offsets.insert(slot.slot, self.register_count);
            self.register_count += shape.elements();
            if slot.role == SlotRole::State {
                match constant_values(self.artifact, slot.initializer, shape) {
                    Ok(initializer) => {
                        self.states.insert(
                            slot.slot,
                            PendingState {
                                slot: slot.slot,
                                shape,
                                initializer,
                                update: None,
                            },
                        );
                    }
                    Err(detail) => self.reject(None, None, detail),
                }
            }
        }
    }

    fn collect_inputs(&mut self) {
        let required = turn_required_nodes(self.artifact);
        let required_slots = required
            .iter()
            .flat_map(|node| {
                self.artifact.nodes()[node.get() as usize]
                    .input_bindings
                    .clone()
            })
            .filter_map(
                |binding| match self.artifact.bindings().get(binding as usize) {
                    Some(BindingDeclaration::Input {
                        source: ArtifactSource::Slot(slot),
                        ..
                    }) => Some(*slot),
                    _ => None,
                },
            )
            .collect::<BTreeSet<_>>();
        for input in self.artifact.inputs() {
            if required_slots.contains(&input.slot)
                && let Some(shape) = self.shapes.get(&input.slot).copied()
            {
                self.inputs.push((input.slot, input.name.clone(), shape));
            }
        }
    }

    fn lower_nodes(&mut self) {
        let required = turn_required_nodes(self.artifact);
        for node in self.artifact.nodes() {
            if !required.contains(&node.node) {
                continue;
            }
            let name = node.operation.operation_name.as_str();
            if node.operation.module_path.as_ref() == ["core"] && name == "composite-pack" {
                continue;
            }
            let inputs = node
                .input_bindings
                .clone()
                .filter_map(
                    |binding| match self.artifact.bindings().get(binding as usize) {
                        Some(BindingDeclaration::Input { source, .. }) => Some(*source),
                        _ => None,
                    },
                )
                .collect::<Vec<_>>();
            let outputs = node
                .output_bindings
                .clone()
                .filter_map(
                    |binding| match self.artifact.bindings().get(binding as usize) {
                        Some(BindingDeclaration::Output { target, .. }) => Some(*target),
                        _ => None,
                    },
                )
                .collect::<Vec<_>>();
            let runtime_operation = node.operation.module_path.as_ref() == ["runtime"];
            if outputs.iter().any(|slot| self.states.contains_key(slot)) {
                if runtime_operation {
                    self.lower_state(node.node, name, &inputs, &outputs);
                } else {
                    self.reject(
                        Some(node.node),
                        Some(display_operation(&node.operation)),
                        "batch state updates must use a compiler-selected runtime operation",
                    );
                }
                continue;
            }
            if outputs.len() != 1 {
                self.reject(
                    Some(node.node),
                    Some(display_operation(&node.operation)),
                    format!("expected one output, found {}", outputs.len()),
                );
                continue;
            }
            let output = outputs[0];
            let result = if runtime_operation && name.starts_with("HorizontalConcatenate") {
                self.lower_concatenate(output, &inputs, true)
            } else if runtime_operation && name.starts_with("VerticalConcatenate") {
                self.lower_concatenate(output, &inputs, false)
            } else if runtime_operation && name.starts_with("Transpose") {
                self.lower_transpose(output, &inputs)
            } else if runtime_operation && name.starts_with("MatMul") {
                self.lower_matmul(output, &inputs)
            } else if runtime_operation && name.starts_with("Dot") {
                self.lower_dot(output, &inputs)
            } else if runtime_operation && name.starts_with("Negate") {
                self.lower_negate(output, &inputs)
            } else if runtime_operation && let Some(operation) = scalar_operation(name) {
                self.lower_elementwise(output, &inputs, operation)
            } else {
                Err(format!(
                    "generic fixed-shape lowering does not support {name}"
                ))
            };
            if let Err(detail) = result {
                self.reject(
                    Some(node.node),
                    Some(display_operation(&node.operation)),
                    detail,
                );
            }
        }
    }

    fn lower_state(
        &mut self,
        node: NodeId,
        name: &str,
        inputs: &[ArtifactSource],
        outputs: &[CellSlotId],
    ) {
        if !name.starts_with("Assign") || inputs.len() != 1 || outputs.len() != 1 {
            self.reject(
                Some(node),
                Some(name.to_owned()),
                "batch state requires one whole-value Assign",
            );
            return;
        }
        let target = outputs[0];
        let shape = self.states[&target].shape;
        let update = (0..shape.elements())
            .map(|component| self.operand(inputs[0], component))
            .collect::<Result<Vec<_>, _>>();
        match update {
            Ok(update) => self.states.get_mut(&target).unwrap().update = Some(update),
            Err(detail) => self.reject(Some(node), Some(name.to_owned()), detail),
        }
    }

    fn lower_elementwise(
        &mut self,
        output: CellSlotId,
        inputs: &[ArtifactSource],
        operation: ElementwiseOperation,
    ) -> Result<(), String> {
        if inputs.len() != operation.arity() {
            return Err(format!(
                "expected {} inputs, found {}",
                operation.arity(),
                inputs.len()
            ));
        }
        let shape = self.shape(output)?;
        for component in 0..shape.elements() {
            let operands = inputs
                .iter()
                .map(|source| {
                    let input_shape = self.source_shape(*source)?;
                    let component = if input_shape.elements() == 1 {
                        0
                    } else {
                        component
                    };
                    self.operand(*source, component)
                })
                .collect::<Result<Vec<_>, String>>()?;
            self.emit(
                output,
                component,
                ScalarComputation::Elementwise {
                    operation,
                    inputs: operands,
                },
            );
        }
        Ok(())
    }

    fn lower_negate(
        &mut self,
        output: CellSlotId,
        inputs: &[ArtifactSource],
    ) -> Result<(), String> {
        if inputs.len() != 1 {
            return Err("negate requires one input".to_owned());
        }
        let shape = self.shape(output)?;
        for component in 0..shape.elements() {
            let input = self.operand(inputs[0], component)?;
            self.emit(output, component, ScalarComputation::Negate(input));
        }
        Ok(())
    }

    fn lower_dot(&mut self, output: CellSlotId, inputs: &[ArtifactSource]) -> Result<(), String> {
        if inputs.len() != 2 || self.shape(output)?.elements() != 1 {
            return Err("dot requires two inputs and a scalar output".to_owned());
        }
        let left = self.source_shape(inputs[0])?;
        let right = self.source_shape(inputs[1])?;
        if left.elements() != right.elements() {
            return Err("dot input element counts differ".to_owned());
        }
        let terms = (0..left.elements())
            .map(|component| {
                Ok((
                    self.operand(inputs[0], component)?,
                    self.operand(inputs[1], component)?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        self.emit(output, 0, ScalarComputation::SumProducts(terms));
        Ok(())
    }

    fn lower_matmul(
        &mut self,
        output: CellSlotId,
        inputs: &[ArtifactSource],
    ) -> Result<(), String> {
        if inputs.len() != 2 {
            return Err("matrix multiplication requires two inputs".to_owned());
        }
        let left = self.source_shape(inputs[0])?;
        let right = self.source_shape(inputs[1])?;
        let result = self.shape(output)?;
        if left.columns != right.rows || result.rows != left.rows || result.columns != right.columns
        {
            return Err(format!(
                "invalid matrix product {}x{} ** {}x{} -> {}x{}",
                left.rows, left.columns, right.rows, right.columns, result.rows, result.columns
            ));
        }
        for column in 0..result.columns {
            for row in 0..result.rows {
                let terms = (0..left.columns)
                    .map(|inner| {
                        Ok((
                            self.operand(inputs[0], left.index(row, inner))?,
                            self.operand(inputs[1], right.index(inner, column))?,
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                self.emit(
                    output,
                    result.index(row, column),
                    ScalarComputation::SumProducts(terms),
                );
            }
        }
        Ok(())
    }

    fn lower_transpose(
        &mut self,
        output: CellSlotId,
        inputs: &[ArtifactSource],
    ) -> Result<(), String> {
        if inputs.len() != 1 {
            return Err("transpose requires one input".to_owned());
        }
        let input = self.source_shape(inputs[0])?;
        let result = self.shape(output)?;
        if result.rows != input.columns || result.columns != input.rows {
            return Err("transpose output shape is inconsistent".to_owned());
        }
        for column in 0..result.columns {
            for row in 0..result.rows {
                let source = self.operand(inputs[0], input.index(column, row))?;
                self.emit(
                    output,
                    result.index(row, column),
                    ScalarComputation::Copy(source),
                );
            }
        }
        Ok(())
    }

    fn lower_concatenate(
        &mut self,
        output: CellSlotId,
        inputs: &[ArtifactSource],
        horizontal: bool,
    ) -> Result<(), String> {
        let result = self.shape(output)?;
        let mut row_offset = 0;
        let mut column_offset = 0;
        for source in inputs {
            let shape = self.source_shape(*source)?;
            if horizontal && shape.rows != result.rows {
                return Err("horizontal concatenation row count differs".to_owned());
            }
            if !horizontal && shape.columns != result.columns {
                return Err("vertical concatenation column count differs".to_owned());
            }
            for column in 0..shape.columns {
                for row in 0..shape.rows {
                    let destination = result.index(row + row_offset, column + column_offset);
                    let operand = self.operand(*source, shape.index(row, column))?;
                    self.emit(output, destination, ScalarComputation::Copy(operand));
                }
            }
            if horizontal {
                column_offset += shape.columns;
            } else {
                row_offset += shape.rows;
            }
        }
        if (horizontal && column_offset != result.columns)
            || (!horizontal && row_offset != result.rows)
        {
            return Err("concatenation inputs do not fill the output shape".to_owned());
        }
        Ok(())
    }

    fn emit(&mut self, slot: CellSlotId, component: usize, computation: ScalarComputation) {
        self.instructions.push(ScalarInstruction {
            output: self.register_offsets[&slot] + component,
            computation,
        });
    }

    fn operand(&self, source: ArtifactSource, component: usize) -> Result<ScalarOperand, String> {
        match source {
            ArtifactSource::Slot(slot) => {
                let shape = self.shape(slot)?;
                if component >= shape.elements() {
                    return Err(format!(
                        "slot {} component {component} is out of bounds",
                        slot.get()
                    ));
                }
                Ok(ScalarOperand::Register(
                    self.register_offsets[&slot] + component,
                ))
            }
            ArtifactSource::Constant(constant) => {
                let values = artifact_constant_values(self.artifact, constant)?;
                values
                    .get(component)
                    .copied()
                    .map(ScalarOperand::Constant)
                    .ok_or_else(|| format!("constant component {component} is out of bounds"))
            }
        }
    }

    fn source_shape(&self, source: ArtifactSource) -> Result<FixedShape, String> {
        match source {
            ArtifactSource::Slot(slot) => self.shape(slot),
            ArtifactSource::Constant(constant) => {
                let value = self
                    .artifact
                    .constants()
                    .get(constant)
                    .ok_or_else(|| format!("constant {} does not exist", constant.get()))?;
                match value.data() {
                    ValueData::F32(_) => Ok(FixedShape::scalar()),
                    ValueData::Matrix(_) => fixed_shape(self.artifact, value.schema()),
                    _ => Err("only f32 constants are admitted".to_owned()),
                }
            }
        }
    }

    fn shape(&self, slot: CellSlotId) -> Result<FixedShape, String> {
        self.shapes
            .get(&slot)
            .copied()
            .ok_or_else(|| format!("slot {} has no fixed f32 shape", slot.get()))
    }

    fn reject(
        &mut self,
        node: Option<NodeId>,
        operation: Option<String>,
        detail: impl Into<String>,
    ) {
        self.diagnostics.push(GpuDiagnostic {
            code: GpuDiagnosticCode::OperationUnsupported,
            node,
            operation,
            detail: detail.into(),
        });
    }
}

fn scalar_operation(name: &str) -> Option<ElementwiseOperation> {
    use super::{BinaryOperation, UnaryOperation};

    if name.starts_with("Add") {
        Some(ElementwiseOperation::Binary(BinaryOperation::Add))
    } else if name.starts_with("Sub") {
        Some(ElementwiseOperation::Binary(BinaryOperation::Subtract))
    } else if name.starts_with("Mul") {
        Some(ElementwiseOperation::Binary(BinaryOperation::Multiply))
    } else if name.starts_with("Div") {
        Some(ElementwiseOperation::Binary(BinaryOperation::Divide))
    } else if name.starts_with("MathSin") {
        Some(ElementwiseOperation::Unary(UnaryOperation::Sin))
    } else if name.starts_with("MathCos") {
        Some(ElementwiseOperation::Unary(UnaryOperation::Cos))
    } else if name.starts_with("Atan2") {
        Some(ElementwiseOperation::Atan2)
    } else {
        None
    }
}

fn fixed_shape(
    artifact: &ProgramArtifact,
    schema: mech_core::SchemaId,
) -> Result<FixedShape, String> {
    let schema = artifact
        .schemas()
        .get(schema)
        .ok_or_else(|| "schema does not exist".to_owned())?;
    match schema.body() {
        SchemaBody::FloatingPoint(FloatWidth::W32) => Ok(FixedShape::scalar()),
        SchemaBody::Matrix {
            element,
            dimensions,
        } if matches!(element.as_ref(), SchemaBody::FloatingPoint(FloatWidth::W32)) => {
            let dimensions = dimensions
                .iter()
                .map(|dimension| match dimension {
                    DimensionExpr::Constant(value) => usize::try_from(*value)
                        .map_err(|_| "matrix dimension does not fit usize".to_owned()),
                    _ => Err("matrix dimension is not compile-time constant".to_owned()),
                })
                .collect::<Result<Vec<_>, _>>()?;
            if dimensions.is_empty() || dimensions.len() > 2 {
                return Err("only scalar, vector, and matrix shapes are admitted".to_owned());
            }
            Ok(FixedShape {
                rows: dimensions[0],
                columns: dimensions.get(1).copied().unwrap_or(1),
            })
        }
        _ => Err("schema is not fixed-shape f32 numeric data".to_owned()),
    }
}

fn artifact_constant_values(
    artifact: &ProgramArtifact,
    constant: mech_core::ConstantId,
) -> Result<Vec<f32>, String> {
    let value = artifact
        .constants()
        .get(constant)
        .ok_or_else(|| format!("constant {} does not exist", constant.get()))?;
    match value.data() {
        ValueData::F32(value) => Ok(vec![value.to_f32()]),
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::F32(values) => Ok(values.iter().map(|value| value.to_f32()).collect()),
            _ => Err("matrix constant is not f32".to_owned()),
        },
        _ => Err("constant is not f32 numeric data".to_owned()),
    }
}

fn constant_values(
    artifact: &ProgramArtifact,
    initializer: Option<mech_engine::InitializerReference>,
    shape: FixedShape,
) -> Result<Vec<f32>, String> {
    let Some(mech_engine::InitializerReference::Constant(constant)) = initializer else {
        return Err("batch state requires a constant initializer".to_owned());
    };
    let values = artifact_constant_values(artifact, constant)?;
    if values.len() != shape.elements() {
        return Err(format!(
            "state initializer has {} elements, expected {}",
            values.len(),
            shape.elements()
        ));
    }
    Ok(values)
}

fn generate_wgsl(
    instances: u32,
    register_offsets: &BTreeMap<CellSlotId, usize>,
    instructions: &[ScalarInstruction],
    inputs: &[BatchedInput],
    states: &[BatchedState],
) -> String {
    let mut shader = String::from("// Generic fixed-shape Mech batch kernel.\n");
    for input in inputs {
        shader.push_str(&format!(
            "@group(0) @binding({}) var<storage, read> input_{}: array<f32>;\n",
            input.binding,
            input.slot.get()
        ));
    }
    for state in states {
        shader.push_str(&format!(
            "@group(0) @binding({}) var<storage, read> state_read_{}: array<f32>;\n",
            state.read_binding,
            state.slot.get()
        ));
        shader.push_str(&format!(
            "@group(0) @binding({}) var<storage, read_write> state_write_{}: array<f32>;\n",
            state.write_binding,
            state.slot.get()
        ));
    }
    shader.push_str(&format!(
        "\n@compute @workgroup_size({WORKGROUP_SIZE})\nfn main(@builtin(global_invocation_id) gid: vec3<u32>) {{\n  let index = gid.x;\n  if (index >= {instances}u) {{ return; }}\n"
    ));
    for input in inputs {
        let offset = register_offsets[&input.slot];
        for component in 0..input.shape.elements() {
            shader.push_str(&format!(
                "  let r{} = input_{}[index * {}u + {}u];\n",
                offset + component,
                input.slot.get(),
                input.shape.elements(),
                component
            ));
        }
    }
    for state in states {
        let offset = register_offsets[&state.slot];
        for component in 0..state.shape.elements() {
            shader.push_str(&format!(
                "  let r{} = state_read_{}[index * {}u + {}u];\n",
                offset + component,
                state.slot.get(),
                state.shape.elements(),
                component
            ));
        }
    }
    for instruction in instructions {
        shader.push_str(&format!(
            "  let r{} = {};\n",
            instruction.output,
            instruction.computation.wgsl()
        ));
    }
    for state in states {
        for (component, source) in state.update.iter().enumerate() {
            shader.push_str(&format!(
                "  state_write_{}[index * {}u + {}u] = {};\n",
                state.slot.get(),
                state.shape.elements(),
                component,
                source.wgsl()
            ));
        }
    }
    shader.push_str("}\n");
    shader
}

#[cfg(feature = "native")]
mod native {
    use std::{
        collections::BTreeMap,
        sync::{Arc, mpsc},
        time::{Duration, Instant},
    };

    use mech_core::CellSlotId;
    use wgpu::util::DeviceExt;

    use super::{BatchedExecutionError, BatchedGpuProgram};

    #[derive(Debug)]
    pub struct BatchedResidentGpuSession {
        adapter: String,
        device: wgpu::Device,
        queue: wgpu::Queue,
        pipeline: wgpu::ComputePipeline,
        bind_groups: [wgpu::BindGroup; 2],
        output_buffers: [BTreeMap<CellSlotId, Arc<wgpu::Buffer>>; 2],
        output_elements: BTreeMap<CellSlotId, usize>,
        workgroups: u32,
        next_group: usize,
        last_output_group: Option<usize>,
    }

    #[derive(Clone, Debug)]
    pub struct BatchedDispatchProfile {
        pub adapter: String,
        pub turns: u32,
        pub dispatch: Duration,
        pub readback: Duration,
        pub state: BTreeMap<CellSlotId, Vec<f32>>,
    }

    impl BatchedGpuProgram {
        pub fn prepare_resident(
            &self,
            inputs: &BTreeMap<String, Vec<f32>>,
        ) -> Result<BatchedResidentGpuSession, BatchedExecutionError> {
            pollster::block_on(self.prepare_resident_async(inputs))
        }

        async fn prepare_resident_async(
            &self,
            inputs: &BTreeMap<String, Vec<f32>>,
        ) -> Result<BatchedResidentGpuSession, BatchedExecutionError> {
            let instance = wgpu::Instance::default();
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .ok_or_else(|| {
                    BatchedExecutionError::Native("GPU adapter unavailable".to_owned())
                })?;
            let adapter_info = adapter.get_info();
            let adapter_name = format!("{} ({:?})", adapter_info.name, adapter_info.backend);
            let required_storage_buffers = (self.inputs.len() + self.states.len() * 2) as u32;
            let limits = adapter.limits();
            if required_storage_buffers > limits.max_storage_buffers_per_shader_stage {
                return Err(BatchedExecutionError::Native(format!(
                    "kernel requires {required_storage_buffers} storage buffers; adapter supports {}",
                    limits.max_storage_buffers_per_shader_stage
                )));
            }
            if self.workgroup_count() > limits.max_compute_workgroups_per_dimension {
                return Err(BatchedExecutionError::Native(format!(
                    "kernel requires {} workgroups; adapter supports {}",
                    self.workgroup_count(),
                    limits.max_compute_workgroups_per_dimension
                )));
            }
            let required_limits = wgpu::Limits {
                max_storage_buffers_per_shader_stage: required_storage_buffers,
                ..wgpu::Limits::downlevel_defaults()
            };
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("Mech fixed-shape batch device"),
                        required_features: wgpu::Features::empty(),
                        required_limits,
                    },
                    None,
                )
                .await
                .map_err(|error| BatchedExecutionError::Native(error.to_string()))?;

            let expanded_inputs = self.expand_inputs(inputs)?;
            let mut input_buffers = BTreeMap::new();
            for input in &self.inputs {
                input_buffers.insert(
                    input.slot,
                    Arc::new(
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(&input.name),
                            contents: bytemuck::cast_slice(&expanded_inputs[&input.slot]),
                            usage: wgpu::BufferUsages::STORAGE,
                        }),
                    ),
                );
            }

            let initial_state = self.initial_state();
            let mut state_buffers = BTreeMap::new();
            for state in &self.states {
                let initial = Arc::new(device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some("Mech fixed-shape initial state"),
                        contents: bytemuck::cast_slice(&initial_state[&state.slot]),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    },
                ));
                let alternate = Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Mech fixed-shape alternate state"),
                    size: (initial_state[&state.slot].len() * std::mem::size_of::<f32>()) as u64,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }));
                state_buffers.insert(state.slot, [initial, alternate]);
            }

            let layout_entries =
                self.inputs
                    .iter()
                    .map(|input| (input.binding, true))
                    .chain(self.states.iter().flat_map(|state| {
                        [(state.read_binding, true), (state.write_binding, false)]
                    }))
                    .map(|(binding, read_only)| wgpu::BindGroupLayoutEntry {
                        binding,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    })
                    .collect::<Vec<_>>();
            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Mech fixed-shape batch bindings"),
                    entries: &layout_entries,
                });
            let bind_groups = [0, 1].map(|group| {
                let entries = self
                    .inputs
                    .iter()
                    .map(|input| wgpu::BindGroupEntry {
                        binding: input.binding,
                        resource: input_buffers[&input.slot].as_entire_binding(),
                    })
                    .chain(self.states.iter().flat_map(|state| {
                        [
                            wgpu::BindGroupEntry {
                                binding: state.read_binding,
                                resource: state_buffers[&state.slot][group].as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: state.write_binding,
                                resource: state_buffers[&state.slot][1 - group].as_entire_binding(),
                            },
                        ]
                    }))
                    .collect::<Vec<_>>();
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Mech fixed-shape batch bind group"),
                    layout: &bind_group_layout,
                    entries: &entries,
                })
            });
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Scalarized Mech fixed-shape batch"),
                source: wgpu::ShaderSource::Wgsl(self.wgsl.clone().into()),
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Mech fixed-shape batch pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Mech fixed-shape batch pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            });

            let output_buffers = [0, 1].map(|group| {
                self.states
                    .iter()
                    .map(|state| (state.slot, state_buffers[&state.slot][1 - group].clone()))
                    .collect()
            });
            let output_elements = self
                .states
                .iter()
                .map(|state| (state.slot, state.shape.elements() * self.instances as usize))
                .collect();
            Ok(BatchedResidentGpuSession {
                adapter: adapter_name,
                device,
                queue,
                pipeline,
                bind_groups,
                output_buffers,
                output_elements,
                workgroups: self.workgroup_count(),
                next_group: 0,
                last_output_group: None,
            })
        }
    }

    impl BatchedResidentGpuSession {
        pub fn adapter(&self) -> &str {
            &self.adapter
        }

        pub fn dispatch_turns(&mut self, turns: u32) -> Result<Duration, BatchedExecutionError> {
            if turns == 0 {
                return Err(BatchedExecutionError::ZeroTurns);
            }
            let started = Instant::now();
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Mech fixed-shape batch command encoder"),
                });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Mech fixed-shape batch compute pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipeline);
                for _ in 0..turns {
                    let group = self.next_group;
                    pass.set_bind_group(0, &self.bind_groups[group], &[]);
                    pass.dispatch_workgroups(self.workgroups, 1, 1);
                    self.last_output_group = Some(group);
                    self.next_group = 1 - group;
                }
            }
            self.queue.submit(Some(encoder.finish()));
            self.device.poll(wgpu::Maintain::Wait);
            Ok(started.elapsed())
        }

        pub fn read_state(
            &self,
        ) -> Result<(Duration, BTreeMap<CellSlotId, Vec<f32>>), BatchedExecutionError> {
            let group = self.last_output_group.ok_or_else(|| {
                BatchedExecutionError::Native("no batch turns have run".to_owned())
            })?;
            let started = Instant::now();
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Mech fixed-shape batch readback encoder"),
                });
            let mut readbacks = Vec::new();
            for (slot, buffer) in &self.output_buffers[group] {
                let size = (self.output_elements[slot] * std::mem::size_of::<f32>()) as u64;
                let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Mech fixed-shape batch readback"),
                    size,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                encoder.copy_buffer_to_buffer(buffer, 0, &readback, 0, size);
                readbacks.push((*slot, readback));
            }
            self.queue.submit(Some(encoder.finish()));

            let mut state = BTreeMap::new();
            for (slot, readback) in readbacks {
                let slice = readback.slice(..);
                let (sender, receiver) = mpsc::channel();
                slice.map_async(wgpu::MapMode::Read, move |result| {
                    let _ = sender.send(result);
                });
                self.device.poll(wgpu::Maintain::Wait);
                receiver
                    .recv()
                    .map_err(|_| BatchedExecutionError::Native("map channel closed".to_owned()))?
                    .map_err(|error| BatchedExecutionError::Native(error.to_string()))?;
                let mapped = slice.get_mapped_range();
                state.insert(slot, bytemuck::cast_slice::<u8, f32>(&mapped).to_vec());
                drop(mapped);
                readback.unmap();
            }
            Ok((started.elapsed(), state))
        }

        pub fn run_turns(
            &mut self,
            turns: u32,
        ) -> Result<BatchedDispatchProfile, BatchedExecutionError> {
            let dispatch = self.dispatch_turns(turns)?;
            let (readback, state) = self.read_state()?;
            Ok(BatchedDispatchProfile {
                adapter: self.adapter.clone(),
                turns,
                dispatch,
                readback,
                state,
            })
        }
    }
}

#[cfg(feature = "native")]
pub use native::*;
