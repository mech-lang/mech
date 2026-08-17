use std::collections::{BTreeMap, BTreeSet};

use mech_core::{
    CellSlotId, DimensionExpr, FloatWidth, IntegrityConstraintId, NodeId, SchemaBody, ValueData,
    snapshot::SequenceView,
};
use mech_engine::{
    ArtifactSource, BindingDeclaration, ComputeRegionDeclaration, ProducerReference,
    ProgramArtifact, SlotRole,
};
use wide::{CmpEq, CmpGe, CmpGt, CmpLe, CmpLt, CmpNe, f32x4};

use super::{
    BinaryOperation, ElementwiseOperation, GpuAdmissionError, GpuDiagnostic, GpuDiagnosticCode,
    UnaryOperation, WORKGROUP_SIZE, display_operation, turn_required_nodes,
};

#[cfg(feature = "jit")]
mod jit;
#[cfg(feature = "jit")]
pub use jit::*;

const SIMD_LANES: usize = 4;

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

    fn evaluate_simd(self, registers: &[f32x4]) -> f32x4 {
        match self {
            Self::Register(register) => registers[register],
            Self::Constant(value) => f32x4::splat(value),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ComparisonOperation {
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
}

impl ComparisonOperation {
    fn apply(self, left: f32, right: f32) -> bool {
        match self {
            Self::Equal => left == right,
            Self::NotEqual => left != right,
            Self::Less => left < right,
            Self::Greater => left > right,
            Self::LessEqual => left <= right,
            Self::GreaterEqual => left >= right,
        }
    }

    fn apply_simd(self, left: f32x4, right: f32x4) -> f32x4 {
        self.mask_simd(left, right).blend(f32x4::ONE, f32x4::ZERO)
    }

    fn mask_simd(self, left: f32x4, right: f32x4) -> f32x4 {
        match self {
            Self::Equal => left.cmp_eq(right),
            Self::NotEqual => left.cmp_ne(right),
            Self::Less => left.cmp_lt(right),
            Self::Greater => left.cmp_gt(right),
            Self::LessEqual => left.cmp_le(right),
            Self::GreaterEqual => left.cmp_ge(right),
        }
    }

    const fn wgsl(self) -> &'static str {
        match self {
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::Less => "<",
            Self::Greater => ">",
            Self::LessEqual => "<=",
            Self::GreaterEqual => ">=",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LogicOperation {
    And,
    Or,
    Xor,
    Not,
}

impl LogicOperation {
    fn apply(self, left: bool, right: Option<bool>) -> bool {
        match self {
            Self::And => left && right.expect("binary logic operation has a right operand"),
            Self::Or => left || right.expect("binary logic operation has a right operand"),
            Self::Xor => left ^ right.expect("binary logic operation has a right operand"),
            Self::Not => !left,
        }
    }
}

#[derive(Clone, Debug)]
enum ScalarComputation {
    Copy(ScalarOperand),
    Negate(ScalarOperand),
    Absolute(ScalarOperand),
    IsFinite(ScalarOperand),
    Compare {
        operation: ComparisonOperation,
        left: ScalarOperand,
        right: ScalarOperand,
    },
    Logic {
        operation: LogicOperation,
        inputs: Vec<ScalarOperand>,
    },
    Elementwise {
        operation: ElementwiseOperation,
        inputs: Vec<ScalarOperand>,
    },
    SumProducts(Vec<(ScalarOperand, ScalarOperand)>),
}

#[derive(Clone, Debug)]
enum ScalarPredicate {
    Value(ScalarOperand),
    IsFinite(ScalarOperand),
    AbsoluteDifferenceWithin {
        left: ScalarOperand,
        right: ScalarOperand,
        tolerance: ScalarOperand,
    },
    Compare {
        operation: ComparisonOperation,
        left: ScalarOperand,
        right: ScalarOperand,
    },
    All(Vec<ScalarPredicate>),
    Logic {
        operation: LogicOperation,
        inputs: Vec<ScalarPredicate>,
    },
}

impl ScalarPredicate {
    fn evaluate(&self, registers: &[f32]) -> bool {
        match self {
            Self::Value(value) => value.evaluate(registers) != 0.0,
            Self::IsFinite(value) => value.evaluate(registers).is_finite(),
            Self::AbsoluteDifferenceWithin {
                left,
                right,
                tolerance,
            } => {
                (left.evaluate(registers) - right.evaluate(registers)).abs()
                    <= tolerance.evaluate(registers)
            }
            Self::Compare {
                operation,
                left,
                right,
            } => operation.apply(left.evaluate(registers), right.evaluate(registers)),
            Self::All(inputs) => inputs.iter().all(|input| input.evaluate(registers)),
            Self::Logic { operation, inputs } => {
                let left = inputs[0].evaluate(registers);
                let right = inputs.get(1).map(|input| input.evaluate(registers));
                operation.apply(left, right)
            }
        }
    }

    fn evaluate_simd_mask(&self, registers: &[f32x4]) -> f32x4 {
        match self {
            Self::Value(value) => value.evaluate_simd(registers).cmp_ne(f32x4::ZERO),
            Self::IsFinite(value) => value.evaluate_simd(registers).is_finite(),
            Self::AbsoluteDifferenceWithin {
                left,
                right,
                tolerance,
            } => (left.evaluate_simd(registers) - right.evaluate_simd(registers))
                .abs()
                .cmp_le(tolerance.evaluate_simd(registers)),
            Self::Compare {
                operation,
                left,
                right,
            } => operation.mask_simd(
                left.evaluate_simd(registers),
                right.evaluate_simd(registers),
            ),
            Self::All(inputs) => inputs.iter().fold(!f32x4::ZERO, |mask, input| {
                mask & input.evaluate_simd_mask(registers)
            }),
            Self::Logic { operation, inputs } => {
                let left = inputs[0].evaluate_simd_mask(registers);
                let right = inputs
                    .get(1)
                    .map(|input| input.evaluate_simd_mask(registers));
                match operation {
                    LogicOperation::And => left & right.unwrap(),
                    LogicOperation::Or => left | right.unwrap(),
                    LogicOperation::Xor => left ^ right.unwrap(),
                    LogicOperation::Not => !left,
                }
            }
        }
    }

    fn wgsl(&self) -> String {
        match self {
            Self::Value(value) => format!("({} != 0.0)", value.wgsl()),
            Self::IsFinite(value) => {
                format!("(abs({}) <= 3.402823466e38)", value.wgsl())
            }
            Self::AbsoluteDifferenceWithin {
                left,
                right,
                tolerance,
            } => format!(
                "(abs(({}) - ({})) <= ({}))",
                left.wgsl(),
                right.wgsl(),
                tolerance.wgsl()
            ),
            Self::Compare {
                operation,
                left,
                right,
            } => format!(
                "(({}) {} ({}))",
                left.wgsl(),
                operation.wgsl(),
                right.wgsl()
            ),
            Self::All(inputs) => format!(
                "({})",
                inputs
                    .iter()
                    .map(ScalarPredicate::wgsl)
                    .collect::<Vec<_>>()
                    .join(" && ")
            ),
            Self::Logic { operation, inputs } => {
                let left = inputs[0].wgsl();
                match operation {
                    LogicOperation::And => format!("({left} && {})", inputs[1].wgsl()),
                    LogicOperation::Or => format!("({left} || {})", inputs[1].wgsl()),
                    LogicOperation::Xor => format!("({left} != {})", inputs[1].wgsl()),
                    LogicOperation::Not => format!("(!{left})"),
                }
            }
        }
    }

    fn collect_registers(&self, registers: &mut BTreeSet<usize>) {
        match self {
            Self::Value(value) | Self::IsFinite(value) => {
                collect_operand_register(*value, registers);
            }
            Self::AbsoluteDifferenceWithin {
                left,
                right,
                tolerance,
            } => {
                collect_operand_register(*left, registers);
                collect_operand_register(*right, registers);
                collect_operand_register(*tolerance, registers);
            }
            Self::Compare { left, right, .. } => {
                collect_operand_register(*left, registers);
                collect_operand_register(*right, registers);
            }
            Self::All(inputs) | Self::Logic { inputs, .. } => {
                for input in inputs {
                    input.collect_registers(registers);
                }
            }
        }
    }
}

impl ScalarComputation {
    fn evaluate(&self, registers: &[f32]) -> f32 {
        match self {
            Self::Copy(input) => input.evaluate(registers),
            Self::Negate(input) => -input.evaluate(registers),
            Self::Absolute(input) => input.evaluate(registers).abs(),
            Self::IsFinite(input) => {
                if input.evaluate(registers).is_finite() {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Compare {
                operation,
                left,
                right,
            } => {
                if operation.apply(left.evaluate(registers), right.evaluate(registers)) {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Logic { operation, inputs } => {
                let left = inputs[0].evaluate(registers) != 0.0;
                let right = inputs.get(1).map(|input| input.evaluate(registers) != 0.0);
                if operation.apply(left, right) {
                    1.0
                } else {
                    0.0
                }
            }
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
            Self::Absolute(input) => format!("abs({})", input.wgsl()),
            Self::IsFinite(input) => {
                format!("select(0.0, 1.0, abs({}) <= 3.402823466e38)", input.wgsl())
            }
            Self::Compare {
                operation,
                left,
                right,
            } => format!(
                "select(0.0, 1.0, ({}) {} ({}))",
                left.wgsl(),
                operation.wgsl(),
                right.wgsl()
            ),
            Self::Logic { operation, inputs } => {
                let left = format!("({} != 0.0)", inputs[0].wgsl());
                let condition = match operation {
                    LogicOperation::And => {
                        format!("{left} && ({} != 0.0)", inputs[1].wgsl())
                    }
                    LogicOperation::Or => {
                        format!("{left} || ({} != 0.0)", inputs[1].wgsl())
                    }
                    LogicOperation::Xor => {
                        format!("{left} != ({} != 0.0)", inputs[1].wgsl())
                    }
                    LogicOperation::Not => format!("!{left}"),
                };
                format!("select(0.0, 1.0, {condition})")
            }
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

    fn evaluate_simd(&self, registers: &[f32x4]) -> f32x4 {
        match self {
            Self::Copy(input) => input.evaluate_simd(registers),
            Self::Negate(input) => -input.evaluate_simd(registers),
            Self::Absolute(input) => input.evaluate_simd(registers).abs(),
            Self::IsFinite(input) => input
                .evaluate_simd(registers)
                .is_finite()
                .blend(f32x4::ONE, f32x4::ZERO),
            Self::Compare {
                operation,
                left,
                right,
            } => operation.apply_simd(
                left.evaluate_simd(registers),
                right.evaluate_simd(registers),
            ),
            Self::Logic { operation, inputs } => {
                let left = inputs[0].evaluate_simd(registers).cmp_ne(f32x4::ZERO);
                let right = inputs
                    .get(1)
                    .map(|input| input.evaluate_simd(registers).cmp_ne(f32x4::ZERO));
                let mask = match operation {
                    LogicOperation::And => left & right.unwrap(),
                    LogicOperation::Or => left | right.unwrap(),
                    LogicOperation::Xor => left ^ right.unwrap(),
                    LogicOperation::Not => !left,
                };
                mask.blend(f32x4::ONE, f32x4::ZERO)
            }
            Self::Elementwise { operation, inputs } => {
                let mut values = [f32x4::ZERO; 2];
                for (index, input) in inputs.iter().enumerate() {
                    values[index] = input.evaluate_simd(registers);
                }
                match operation {
                    ElementwiseOperation::Binary(operation) => match operation {
                        BinaryOperation::Add => values[0] + values[1],
                        BinaryOperation::Subtract => values[0] - values[1],
                        BinaryOperation::Multiply => values[0] * values[1],
                        BinaryOperation::Divide => values[0] / values[1],
                    },
                    ElementwiseOperation::Unary(operation) => match operation {
                        UnaryOperation::Sin => values[0].sin(),
                        UnaryOperation::Cos => values[0].cos(),
                    },
                    ElementwiseOperation::Atan2 => values[0].atan2(values[1]),
                    ElementwiseOperation::Identity => values[0],
                    ElementwiseOperation::Pack2 => {
                        unreachable!("pack2 is not admitted by the fixed-shape scalarizer")
                    }
                }
            }
            Self::SumProducts(terms) => terms.iter().fold(f32x4::ZERO, |sum, (left, right)| {
                left.evaluate_simd(registers)
                    .mul_add(right.evaluate_simd(registers), sum)
            }),
        }
    }

    fn collect_registers(&self, registers: &mut BTreeSet<usize>) {
        match self {
            Self::Copy(input)
            | Self::Negate(input)
            | Self::Absolute(input)
            | Self::IsFinite(input) => {
                collect_operand_register(*input, registers);
            }
            Self::Compare { left, right, .. } => {
                collect_operand_register(*left, registers);
                collect_operand_register(*right, registers);
            }
            Self::Logic { inputs, .. } | Self::Elementwise { inputs, .. } => {
                for input in inputs {
                    collect_operand_register(*input, registers);
                }
            }
            Self::SumProducts(terms) => {
                for (left, right) in terms {
                    collect_operand_register(*left, registers);
                    collect_operand_register(*right, registers);
                }
            }
        }
    }
}

fn collect_operand_register(operand: ScalarOperand, registers: &mut BTreeSet<usize>) {
    if let ScalarOperand::Register(register) = operand {
        registers.insert(register);
    }
}

fn compile_scalar_predicate(
    operand: ScalarOperand,
    producers: &BTreeMap<usize, &ScalarComputation>,
) -> ScalarPredicate {
    let ScalarOperand::Register(register) = operand else {
        return ScalarPredicate::Value(operand);
    };
    match producers.get(&register).copied() {
        Some(ScalarComputation::Copy(input)) => compile_scalar_predicate(*input, producers),
        Some(ScalarComputation::IsFinite(input)) => ScalarPredicate::IsFinite(*input),
        Some(ScalarComputation::Compare {
            operation,
            left,
            right,
        }) => compile_comparison_predicate(*operation, *left, *right, producers),
        Some(ScalarComputation::Logic { operation, inputs }) => {
            let inputs = inputs
                .iter()
                .map(|input| compile_scalar_predicate(*input, producers))
                .collect::<Vec<_>>();
            if *operation == LogicOperation::And {
                ScalarPredicate::All(flatten_all(inputs))
            } else {
                ScalarPredicate::Logic {
                    operation: *operation,
                    inputs,
                }
            }
        }
        _ => ScalarPredicate::Value(operand),
    }
}

fn flatten_all(inputs: Vec<ScalarPredicate>) -> Vec<ScalarPredicate> {
    inputs
        .into_iter()
        .flat_map(|input| match input {
            ScalarPredicate::All(nested) => nested,
            input => vec![input],
        })
        .collect()
}

fn compile_comparison_predicate(
    operation: ComparisonOperation,
    left: ScalarOperand,
    right: ScalarOperand,
    producers: &BTreeMap<usize, &ScalarComputation>,
) -> ScalarPredicate {
    if operation == ComparisonOperation::LessEqual
        && let ScalarOperand::Register(absolute_register) = left
        && let Some(ScalarComputation::Absolute(ScalarOperand::Register(difference_register))) =
            producers.get(&absolute_register).copied()
        && let Some(ScalarComputation::Elementwise {
            operation: ElementwiseOperation::Binary(BinaryOperation::Subtract),
            inputs,
        }) = producers.get(difference_register).copied()
        && inputs.len() == 2
    {
        return ScalarPredicate::AbsoluteDifferenceWithin {
            left: inputs[0],
            right: inputs[1],
            tolerance: right,
        };
    }
    ScalarPredicate::Compare {
        operation,
        left,
        right,
    }
}

fn prune_dead_instructions(
    instructions: Vec<ScalarInstruction>,
    states: &BTreeMap<CellSlotId, PendingState>,
    constraints: &[BatchedConstraint],
) -> Vec<ScalarInstruction> {
    let mut live = BTreeSet::new();
    for state in states.values() {
        if let Some(update) = &state.update {
            for source in update {
                collect_operand_register(*source, &mut live);
            }
        }
    }
    for constraint in constraints {
        constraint.predicate.collect_registers(&mut live);
    }

    let mut retained = Vec::with_capacity(instructions.len());
    for instruction in instructions.into_iter().rev() {
        if live.contains(&instruction.output) {
            instruction.computation.collect_registers(&mut live);
            retained.push(instruction);
        }
    }
    retained.reverse();
    retained
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

#[derive(Clone, Debug)]
struct BatchedConstraint {
    id: IntegrityConstraintId,
    name: Box<str>,
    predicate: ScalarPredicate,
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
    constraints: Vec<BatchedConstraint>,
    wgsl: String,
}

/// Bounded fault evidence for one rejected candidate. Sessions retain only the
/// latest record plus a total count, never an append-only transaction log.
#[derive(Clone, Debug, PartialEq)]
pub struct BatchedIntegrityFault {
    pub attempted_turn: u64,
    pub instance: u32,
    pub constraint: IntegrityConstraintId,
    pub constraint_name: Box<str>,
}

#[derive(Debug, Default)]
struct BatchedFaultRecorder {
    attempted_turns: u64,
    fault_count: u64,
    last_fault: Option<BatchedIntegrityFault>,
}

impl BatchedFaultRecorder {
    fn next_turn(&mut self) -> u64 {
        self.attempted_turns = self.attempted_turns.saturating_add(1);
        self.attempted_turns
    }

    fn record(&mut self, fault: BatchedIntegrityFault) -> BatchedExecutionError {
        self.fault_count = self.fault_count.saturating_add(1);
        self.last_fault = Some(fault.clone());
        BatchedExecutionError::Integrity(fault)
    }
}

#[derive(Debug)]
pub struct BatchedCpuSession<'a> {
    program: &'a BatchedGpuProgram,
    inputs: BTreeMap<CellSlotId, Vec<f32>>,
    state: BTreeMap<CellSlotId, Vec<f32>>,
    next_state: BTreeMap<CellSlotId, Vec<f32>>,
    registers: Vec<f32>,
    faults: BatchedFaultRecorder,
}

#[derive(Debug)]
pub struct BatchedSimdCpuSession<'a> {
    program: &'a BatchedGpuProgram,
    inputs: BTreeMap<CellSlotId, Vec<f32>>,
    state: BTreeMap<CellSlotId, Vec<f32>>,
    next_state: BTreeMap<CellSlotId, Vec<f32>>,
    registers: Vec<f32x4>,
    faults: BatchedFaultRecorder,
}

impl super::GpuHost {
    /// Scalarizes generic fixed-shape f32 math and matrix operations, then maps
    /// the resulting kernel over `instances` independent program states.
    ///
    /// Prefer [`Self::compile_broadcast`] for source-driven activation. This
    /// count-taking entry point remains only for callers that have not moved
    /// their batch extent into actual input values yet.
    pub fn compile_batched(
        &self,
        artifact: &ProgramArtifact,
        instances: u32,
    ) -> Result<BatchedGpuProgram, GpuAdmissionError> {
        BatchCompiler::new(artifact, instances).compile()
    }

    /// Compiles one fixed-shape source program and derives its outer broadcast
    /// extent from the supplied input arrays.
    ///
    /// Every required input contains either one source value (broadcast to all
    /// lanes) or one value per lane. Non-singleton extents must agree. The
    /// artifact remains one EKF-sized graph regardless of the number of lanes.
    /// Fixed-shape matrix components use Mech's column-major runtime order;
    /// artifact matrix constants are converted from canonical row-major order
    /// once during lowering, never in the steady-state turn loop.
    pub fn compile_broadcast(
        &self,
        artifact: &ProgramArtifact,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedGpuProgram, GpuAdmissionError> {
        self.compile_broadcast_for_regions(artifact, artifact.compute_regions(), inputs)
    }

    /// Admits one named compute region and derives its outer extent from the
    /// region's activation arrays.
    fn compile_broadcast_for_regions(
        &self,
        artifact: &ProgramArtifact,
        regions: &[ComputeRegionDeclaration],
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedGpuProgram, GpuAdmissionError> {
        let plan = self.plan(artifact);
        let mut diagnostics = plan
            .violations
            .iter()
            .map(|violation| GpuDiagnostic {
                code: GpuDiagnosticCode::PlacementConstraintUnsatisfied,
                node: violation.node,
                operation: None,
                detail: format!("region `{}`: {}", violation.region, violation.reason),
            })
            .collect::<Vec<_>>();
        for region in regions
            .iter()
            .filter(|region| region.placement == mech_core::ComputePlacement::Cpu)
        {
            diagnostics.push(GpuDiagnostic {
                code: GpuDiagnosticCode::PlacementConstraintUnsatisfied,
                node: region.nodes.first().copied(),
                operation: None,
                detail: format!(
                    "region `{}` requires CPU execution and cannot be lowered for GPU broadcast",
                    region.name
                ),
            });
        }
        if regions.len() != 1 {
            diagnostics.push(GpuDiagnostic {
                code: GpuDiagnosticCode::PlacementConstraintUnsatisfied,
                node: None,
                operation: None,
                detail: format!(
                    "source-driven broadcast requires exactly one compute region, found {}",
                    regions.len()
                ),
            });
        }
        if !diagnostics.is_empty() {
            return Err(GpuAdmissionError { diagnostics });
        }
        self.compile_broadcast(artifact, inputs)
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

    pub fn integrity_constraints(&self) -> impl Iterator<Item = IntegrityConstraintId> + '_ {
        self.constraints.iter().map(|constraint| constraint.id)
    }

    pub fn named_integrity_constraints(
        &self,
    ) -> impl Iterator<Item = (IntegrityConstraintId, &str)> + '_ {
        self.constraints
            .iter()
            .map(|constraint| (constraint.id, constraint.name.as_ref()))
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
            faults: BatchedFaultRecorder::default(),
        })
    }

    /// Prepares a four-lane `f32` CPU executor for the same scalarized region
    /// used by the scalar CPU and GPU backends.
    pub fn prepare_simd_cpu(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedSimdCpuSession<'_>, BatchedExecutionError> {
        let inputs = self.expand_inputs(inputs)?;
        let state = self.initial_state();
        let next_state = state
            .iter()
            .map(|(slot, values)| (*slot, vec![0.0; values.len()]))
            .collect();
        Ok(BatchedSimdCpuSession {
            program: self,
            inputs,
            state,
            next_state,
            registers: vec![f32x4::ZERO; self.register_count],
            faults: BatchedFaultRecorder::default(),
        })
    }

    pub const fn simd_lanes(&self) -> usize {
        SIMD_LANES
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

    fn failed_constraint(
        &self,
        registers: &[f32],
        attempted_turn: u64,
        instance: u32,
    ) -> Option<BatchedIntegrityFault> {
        self.constraints.iter().find_map(|constraint| {
            (!constraint.predicate.evaluate(registers)).then_some(BatchedIntegrityFault {
                attempted_turn,
                instance,
                constraint: constraint.id,
                constraint_name: constraint.name.clone(),
            })
        })
    }

    fn failed_packed_constraint(
        &self,
        packed: u64,
        attempted_turn: u64,
    ) -> Option<BatchedIntegrityFault> {
        let code = (packed & 0xff) as usize;
        (code != 0).then(|| {
            let constraint = &self.constraints[code - 1];
            BatchedIntegrityFault {
                attempted_turn,
                instance: (packed >> 8) as u32,
                constraint: constraint.id,
                constraint_name: constraint.name.clone(),
            }
        })
    }
}

impl BatchedCpuSession<'_> {
    pub fn dispatch_turns(&mut self, turns: u32) -> Result<(), BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        for _ in 0..turns {
            let attempted_turn = self.faults.next_turn();
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
                if let Some(fault) =
                    self.program
                        .failed_constraint(&self.registers, attempted_turn, instance as u32)
                {
                    return Err(self.faults.record(fault));
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

    pub const fn fault_count(&self) -> u64 {
        self.faults.fault_count
    }

    pub fn last_fault(&self) -> Option<&BatchedIntegrityFault> {
        self.faults.last_fault.as_ref()
    }
}

impl BatchedSimdCpuSession<'_> {
    pub fn dispatch_turns(&mut self, turns: u32) -> Result<(), BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        let instances = self.program.instances as usize;
        for _ in 0..turns {
            let attempted_turn = self.faults.next_turn();
            for first_instance in (0..instances).step_by(SIMD_LANES) {
                for input in &self.program.inputs {
                    let offset = self.program.register_offsets[&input.slot];
                    let elements = input.shape.elements();
                    let values = &self.inputs[&input.slot];
                    for component in 0..elements {
                        self.registers[offset + component] =
                            gather_simd(values, first_instance, instances, elements, component);
                    }
                }
                for state in &self.program.states {
                    let offset = self.program.register_offsets[&state.slot];
                    let elements = state.shape.elements();
                    let values = &self.state[&state.slot];
                    for component in 0..elements {
                        self.registers[offset + component] =
                            gather_simd(values, first_instance, instances, elements, component);
                    }
                }
                for instruction in &self.program.instructions {
                    self.registers[instruction.output] =
                        instruction.computation.evaluate_simd(&self.registers);
                }
                for constraint in &self.program.constraints {
                    let valid_mask = constraint.predicate.evaluate_simd_mask(&self.registers);
                    let active_lanes = (instances - first_instance).min(SIMD_LANES);
                    let active_mask = (1_u32 << active_lanes) - 1;
                    let failed_lanes = (!valid_mask).move_mask() as u32 & active_mask;
                    if failed_lanes != 0 {
                        let lane = failed_lanes.trailing_zeros() as usize;
                        return Err(self.faults.record(BatchedIntegrityFault {
                            attempted_turn,
                            instance: (first_instance + lane) as u32,
                            constraint: constraint.id,
                            constraint_name: constraint.name.clone(),
                        }));
                    }
                }
                for state in &self.program.states {
                    let elements = state.shape.elements();
                    let destination = self.next_state.get_mut(&state.slot).unwrap();
                    for (component, source) in state.update.iter().enumerate() {
                        let lanes = source.evaluate_simd(&self.registers).to_array();
                        for (lane, value) in lanes.into_iter().enumerate() {
                            let instance = first_instance + lane;
                            if instance < instances {
                                destination[instance * elements + component] = value;
                            }
                        }
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

    pub const fn fault_count(&self) -> u64 {
        self.faults.fault_count
    }

    pub fn last_fault(&self) -> Option<&BatchedIntegrityFault> {
        self.faults.last_fault.as_ref()
    }
}

fn gather_simd(
    values: &[f32],
    first_instance: usize,
    instances: usize,
    elements: usize,
    component: usize,
) -> f32x4 {
    let mut lanes = [0.0; SIMD_LANES];
    for (lane, value) in lanes.iter_mut().enumerate() {
        let instance = first_instance + lane;
        if instance < instances {
            *value = values[instance * elements + component];
        }
    }
    f32x4::new(lanes)
}

#[derive(Clone, Debug, PartialEq)]
pub enum BatchedExecutionError {
    ZeroTurns,
    MissingInput(String),
    InputLength {
        name: String,
        expected_single: usize,
        expected_batch: usize,
        actual: usize,
    },
    IntegrityConfiguration(String),
    Integrity(BatchedIntegrityFault),
    Native(String),
}

impl std::fmt::Display for BatchedExecutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for BatchedExecutionError {}

fn infer_broadcast_instances(
    artifact: &ProgramArtifact,
    inputs: &BTreeMap<String, Vec<f32>>,
) -> Result<u32, GpuAdmissionError> {
    let required = turn_required_nodes(artifact);
    let required_slots = required
        .iter()
        .flat_map(|node| artifact.nodes()[node.get() as usize].input_bindings.clone())
        .filter_map(|binding| match artifact.bindings().get(binding as usize) {
            Some(BindingDeclaration::Input {
                source: ArtifactSource::Slot(slot),
                ..
            }) => Some(*slot),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let slots = artifact
        .slots()
        .iter()
        .map(|slot| (slot.slot, slot.schema))
        .collect::<BTreeMap<_, _>>();
    let mut diagnostics = Vec::new();
    let mut batch = None::<usize>;

    for input in artifact
        .inputs()
        .iter()
        .filter(|input| required_slots.contains(&input.slot))
    {
        let Some(values) = inputs.get(&input.name) else {
            diagnostics.push(GpuDiagnostic {
                code: GpuDiagnosticCode::ShapeMismatch,
                node: None,
                operation: None,
                detail: format!(
                    "broadcast activation is missing required input `{}`",
                    input.name
                ),
            });
            continue;
        };
        let Some(schema) = slots.get(&input.slot) else {
            diagnostics.push(GpuDiagnostic {
                code: GpuDiagnosticCode::ArtifactMalformed,
                node: None,
                operation: None,
                detail: format!("input `{}` refers to an unknown slot", input.name),
            });
            continue;
        };
        let elements = match fixed_shape(artifact, *schema) {
            Ok(shape) => shape.elements(),
            Err(detail) => {
                diagnostics.push(GpuDiagnostic {
                    code: GpuDiagnosticCode::SchemaUnsupported,
                    node: None,
                    operation: None,
                    detail: format!("input `{}`: {detail}", input.name),
                });
                continue;
            }
        };
        if elements == 0 {
            diagnostics.push(GpuDiagnostic {
                code: GpuDiagnosticCode::ShapeMismatch,
                node: None,
                operation: None,
                detail: format!("input `{}` has a zero-sized inner shape", input.name),
            });
            continue;
        }
        if values.is_empty() || values.len() % elements != 0 {
            diagnostics.push(GpuDiagnostic {
                code: GpuDiagnosticCode::ShapeMismatch,
                node: None,
                operation: None,
                detail: format!(
                    "input `{}` has {} element(s); one broadcast item requires {elements}",
                    input.name,
                    values.len()
                ),
            });
            continue;
        }
        let extent = values.len() / elements;
        if extent == 1 {
            continue;
        }
        match batch {
            None => batch = Some(extent),
            Some(expected) if expected == extent => {}
            Some(expected) => diagnostics.push(GpuDiagnostic {
                code: GpuDiagnosticCode::ShapeMismatch,
                node: None,
                operation: None,
                detail: format!(
                    "input `{}` has broadcast extent {extent}, expected 1 or {expected}",
                    input.name
                ),
            }),
        }
    }

    if !diagnostics.is_empty() {
        return Err(GpuAdmissionError { diagnostics });
    }
    let instances = batch.unwrap_or(1);
    u32::try_from(instances).map_err(|_| GpuAdmissionError {
        diagnostics: vec![GpuDiagnostic {
            code: GpuDiagnosticCode::ShapeMismatch,
            node: None,
            operation: None,
            detail: format!("broadcast extent {instances} exceeds the u32 executor limit"),
        }],
    })
}

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

        let predicate_producers = self
            .instructions
            .iter()
            .map(|instruction| (instruction.output, &instruction.computation))
            .collect::<BTreeMap<_, _>>();
        let constraints = self
            .artifact
            .constraints()
            .iter()
            .map(|constraint| {
                let [predicate] = constraint.inputs.as_ref() else {
                    return Err(format!(
                        "integrity constraint {} must have one predicate input",
                        constraint.constraint.get()
                    ));
                };
                Ok(BatchedConstraint {
                    id: constraint.constraint,
                    name: constraint.name.clone().into_boxed_str(),
                    predicate: compile_scalar_predicate(
                        self.operand(*predicate, 0)?,
                        &predicate_producers,
                    ),
                })
            })
            .collect::<Result<Vec<_>, String>>();
        drop(predicate_producers);
        let constraints = match constraints {
            Ok(constraints) if constraints.len() < 256 => constraints,
            Ok(constraints) => {
                self.reject(
                    None,
                    None,
                    format!(
                        "checked batch kernels support at most 255 integrity constraints, found {}",
                        constraints.len()
                    ),
                );
                Vec::new()
            }
            Err(detail) => {
                self.reject(None, None, detail);
                Vec::new()
            }
        };
        if !self.diagnostics.is_empty() {
            return Err(GpuAdmissionError {
                diagnostics: self.diagnostics,
            });
        }
        self.instructions = prune_dead_instructions(
            std::mem::take(&mut self.instructions),
            &self.states,
            &constraints,
        );

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
            &constraints,
        );
        Ok(BatchedGpuProgram {
            instances: self.instances,
            register_count: self.register_count,
            register_offsets: self.register_offsets,
            instructions: self.instructions,
            inputs,
            states,
            constraints,
            wgsl,
        })
    }

    fn collect_slots(&mut self) {
        for slot in self.artifact.slots() {
            // E3 gives public outputs dedicated publication slots. Batched
            // kernels operate on the underlying numeric graph and persistent
            // state, so the output aliases are not registers of their own.
            if slot.role == SlotRole::Output {
                continue;
            }
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
            let operation = display_operation(&node.operation);
            if operation == "core/composite-pack" {
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
            if outputs.iter().any(|slot| self.states.contains_key(slot)) {
                self.lower_state(node.node, &operation, &inputs, &outputs);
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
            let result = if operation == "access/scalar" {
                self.lower_access_2d(output, &inputs)
            } else if operation == "matrix/horzcat" {
                self.lower_concatenate(output, &inputs, true)
            } else if operation == "matrix/vertcat" {
                self.lower_concatenate(output, &inputs, false)
            } else if operation == "matrix/transpose" {
                self.lower_transpose(output, &inputs)
            } else if operation == "matrix/multiply" {
                self.lower_matmul(output, &inputs)
            } else if operation == "matrix/dot" {
                self.lower_dot(output, &inputs)
            } else if operation == "math/neg" {
                self.lower_negate(output, &inputs)
            } else if operation == "math/abs" {
                self.lower_absolute(output, &inputs)
            } else if let Some(comparison) = comparison_operation(&operation) {
                self.lower_compare(output, &inputs, comparison)
            } else if let Some(logic) = logic_operation(&operation) {
                self.lower_logic(output, &inputs, logic)
            } else if let Some(elementwise) = scalar_operation(&operation) {
                self.lower_elementwise(output, &inputs, elementwise)
            } else {
                Err(format!(
                    "generic fixed-shape lowering does not support {operation}"
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
        if name != "core/assign" || inputs.len() != 1 || outputs.len() != 1 {
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

    fn lower_access_2d(
        &mut self,
        output: CellSlotId,
        inputs: &[ArtifactSource],
    ) -> Result<(), String> {
        if inputs.len() != 3 || self.shape(output)?.elements() != 1 {
            return Err(
                "fixed scalar matrix access requires a matrix, row, column, and scalar output"
                    .to_owned(),
            );
        }
        let shape = self.source_shape(inputs[0])?;
        let row = self.constant_index(inputs[1], "row")?;
        let column = self.constant_index(inputs[2], "column")?;
        if row >= shape.rows || column >= shape.columns {
            return Err(format!(
                "matrix access [{},{}] is outside {}x{}",
                row + 1,
                column + 1,
                shape.rows,
                shape.columns
            ));
        }
        self.emit(
            output,
            0,
            ScalarComputation::Copy(self.operand(inputs[0], shape.index(row, column))?),
        );
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

    fn lower_absolute(
        &mut self,
        output: CellSlotId,
        inputs: &[ArtifactSource],
    ) -> Result<(), String> {
        if inputs.len() != 1 || self.shape(output)?.elements() != 1 {
            return Err("absolute value requires one scalar input and output".to_owned());
        }
        self.emit(
            output,
            0,
            ScalarComputation::Absolute(self.operand(inputs[0], 0)?),
        );
        Ok(())
    }

    fn lower_compare(
        &mut self,
        output: CellSlotId,
        inputs: &[ArtifactSource],
        operation: ComparisonOperation,
    ) -> Result<(), String> {
        if inputs.len() != 2 || self.shape(output)?.elements() != 1 {
            return Err("comparison requires two scalar inputs and one scalar output".to_owned());
        }
        let right = self.operand(inputs[1], 0)?;
        if operation == ComparisonOperation::LessEqual
            && matches!(right, ScalarOperand::Constant(value) if value == f32::MAX)
            && let Some(value) = self.absolute_value_input(inputs[0])
        {
            self.emit(
                output,
                0,
                ScalarComputation::IsFinite(self.operand(value, 0)?),
            );
            return Ok(());
        }
        self.emit(
            output,
            0,
            ScalarComputation::Compare {
                operation,
                left: self.operand(inputs[0], 0)?,
                right,
            },
        );
        Ok(())
    }

    fn absolute_value_input(&self, source: ArtifactSource) -> Option<ArtifactSource> {
        let ArtifactSource::Slot(slot) = source else {
            return None;
        };
        let slot = self
            .artifact
            .slots()
            .iter()
            .find(|declaration| declaration.slot == slot)?;
        let ProducerReference::NodeOutput { node, .. } = slot.producer else {
            return None;
        };
        let node = self.artifact.nodes().get(node.get() as usize)?;
        if display_operation(&node.operation) != "math/abs" {
            return None;
        }
        node.input_bindings.clone().find_map(|binding| {
            match self.artifact.bindings().get(binding as usize) {
                Some(BindingDeclaration::Input { source, .. }) => Some(*source),
                _ => None,
            }
        })
    }

    fn lower_logic(
        &mut self,
        output: CellSlotId,
        inputs: &[ArtifactSource],
        operation: LogicOperation,
    ) -> Result<(), String> {
        let expected = usize::from(operation != LogicOperation::Not) + 1;
        if inputs.len() != expected || self.shape(output)?.elements() != 1 {
            return Err(format!(
                "logic operation requires {expected} scalar input(s) and one scalar output"
            ));
        }
        let inputs = inputs
            .iter()
            .map(|source| self.operand(*source, 0))
            .collect::<Result<Vec<_>, _>>()?;
        self.emit(output, 0, ScalarComputation::Logic { operation, inputs });
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

    fn constant_index(&self, source: ArtifactSource, role: &str) -> Result<usize, String> {
        let ArtifactSource::Constant(constant) = source else {
            return Err(format!("matrix {role} index must be compile-time constant"));
        };
        let value = self
            .artifact
            .constants()
            .get(constant)
            .ok_or_else(|| format!("constant {} does not exist", constant.get()))?;
        let ValueData::Index(index) = value.data() else {
            return Err(format!("matrix {role} index is not an index value"));
        };
        let zero_based = index
            .checked_sub(1)
            .ok_or_else(|| format!("matrix {role} index must be at least 1"))?;
        usize::try_from(zero_based).map_err(|_| format!("matrix {role} index does not fit usize"))
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

    match name {
        "math/add" => Some(ElementwiseOperation::Binary(BinaryOperation::Add)),
        "math/sub" => Some(ElementwiseOperation::Binary(BinaryOperation::Subtract)),
        "math/mul" => Some(ElementwiseOperation::Binary(BinaryOperation::Multiply)),
        "math/div" => Some(ElementwiseOperation::Binary(BinaryOperation::Divide)),
        "math/sin" => Some(ElementwiseOperation::Unary(UnaryOperation::Sin)),
        "math/cos" => Some(ElementwiseOperation::Unary(UnaryOperation::Cos)),
        "math/atan2" => Some(ElementwiseOperation::Atan2),
        _ => None,
    }
}

fn comparison_operation(name: &str) -> Option<ComparisonOperation> {
    match name {
        "compare/eq" => Some(ComparisonOperation::Equal),
        "compare/neq" => Some(ComparisonOperation::NotEqual),
        "compare/lte" => Some(ComparisonOperation::LessEqual),
        "compare/lt" => Some(ComparisonOperation::Less),
        "compare/gte" => Some(ComparisonOperation::GreaterEqual),
        "compare/gt" => Some(ComparisonOperation::Greater),
        _ => None,
    }
}

fn logic_operation(name: &str) -> Option<LogicOperation> {
    match name {
        "logic/and" => Some(LogicOperation::And),
        "logic/or" => Some(LogicOperation::Or),
        "logic/xor" => Some(LogicOperation::Xor),
        "logic/not" => Some(LogicOperation::Not),
        _ => None,
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
        SchemaBody::Bool => Ok(FixedShape::scalar()),
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
            SequenceView::F32(values) => {
                let shape = fixed_shape(artifact, value.schema())?;
                // ProgramArtifact snapshots are canonical row-major values;
                // the fixed-matrix register program follows Mech's
                // column-major runtime matrix contract.
                let row_major = values
                    .iter()
                    .map(|value| value.to_f32())
                    .collect::<Vec<_>>();
                let mut column_major = vec![0.0; row_major.len()];
                for row in 0..shape.rows {
                    for column in 0..shape.columns {
                        column_major[shape.index(row, column)] =
                            row_major[row * shape.columns + column];
                    }
                }
                Ok(column_major)
            }
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
    constraints: &[BatchedConstraint],
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
    if !constraints.is_empty() {
        let binding = inputs.len() as u32 + states.len() as u32 * 2;
        shader.push_str(&format!(
            "@group(0) @binding({binding}) var<storage, read_write> integrity_fault: array<atomic<u32>>;\n\n\
             fn record_integrity_fault(code: u32, instance: u32) {{\n\
               atomicAdd(&integrity_fault[0], 1u);\n\
               atomicMin(&integrity_fault[1], (instance << 8u) | code);\n\
             }}\n"
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
    if !constraints.is_empty() {
        shader.push_str("  var integrity_code = 0u;\n");
        for (index, constraint) in constraints.iter().enumerate() {
            let code = index + 1;
            shader.push_str(&format!(
                "  if (integrity_code == 0u && !{}) {{ integrity_code = {code}u; }}\n",
                constraint.predicate.wgsl()
            ));
        }
        shader.push_str(
            "  if (integrity_code != 0u) { record_integrity_fault(integrity_code, index); }\n",
        );
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

    use super::{
        BatchedExecutionError, BatchedFaultRecorder, BatchedGpuProgram, BatchedIntegrityFault,
    };

    const GPU_FAULT_WORDS: usize = 2;

    #[derive(Debug)]
    pub struct BatchedResidentGpuSession {
        adapter: String,
        device: wgpu::Device,
        queue: wgpu::Queue,
        pipeline: wgpu::ComputePipeline,
        bind_groups: [wgpu::BindGroup; 2],
        output_buffers: [BTreeMap<CellSlotId, Arc<wgpu::Buffer>>; 2],
        output_elements: BTreeMap<CellSlotId, usize>,
        constraints: Box<[super::BatchedConstraint]>,
        integrity_fault: Option<Arc<wgpu::Buffer>>,
        integrity_readback: Option<wgpu::Buffer>,
        workgroups: u32,
        next_group: usize,
        last_output_group: Option<usize>,
        faults: BatchedFaultRecorder,
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
            if !self.constraints.is_empty() && self.instances >= (1 << 24) {
                return Err(BatchedExecutionError::IntegrityConfiguration(
                    "checked GPU fault records support fewer than 2^24 instances".to_owned(),
                ));
            }
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
            let required_storage_buffers = (self.inputs.len()
                + self.states.len() * 2
                + usize::from(!self.constraints.is_empty()))
                as u32;
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

            let integrity_fault = (!self.constraints.is_empty()).then(|| {
                Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Mech integrity-constraint fault"),
                    size: (GPU_FAULT_WORDS * std::mem::size_of::<u32>()) as u64,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }))
            });
            let integrity_readback = (!self.constraints.is_empty()).then(|| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Mech integrity-constraint readback"),
                    size: (GPU_FAULT_WORDS * std::mem::size_of::<u32>()) as u64,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                })
            });

            let mut layout_entries =
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
            if !self.constraints.is_empty() {
                layout_entries.push(wgpu::BindGroupLayoutEntry {
                    binding: (self.inputs.len() + self.states.len() * 2) as u32,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                });
            }
            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Mech fixed-shape batch bindings"),
                    entries: &layout_entries,
                });
            let bind_groups = [0, 1].map(|group| {
                let mut entries = self
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
                if let Some(fault) = &integrity_fault {
                    entries.push(wgpu::BindGroupEntry {
                        binding: (self.inputs.len() + self.states.len() * 2) as u32,
                        resource: fault.as_entire_binding(),
                    });
                }
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
                constraints: self.constraints.clone().into_boxed_slice(),
                integrity_fault,
                integrity_readback,
                workgroups: self.workgroup_count(),
                next_group: 0,
                last_output_group: None,
                faults: BatchedFaultRecorder::default(),
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
            if !self.constraints.is_empty() {
                return self.dispatch_checked_turns(turns);
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

        fn dispatch_checked_turns(
            &mut self,
            turns: u32,
        ) -> Result<Duration, BatchedExecutionError> {
            let started = Instant::now();
            for _ in 0..turns {
                let attempted_turn = self.faults.next_turn();
                let fault_buffer = self
                    .integrity_fault
                    .as_ref()
                    .expect("checked GPU session has a fault buffer");
                let cleared_fault = [0_u32, u32::MAX];
                self.queue
                    .write_buffer(fault_buffer, 0, bytemuck::cast_slice(&cleared_fault));
                let group = self.next_group;
                let mut encoder =
                    self.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Mech checked fixed-shape batch turn"),
                        });
                {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Mech checked fixed-shape batch compute pass"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&self.pipeline);
                    pass.set_bind_group(0, &self.bind_groups[group], &[]);
                    pass.dispatch_workgroups(self.workgroups, 1, 1);
                }
                encoder.copy_buffer_to_buffer(
                    fault_buffer,
                    0,
                    self.integrity_readback
                        .as_ref()
                        .expect("checked GPU session has a fault readback"),
                    0,
                    (GPU_FAULT_WORDS * std::mem::size_of::<u32>()) as u64,
                );
                self.queue.submit(Some(encoder.finish()));
                self.device.poll(wgpu::Maintain::Wait);
                let words = self.read_integrity_fault()?;
                if words[0] != 0 {
                    let packed = words[1];
                    let code = packed & 0xff;
                    if code == 0 {
                        return Err(BatchedExecutionError::Native(
                            "GPU returned an empty integrity constraint code".to_owned(),
                        ));
                    }
                    let constraint_index = code as usize - 1;
                    let Some(constraint) = self.constraints.get(constraint_index) else {
                        return Err(BatchedExecutionError::Native(format!(
                            "GPU returned unknown integrity constraint code {code}"
                        )));
                    };
                    let fault = BatchedIntegrityFault {
                        attempted_turn,
                        instance: packed >> 8,
                        constraint: constraint.id,
                        constraint_name: constraint.name.clone(),
                    };
                    return Err(self.faults.record(fault));
                }
                self.last_output_group = Some(group);
                self.next_group = 1 - group;
            }
            Ok(started.elapsed())
        }

        fn read_integrity_fault(&self) -> Result<[u32; GPU_FAULT_WORDS], BatchedExecutionError> {
            let readback = self
                .integrity_readback
                .as_ref()
                .expect("checked GPU session has a fault readback");
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
            let mut words = [0_u32; GPU_FAULT_WORDS];
            words.copy_from_slice(bytemuck::cast_slice::<u8, u32>(&mapped));
            drop(mapped);
            readback.unmap();
            Ok(words)
        }

        pub const fn fault_count(&self) -> u64 {
            self.faults.fault_count
        }

        pub fn last_fault(&self) -> Option<&BatchedIntegrityFault> {
            self.faults.last_fault.as_ref()
        }

        pub fn read_state(
            &self,
        ) -> Result<(Duration, BTreeMap<CellSlotId, Vec<f32>>), BatchedExecutionError> {
            let group = self.last_output_group.ok_or_else(|| {
                BatchedExecutionError::Native("no batch turns have run".to_owned())
            })?;
            self.read_state_group(group)
        }

        /// Reads the currently published state, including the initial state or
        /// the estimate retained after a rejected candidate.
        pub fn read_published_state(
            &self,
        ) -> Result<(Duration, BTreeMap<CellSlotId, Vec<f32>>), BatchedExecutionError> {
            self.read_state_group(1 - self.next_group)
        }

        fn read_state_group(
            &self,
            group: usize,
        ) -> Result<(Duration, BTreeMap<CellSlotId, Vec<f32>>), BatchedExecutionError> {
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
