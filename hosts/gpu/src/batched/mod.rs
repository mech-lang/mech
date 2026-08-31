use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use mech_compute::{
    ComparisonOperation, ComputeKernel, ComputeProgram, FixedShape, FixedShapeConstraint,
    FixedShapeInputStorage, FixedShapeIr, FixedShapeStateStorage, FixedShapeStoragePlan,
    LogicOperation, ScalarComputation, ScalarInstruction, ScalarOperand, ScalarPredicate,
    build_compute_region_interface, plan_compute_artifact,
};
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
const FIXED_SHAPE_INTEGRITY_WORDS: usize = 2;

fn evaluate_operand_simd(operand: ScalarOperand, registers: &[f32x4]) -> f32x4 {
    match operand {
        ScalarOperand::Register(register) => registers[register],
        ScalarOperand::Constant(value) => f32x4::splat(value),
    }
}

fn comparison_mask_simd(operation: ComparisonOperation, left: f32x4, right: f32x4) -> f32x4 {
    match operation {
        ComparisonOperation::Equal => left.cmp_eq(right),
        ComparisonOperation::NotEqual => left.cmp_ne(right),
        ComparisonOperation::Less => left.cmp_lt(right),
        ComparisonOperation::Greater => left.cmp_gt(right),
        ComparisonOperation::LessEqual => left.cmp_le(right),
        ComparisonOperation::GreaterEqual => left.cmp_ge(right),
    }
}

fn evaluate_scalar_computation_simd(computation: &ScalarComputation, registers: &[f32x4]) -> f32x4 {
    match computation {
        ScalarComputation::Copy(input) => evaluate_operand_simd(*input, registers),
        ScalarComputation::Negate(input) => -evaluate_operand_simd(*input, registers),
        ScalarComputation::Absolute(input) => evaluate_operand_simd(*input, registers).abs(),
        ScalarComputation::IsFinite(input) => evaluate_operand_simd(*input, registers)
            .is_finite()
            .blend(f32x4::ONE, f32x4::ZERO),
        ScalarComputation::Compare {
            operation,
            left,
            right,
        } => comparison_mask_simd(
            *operation,
            evaluate_operand_simd(*left, registers),
            evaluate_operand_simd(*right, registers),
        )
        .blend(f32x4::ONE, f32x4::ZERO),
        ScalarComputation::Logic { operation, inputs } => {
            let left = evaluate_operand_simd(inputs[0], registers).cmp_ne(f32x4::ZERO);
            let right = inputs
                .get(1)
                .map(|input| evaluate_operand_simd(*input, registers).cmp_ne(f32x4::ZERO));
            let mask = match operation {
                LogicOperation::And => left & right.unwrap(),
                LogicOperation::Or => left | right.unwrap(),
                LogicOperation::Xor => left ^ right.unwrap(),
                LogicOperation::Not => !left,
            };
            mask.blend(f32x4::ONE, f32x4::ZERO)
        }
        ScalarComputation::Elementwise { operation, inputs } => {
            let mut values = [f32x4::ZERO; 2];
            for (index, input) in inputs.iter().enumerate() {
                values[index] = evaluate_operand_simd(*input, registers);
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
                    UnaryOperation::Sqrt => values[0].sqrt(),
                    UnaryOperation::Ceil => values[0].ceil(),
                },
                ElementwiseOperation::Atan2 => values[0].atan2(values[1]),
                ElementwiseOperation::Identity => values[0],
            }
        }
        ScalarComputation::SumProducts(terms) => {
            terms.iter().fold(f32x4::ZERO, |sum, (left, right)| {
                evaluate_operand_simd(*left, registers)
                    .mul_add(evaluate_operand_simd(*right, registers), sum)
            })
        }
    }
}

fn evaluate_predicate_simd_mask(predicate: &ScalarPredicate, registers: &[f32x4]) -> f32x4 {
    match predicate {
        ScalarPredicate::Value(value) => {
            evaluate_operand_simd(*value, registers).cmp_ne(f32x4::ZERO)
        }
        ScalarPredicate::IsFinite(value) => evaluate_operand_simd(*value, registers).is_finite(),
        ScalarPredicate::AbsoluteDifferenceWithin {
            left,
            right,
            tolerance,
        } => (evaluate_operand_simd(*left, registers) - evaluate_operand_simd(*right, registers))
            .abs()
            .cmp_le(evaluate_operand_simd(*tolerance, registers)),
        ScalarPredicate::Compare {
            operation,
            left,
            right,
        } => comparison_mask_simd(
            *operation,
            evaluate_operand_simd(*left, registers),
            evaluate_operand_simd(*right, registers),
        ),
        ScalarPredicate::All(inputs) => inputs.iter().fold(!f32x4::ZERO, |mask, input| {
            mask & evaluate_predicate_simd_mask(input, registers)
        }),
        ScalarPredicate::Logic { operation, inputs } => {
            let left = evaluate_predicate_simd_mask(&inputs[0], registers);
            let right = inputs
                .get(1)
                .map(|input| evaluate_predicate_simd_mask(input, registers));
            match operation {
                LogicOperation::And => left & right.unwrap(),
                LogicOperation::Or => left | right.unwrap(),
                LogicOperation::Xor => left ^ right.unwrap(),
                LogicOperation::Not => !left,
            }
        }
    }
}

fn scalar_operand_wgsl(operand: ScalarOperand) -> String {
    match operand {
        ScalarOperand::Register(register) => format!("r{register}"),
        ScalarOperand::Constant(value) => super::format_wgsl_f32(value),
    }
}

fn comparison_wgsl(operation: ComparisonOperation) -> &'static str {
    match operation {
        ComparisonOperation::Equal => "==",
        ComparisonOperation::NotEqual => "!=",
        ComparisonOperation::Less => "<",
        ComparisonOperation::Greater => ">",
        ComparisonOperation::LessEqual => "<=",
        ComparisonOperation::GreaterEqual => ">=",
    }
}

fn scalar_predicate_wgsl(predicate: &ScalarPredicate) -> String {
    scalar_predicate_wgsl_with_aliases(predicate, &BTreeMap::new())
}

fn scalar_predicate_wgsl_with_aliases(
    predicate: &ScalarPredicate,
    aliases: &BTreeMap<usize, ScalarOperand>,
) -> String {
    match predicate {
        ScalarPredicate::Value(value) => format!("({} != 0.0)", fast_operand_wgsl(*value, aliases)),
        ScalarPredicate::IsFinite(value) => {
            format!(
                "(abs({}) <= 3.402823466e38)",
                fast_operand_wgsl(*value, aliases)
            )
        }
        ScalarPredicate::AbsoluteDifferenceWithin {
            left,
            right,
            tolerance,
        } => format!(
            "(abs(({}) - ({})) <= ({}))",
            fast_operand_wgsl(*left, aliases),
            fast_operand_wgsl(*right, aliases),
            fast_operand_wgsl(*tolerance, aliases)
        ),
        ScalarPredicate::Compare {
            operation,
            left,
            right,
        } => format!(
            "(({}) {} ({}))",
            fast_operand_wgsl(*left, aliases),
            comparison_wgsl(*operation),
            fast_operand_wgsl(*right, aliases)
        ),
        ScalarPredicate::All(inputs) => format!(
            "({})",
            inputs
                .iter()
                .map(|input| scalar_predicate_wgsl_with_aliases(input, aliases))
                .collect::<Vec<_>>()
                .join(" && ")
        ),
        ScalarPredicate::Logic { operation, inputs } => {
            let left = scalar_predicate_wgsl_with_aliases(&inputs[0], aliases);
            match operation {
                LogicOperation::And => {
                    format!(
                        "({left} && {})",
                        scalar_predicate_wgsl_with_aliases(&inputs[1], aliases)
                    )
                }
                LogicOperation::Or => {
                    format!(
                        "({left} || {})",
                        scalar_predicate_wgsl_with_aliases(&inputs[1], aliases)
                    )
                }
                LogicOperation::Xor => {
                    format!(
                        "({left} != {})",
                        scalar_predicate_wgsl_with_aliases(&inputs[1], aliases)
                    )
                }
                LogicOperation::Not => format!("(!{left})"),
            }
        }
    }
}

fn scalar_computation_wgsl(computation: &ScalarComputation) -> String {
    match computation {
        ScalarComputation::Copy(input) => scalar_operand_wgsl(*input),
        ScalarComputation::Negate(input) => format!("-({})", scalar_operand_wgsl(*input)),
        ScalarComputation::Absolute(input) => format!("abs({})", scalar_operand_wgsl(*input)),
        ScalarComputation::IsFinite(input) => format!(
            "select(0.0, 1.0, abs({}) <= 3.402823466e38)",
            scalar_operand_wgsl(*input)
        ),
        ScalarComputation::Compare {
            operation,
            left,
            right,
        } => format!(
            "select(0.0, 1.0, ({}) {} ({}))",
            scalar_operand_wgsl(*left),
            comparison_wgsl(*operation),
            scalar_operand_wgsl(*right)
        ),
        ScalarComputation::Logic { operation, inputs } => {
            let left = format!("({} != 0.0)", scalar_operand_wgsl(inputs[0]));
            let condition = match operation {
                LogicOperation::And => {
                    format!("{left} && ({} != 0.0)", scalar_operand_wgsl(inputs[1]))
                }
                LogicOperation::Or => {
                    format!("{left} || ({} != 0.0)", scalar_operand_wgsl(inputs[1]))
                }
                LogicOperation::Xor => {
                    format!("{left} != ({} != 0.0)", scalar_operand_wgsl(inputs[1]))
                }
                LogicOperation::Not => format!("!{left}"),
            };
            format!("select(0.0, 1.0, {condition})")
        }
        ScalarComputation::Elementwise { operation, inputs } => {
            let inputs = inputs
                .iter()
                .map(|input| scalar_operand_wgsl(*input))
                .collect::<Vec<_>>();
            super::wgsl_elementwise_expression(*operation, &inputs)
        }
        ScalarComputation::SumProducts(terms) => {
            let mut iter = terms.iter();
            let Some((left, right)) = iter.next() else {
                return "0.0".to_owned();
            };
            let mut expression = format!(
                "({} * {})",
                scalar_operand_wgsl(*left),
                scalar_operand_wgsl(*right)
            );
            for (left, right) in iter {
                expression = format!(
                    "fma({}, {}, {})",
                    scalar_operand_wgsl(*left),
                    scalar_operand_wgsl(*right),
                    expression
                );
            }
            expression
        }
    }
}

enum FastWgslInstruction {
    Alias(ScalarOperand),
    Expression(String),
}

enum FastSumTerm {
    Product(String, String),
    Value(String),
}

fn resolve_fast_operand(
    mut operand: ScalarOperand,
    aliases: &BTreeMap<usize, ScalarOperand>,
) -> ScalarOperand {
    let mut seen = BTreeSet::new();
    while let ScalarOperand::Register(register) = operand {
        if !seen.insert(register) {
            break;
        }
        let Some(next) = aliases.get(&register).copied() else {
            break;
        };
        operand = next;
    }
    operand
}

fn fast_operand_wgsl(operand: ScalarOperand, aliases: &BTreeMap<usize, ScalarOperand>) -> String {
    scalar_operand_wgsl(resolve_fast_operand(operand, aliases))
}

fn fast_wgsl_instruction(
    computation: &ScalarComputation,
    aliases: &BTreeMap<usize, ScalarOperand>,
) -> FastWgslInstruction {
    match computation {
        ScalarComputation::Copy(input) => {
            FastWgslInstruction::Alias(resolve_fast_operand(*input, aliases))
        }
        ScalarComputation::Negate(input) => {
            let input = resolve_fast_operand(*input, aliases);
            match input {
                ScalarOperand::Constant(value) => {
                    FastWgslInstruction::Alias(ScalarOperand::Constant(-value))
                }
                _ => FastWgslInstruction::Expression(format!("-({})", scalar_operand_wgsl(input))),
            }
        }
        ScalarComputation::Absolute(input) => {
            let input = resolve_fast_operand(*input, aliases);
            match input {
                ScalarOperand::Constant(value) => {
                    FastWgslInstruction::Alias(ScalarOperand::Constant(value.abs()))
                }
                _ => {
                    FastWgslInstruction::Expression(format!("abs({})", scalar_operand_wgsl(input)))
                }
            }
        }
        ScalarComputation::Elementwise { operation, inputs } => {
            let inputs = inputs
                .iter()
                .map(|input| resolve_fast_operand(*input, aliases))
                .collect::<Vec<_>>();
            match operation {
                ElementwiseOperation::Identity => FastWgslInstruction::Alias(inputs[0]),
                ElementwiseOperation::Binary(operation) => {
                    if let Some(alias) = simplify_fast_binary(*operation, inputs[0], inputs[1]) {
                        FastWgslInstruction::Alias(alias)
                    } else {
                        FastWgslInstruction::Expression(format!(
                            "({})",
                            super::wgsl_elementwise_expression(
                                ElementwiseOperation::Binary(*operation),
                                &inputs
                                    .iter()
                                    .map(|input| scalar_operand_wgsl(*input))
                                    .collect::<Vec<_>>(),
                            )
                        ))
                    }
                }
                ElementwiseOperation::Unary(operation) => {
                    if let ScalarOperand::Constant(value) = inputs[0] {
                        FastWgslInstruction::Alias(ScalarOperand::Constant(operation.apply(value)))
                    } else {
                        FastWgslInstruction::Expression(super::wgsl_elementwise_expression(
                            ElementwiseOperation::Unary(*operation),
                            &[scalar_operand_wgsl(inputs[0])],
                        ))
                    }
                }
                ElementwiseOperation::Atan2 => {
                    if let (ScalarOperand::Constant(left), ScalarOperand::Constant(right)) =
                        (inputs[0], inputs[1])
                    {
                        FastWgslInstruction::Alias(ScalarOperand::Constant(left.atan2(right)))
                    } else {
                        FastWgslInstruction::Expression(super::wgsl_elementwise_expression(
                            *operation,
                            &inputs
                                .iter()
                                .map(|input| scalar_operand_wgsl(*input))
                                .collect::<Vec<_>>(),
                        ))
                    }
                }
            }
        }
        ScalarComputation::SumProducts(terms) => {
            let mut sum_terms = Vec::new();
            for (left, right) in terms {
                let left = resolve_fast_operand(*left, aliases);
                let right = resolve_fast_operand(*right, aliases);
                if is_zero_fast(left) || is_zero_fast(right) {
                    continue;
                }
                match (left, right) {
                    (ScalarOperand::Constant(left), ScalarOperand::Constant(right)) => {
                        sum_terms.push(FastSumTerm::Value(super::format_wgsl_f32(left * right)));
                    }
                    (ScalarOperand::Constant(1.0), value)
                    | (value, ScalarOperand::Constant(1.0)) => {
                        sum_terms.push(FastSumTerm::Value(scalar_operand_wgsl(value)));
                    }
                    (left, right) => {
                        sum_terms.push(FastSumTerm::Product(
                            scalar_operand_wgsl(left),
                            scalar_operand_wgsl(right),
                        ));
                    }
                }
            }
            if sum_terms.is_empty() {
                FastWgslInstruction::Alias(ScalarOperand::Constant(0.0))
            } else {
                let mut expression = match sum_terms.remove(0) {
                    FastSumTerm::Product(left, right) => format!("({left} * {right})"),
                    FastSumTerm::Value(value) => value,
                };
                for term in sum_terms {
                    expression = match term {
                        FastSumTerm::Product(left, right) => {
                            format!("fma({left}, {right}, {expression})")
                        }
                        FastSumTerm::Value(value) => format!("({expression} + {value})"),
                    };
                }
                FastWgslInstruction::Expression(expression)
            }
        }
        ScalarComputation::IsFinite(input) => FastWgslInstruction::Expression(format!(
            "select(0.0, 1.0, abs({}) <= 3.402823466e38)",
            fast_operand_wgsl(*input, aliases)
        )),
        ScalarComputation::Compare {
            operation,
            left,
            right,
        } => FastWgslInstruction::Expression(format!(
            "select(0.0, 1.0, ({}) {} ({}))",
            fast_operand_wgsl(*left, aliases),
            comparison_wgsl(*operation),
            fast_operand_wgsl(*right, aliases)
        )),
        ScalarComputation::Logic { operation, inputs } => {
            let left = format!("({} != 0.0)", fast_operand_wgsl(inputs[0], aliases));
            let condition = match operation {
                LogicOperation::And => format!(
                    "{left} && ({} != 0.0)",
                    fast_operand_wgsl(inputs[1], aliases)
                ),
                LogicOperation::Or => format!(
                    "{left} || ({} != 0.0)",
                    fast_operand_wgsl(inputs[1], aliases)
                ),
                LogicOperation::Xor => format!(
                    "{left} != ({} != 0.0)",
                    fast_operand_wgsl(inputs[1], aliases)
                ),
                LogicOperation::Not => format!("(!{left})"),
            };
            FastWgslInstruction::Expression(format!("select(0.0, 1.0, {condition})"))
        }
    }
}

fn is_zero_fast(operand: ScalarOperand) -> bool {
    matches!(operand, ScalarOperand::Constant(value) if value == 0.0)
}

fn simplify_fast_binary(
    operation: BinaryOperation,
    left: ScalarOperand,
    right: ScalarOperand,
) -> Option<ScalarOperand> {
    if let (ScalarOperand::Constant(left), ScalarOperand::Constant(right)) = (left, right) {
        return Some(ScalarOperand::Constant(operation.apply(left, right)));
    }
    match operation {
        BinaryOperation::Add if is_zero_fast(right) => Some(left),
        BinaryOperation::Add if is_zero_fast(left) => Some(right),
        BinaryOperation::Subtract if is_zero_fast(right) => Some(left),
        BinaryOperation::Multiply if is_zero_fast(left) || is_zero_fast(right) => {
            Some(ScalarOperand::Constant(0.0))
        }
        BinaryOperation::Multiply if is_one_fast(left) => Some(right),
        BinaryOperation::Multiply if is_one_fast(right) => Some(left),
        BinaryOperation::Divide if is_one_fast(right) => Some(left),
        _ => None,
    }
}

fn is_one_fast(operand: ScalarOperand) -> bool {
    matches!(operand, ScalarOperand::Constant(value) if value == 1.0)
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
    // `abs(x) <= f32::MAX` is the source-level spelling used by older Mech
    // programs for a finite-value constraint. Lower it to the dedicated
    // predicate so native backends share one finite-value representation with
    // the scalar planner instead of treating this as an arbitrary comparison.
    if operation == ComparisonOperation::LessEqual
        && let ScalarOperand::Register(absolute_register) = left
        && let Some(ScalarComputation::Absolute(input)) = producers.get(&absolute_register).copied()
        && let ScalarOperand::Constant(limit) = right
        && limit.to_bits() == f32::MAX.to_bits()
    {
        return ScalarPredicate::IsFinite(*input);
    }
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

fn prune_dead_batched_instructions(
    instructions: Vec<ScalarInstruction>,
    states: &[BatchedState],
) -> Vec<ScalarInstruction> {
    let mut live = BTreeSet::new();
    for state in states {
        for source in &state.update {
            collect_operand_register(*source, &mut live);
        }
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
pub struct FixedShapeKernel {
    instances: u32,
    compute: ComputeProgram,
    register_offsets: BTreeMap<CellSlotId, usize>,
    instructions: Box<[ScalarInstruction]>,
    inputs: Vec<BatchedInput>,
    states: Vec<BatchedState>,
    constraints: Vec<BatchedConstraint>,
    wgsl: String,
}

/// One immutable browser/native storage binding for a fixed-shape input after
/// scalar or per-lane source values have been expanded to the physical batch.
#[derive(Clone, Debug, PartialEq)]
pub struct FixedShapeInputBuffer {
    pub slot: CellSlotId,
    pub name: String,
    pub binding: u32,
    pub elements: usize,
    pub initial_values: Vec<f32>,
}

/// Ping-pong storage metadata for one resident fixed-shape state. The physical
/// buffer is lane-contiguous, so one sampled lane can later be copied without
/// materializing the rest of the batch on the CPU.
#[derive(Clone, Debug, PartialEq)]
pub struct FixedShapeStateBuffer {
    pub slot: CellSlotId,
    pub read_binding: u32,
    pub write_binding: u32,
    pub elements_per_instance: usize,
    pub elements: usize,
    pub initial_values: Vec<f32>,
}

/// Optional two-word atomic fault buffer used by checked fixed-shape kernels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedShapeIntegrityBuffer {
    pub binding: u32,
    pub words: usize,
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

    const fn attempted_turns(&self) -> u64 {
        self.attempted_turns
    }
}

#[derive(Debug)]
pub struct BatchedCpuSession {
    program: Arc<FixedShapeKernel>,
    inputs: BTreeMap<CellSlotId, Vec<f32>>,
    state: BTreeMap<CellSlotId, Vec<f32>>,
    next_state: BTreeMap<CellSlotId, Vec<f32>>,
    registers: Vec<f32>,
    faults: BatchedFaultRecorder,
}

#[derive(Debug)]
pub struct BatchedSimdCpuSession {
    program: Arc<FixedShapeKernel>,
    inputs: BTreeMap<CellSlotId, Vec<f32>>,
    state: BTreeMap<CellSlotId, Vec<f32>>,
    next_state: BTreeMap<CellSlotId, Vec<f32>>,
    registers: Vec<f32x4>,
    faults: BatchedFaultRecorder,
}

impl super::ComputeLowerer {
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
    ) -> Result<FixedShapeKernel, GpuAdmissionError> {
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
    ) -> Result<FixedShapeKernel, GpuAdmissionError> {
        self.compile_broadcast_for_regions(artifact, artifact.compute_regions(), inputs)
    }

    /// Admits one named compute region and derives its outer extent from the
    /// region's activation arrays.
    fn compile_broadcast_for_regions(
        &self,
        artifact: &ProgramArtifact,
        regions: &[ComputeRegionDeclaration],
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<FixedShapeKernel, GpuAdmissionError> {
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
        let instances = infer_broadcast_instances(artifact, inputs)?;
        BatchCompiler::new(artifact, instances).compile()
    }
}

impl FixedShapeKernel {
    pub fn from_compute_program(program: &ComputeProgram) -> Result<Self, GpuAdmissionError> {
        let Some(storage) = program.fixed_shape_storage() else {
            return Err(fixed_shape_program_error(
                "fixed-shape program has no resident storage plan",
            ));
        };
        if !matches!(program.kernel(), ComputeKernel::FixedShape(_)) {
            return Err(fixed_shape_program_error(
                "resident fixed-shape storage requires a fixed-shape kernel",
            ));
        }
        let state_slots = storage
            .states
            .iter()
            .map(|state| state.slot)
            .collect::<BTreeSet<_>>();
        if let Some(output) = program
            .interface()
            .outputs
            .iter()
            .find(|output| !state_slots.contains(&output.slot))
        {
            return Err(fixed_shape_program_error(format!(
                "fixed-shape output `{}` is derived storage; browser and native GPU backends require every published output to be resident state",
                output.name,
            )));
        }

        let mut binding = 0_u32;
        let inputs = storage
            .inputs
            .iter()
            .map(|input| {
                let physical = BatchedInput {
                    slot: input.slot,
                    name: input.name.to_string(),
                    shape: input.shape,
                    binding,
                };
                binding += 1;
                physical
            })
            .collect::<Vec<_>>();
        let states = storage
            .states
            .iter()
            .map(|state| {
                let physical = BatchedState {
                    slot: state.slot,
                    shape: state.shape,
                    initializer: state.initializer.to_vec(),
                    update: state.update.to_vec(),
                    read_binding: binding,
                    write_binding: binding + 1,
                };
                binding += 2;
                physical
            })
            .collect::<Vec<_>>();
        let constraints = storage
            .constraints
            .iter()
            .map(|constraint| BatchedConstraint {
                id: constraint.id,
                name: constraint.name.clone(),
                predicate: constraint.predicate.clone(),
            })
            .collect::<Vec<_>>();
        let ComputeKernel::FixedShape(ir) = program.kernel() else {
            unreachable!("kernel kind was checked above")
        };
        let wgsl = generate_wgsl(
            storage.instances,
            &storage.register_offsets,
            &ir.instructions,
            &inputs,
            &states,
            &constraints,
        );
        Ok(Self {
            instances: storage.instances,
            compute: program.clone(),
            register_offsets: storage.register_offsets.clone(),
            instructions: ir.instructions.clone(),
            inputs,
            states,
            constraints,
            wgsl,
        })
    }

    pub fn compute_program(&self) -> &ComputeProgram {
        &self.compute
    }

    fn fixed_ir(&self) -> &FixedShapeIr {
        let ComputeKernel::FixedShape(ir) = self.compute.kernel() else {
            unreachable!("fixed-shape batch contains an elementwise kernel")
        };
        ir
    }

    pub const fn instances(&self) -> u32 {
        self.instances
    }

    pub const fn workgroup_count(&self) -> u32 {
        self.instances.div_ceil(WORKGROUP_SIZE)
    }

    pub fn wgsl(&self) -> &str {
        &self.wgsl
    }

    /// Returns a copy of this kernel with integrity predicates removed from
    /// the generated device program. This is an explicit opt-in execution
    /// mode for callers that accept unchecked state publication; the source
    /// artifact and checked kernel remain unchanged.
    ///
    /// Keeping this as a separate kernel is important for a fair performance
    /// comparison: an unchecked dispatch must not pay for predicate
    /// evaluation or carry an otherwise-unused atomic fault binding.
    pub fn without_integrity_constraints(&self) -> Self {
        let mut unchecked = self.clone();
        unchecked.constraints.clear();
        unchecked.compute = self.compute.clone().without_integrity_constraints();
        unchecked.instructions =
            prune_dead_batched_instructions(self.instructions.to_vec(), &unchecked.states)
                .into_boxed_slice();
        unchecked.wgsl = generate_wgsl_unchecked(
            unchecked.instances,
            &unchecked.register_offsets,
            &unchecked.instructions,
            &unchecked.inputs,
            &unchecked.states,
        );
        unchecked
    }

    /// Returns the unchecked in-place shader used by backends that explicitly
    /// opt out of rollback publication. Every state value is loaded before its
    /// corresponding write, so one resident state buffer is sufficient.
    #[cfg(feature = "native")]
    pub fn unchecked_in_place_wgsl(&self) -> Result<String, BatchedExecutionError> {
        if !self.constraints.is_empty() {
            return Err(BatchedExecutionError::Native(
                "unchecked in-place WGSL requires a kernel without integrity constraints"
                    .to_owned(),
            ));
        }
        Ok(generate_wgsl_unchecked_in_place(
            self.instances,
            &self.register_offsets,
            &self.instructions,
            &self.inputs,
            &self.states,
        ))
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

    /// Returns the scalar extent of each physical input before batch expansion.
    /// Backends that know the inferred batch size can use this metadata to
    /// specialize runtime arrays without reaching into the lowering plan.
    pub fn input_layout(&self) -> impl Iterator<Item = (CellSlotId, usize)> + '_ {
        self.inputs
            .iter()
            .map(|input| (input.slot, input.shape.elements()))
    }

    /// Materializes the backend-neutral physical input plan used by both the
    /// native wgpu session and the browser WebGPU bridge.
    pub fn physical_inputs(
        &self,
        provided: &BTreeMap<String, Vec<f32>>,
    ) -> Result<Vec<FixedShapeInputBuffer>, BatchedExecutionError> {
        self.inputs
            .iter()
            .map(|input| {
                let values = provided
                    .get(&input.name)
                    .ok_or_else(|| BatchedExecutionError::MissingInput(input.name.clone()))?;
                let initial_values = self.expand_input(input, values)?;
                Ok(FixedShapeInputBuffer {
                    slot: input.slot,
                    name: input.name.clone(),
                    binding: input.binding,
                    elements: initial_values.len(),
                    initial_values,
                })
            })
            .collect()
    }

    /// Materializes the initial ping-pong state plan without selecting a GPU
    /// API. Each instance owns one contiguous fixed-shape value.
    pub fn physical_states(&self) -> Vec<FixedShapeStateBuffer> {
        self.states
            .iter()
            .map(|state| {
                let elements_per_instance = state.shape.elements();
                let initial_values = state
                    .initializer
                    .iter()
                    .copied()
                    .cycle()
                    .take(elements_per_instance * self.instances as usize)
                    .collect::<Vec<_>>();
                FixedShapeStateBuffer {
                    slot: state.slot,
                    read_binding: state.read_binding,
                    write_binding: state.write_binding,
                    elements_per_instance,
                    elements: initial_values.len(),
                    initial_values,
                }
            })
            .collect()
    }

    pub fn integrity_buffer(&self) -> Option<FixedShapeIntegrityBuffer> {
        (!self.constraints.is_empty()).then_some(FixedShapeIntegrityBuffer {
            binding: (self.inputs.len() + self.states.len() * 2) as u32,
            words: FIXED_SHAPE_INTEGRITY_WORDS,
        })
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
    ) -> Result<BatchedCpuSession, BatchedExecutionError> {
        let inputs = self.expand_inputs(inputs)?;
        let state = self.initial_state();
        let next_state = state
            .iter()
            .map(|(slot, values)| (*slot, vec![0.0; values.len()]))
            .collect();
        Ok(BatchedCpuSession {
            program: Arc::new(self.clone()),
            inputs,
            state,
            next_state,
            registers: vec![0.0; self.fixed_ir().register_count],
            faults: BatchedFaultRecorder::default(),
        })
    }

    /// Prepares a four-lane `f32` CPU executor for the same scalarized region
    /// used by the scalar CPU and GPU backends.
    pub fn prepare_simd_cpu(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedSimdCpuSession, BatchedExecutionError> {
        let inputs = self.expand_inputs(inputs)?;
        let state = self.initial_state();
        let next_state = state
            .iter()
            .map(|(slot, values)| (*slot, vec![0.0; values.len()]))
            .collect();
        Ok(BatchedSimdCpuSession {
            program: Arc::new(self.clone()),
            inputs,
            state,
            next_state,
            registers: vec![f32x4::ZERO; self.fixed_ir().register_count],
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
                let expanded = self.expand_input(input, values)?;
                Ok((input.slot, expanded))
            })
            .collect()
    }

    fn expand_input(
        &self,
        input: &BatchedInput,
        values: &[f32],
    ) -> Result<Vec<f32>, BatchedExecutionError> {
        let elements = input.shape.elements();
        let batch_elements = elements * self.instances as usize;
        if values.len() == elements {
            Ok(values
                .iter()
                .copied()
                .cycle()
                .take(batch_elements)
                .collect())
        } else if values.len() == batch_elements {
            Ok(values.to_vec())
        } else {
            Err(BatchedExecutionError::InputLength {
                name: input.name.clone(),
                expected_single: elements,
                expected_batch: batch_elements,
                actual: values.len(),
            })
        }
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

    #[cfg(feature = "jit")]
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

impl BatchedCpuSession {
    pub fn update_inputs(
        &mut self,
        updates: &BTreeMap<String, Vec<f32>>,
    ) -> Result<(), BatchedExecutionError> {
        for (name, values) in updates {
            let input = self
                .program
                .inputs
                .iter()
                .find(|input| input.name == *name)
                .ok_or_else(|| BatchedExecutionError::MissingInput(name.clone()))?;
            self.inputs
                .insert(input.slot, self.program.expand_input(input, values)?);
        }
        Ok(())
    }

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
                for instruction in &self.program.fixed_ir().instructions {
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

    pub const fn attempted_turns(&self) -> u64 {
        self.faults.attempted_turns()
    }

    pub fn last_fault(&self) -> Option<&BatchedIntegrityFault> {
        self.faults.last_fault.as_ref()
    }
}

impl BatchedSimdCpuSession {
    pub fn update_inputs(
        &mut self,
        updates: &BTreeMap<String, Vec<f32>>,
    ) -> Result<(), BatchedExecutionError> {
        for (name, values) in updates {
            let input = self
                .program
                .inputs
                .iter()
                .find(|input| input.name == *name)
                .ok_or_else(|| BatchedExecutionError::MissingInput(name.clone()))?;
            self.inputs
                .insert(input.slot, self.program.expand_input(input, values)?);
        }
        Ok(())
    }

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
                for instruction in &self.program.fixed_ir().instructions {
                    self.registers[instruction.output] =
                        evaluate_scalar_computation_simd(&instruction.computation, &self.registers);
                }
                for constraint in &self.program.constraints {
                    let valid_mask =
                        evaluate_predicate_simd_mask(&constraint.predicate, &self.registers);
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
                        let lanes = evaluate_operand_simd(*source, &self.registers).to_array();
                        if first_instance + SIMD_LANES <= instances {
                            let base = first_instance * elements + component;
                            destination[base] = lanes[0];
                            destination[base + elements] = lanes[1];
                            destination[base + elements * 2] = lanes[2];
                            destination[base + elements * 3] = lanes[3];
                        } else {
                            for (lane, value) in lanes.into_iter().enumerate() {
                                let instance = first_instance + lane;
                                if instance < instances {
                                    destination[instance * elements + component] = value;
                                }
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

    pub const fn attempted_turns(&self) -> u64 {
        self.faults.attempted_turns()
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
    if first_instance + SIMD_LANES <= instances {
        let base = first_instance * elements + component;
        return f32x4::new([
            values[base],
            values[base + elements],
            values[base + elements * 2],
            values[base + elements * 3],
        ]);
    }
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

    fn compile(mut self) -> Result<FixedShapeKernel, GpuAdmissionError> {
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
        let interface =
            build_compute_region_interface(self.artifact, self.artifact.compute_regions().first())?;
        let plan = plan_compute_artifact(self.artifact, self.artifact.compute_regions());
        let kernel = ComputeKernel::FixedShape(FixedShapeIr {
            register_count: self.register_count,
            instructions: self.instructions.into_boxed_slice(),
        });
        let storage = FixedShapeStoragePlan {
            instances: self.instances,
            register_offsets: self.register_offsets,
            inputs: inputs
                .iter()
                .map(|input| FixedShapeInputStorage {
                    slot: input.slot,
                    name: input.name.clone().into_boxed_str(),
                    shape: input.shape,
                })
                .collect(),
            states: states
                .iter()
                .map(|state| FixedShapeStateStorage {
                    slot: state.slot,
                    shape: state.shape,
                    initializer: state.initializer.clone().into(),
                    update: state.update.clone().into_boxed_slice(),
                })
                .collect(),
            constraints: constraints
                .iter()
                .map(|constraint| FixedShapeConstraint {
                    id: constraint.id,
                    name: constraint.name.clone(),
                    predicate: constraint.predicate.clone(),
                })
                .collect(),
        };
        let program =
            ComputeProgram::new(interface, plan, kernel).with_fixed_shape_storage(storage);
        FixedShapeKernel::from_compute_program(&program)
    }

    fn collect_slots(&mut self) {
        let required_nodes = turn_required_nodes(self.artifact);
        let required_slots = required_nodes
            .iter()
            .flat_map(|node| {
                let node = &self.artifact.nodes()[node.get() as usize];
                node.input_bindings
                    .clone()
                    .filter_map(
                        |binding| match self.artifact.bindings().get(binding as usize) {
                            Some(BindingDeclaration::Input {
                                source: ArtifactSource::Slot(slot),
                                ..
                            }) => Some(*slot),
                            _ => None,
                        },
                    )
                    .chain(node.output_bindings.clone().filter_map(|binding| {
                        match self.artifact.bindings().get(binding as usize) {
                            Some(BindingDeclaration::Output { target, .. }) => Some(*target),
                            _ => None,
                        }
                    }))
            })
            .collect::<BTreeSet<_>>();
        for slot in self.artifact.slots() {
            // E3 gives public outputs dedicated publication slots. Batched
            // kernels operate on the underlying numeric graph and persistent
            // state, so the output aliases are not registers of their own.
            if slot.role == SlotRole::Output {
                continue;
            }
            // The semantic artifact can retain selector temporaries whose
            // constant result was folded directly into a later access node.
            // They are not part of the executable turn graph and may use
            // selector-only schemas (for example a matrix of indices), so do
            // not allocate numeric registers or reject them as data slots.
            if !required_slots.contains(&slot.slot) {
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
            if self.static_selector_slot(slot.slot) {
                continue;
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
            if !outputs.is_empty()
                && outputs
                    .iter()
                    .all(|output| self.static_selector_slot(*output))
            {
                continue;
            }
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
            let result = if operation == "access/scalar" || operation == "access/range" {
                self.lower_access(output, &inputs)
            } else if operation == "matrix/horzcat" {
                self.lower_concatenate(output, &inputs, true)
            } else if operation == "matrix/vertcat" {
                self.lower_concatenate(output, &inputs, false)
            } else if operation == "matrix/transpose" {
                self.lower_transpose(output, &inputs)
            } else if operation == "matrix/multiply" {
                self.lower_matmul(output, &inputs)
            } else if operation == "matrix/solve" {
                self.lower_solve(output, &inputs)
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

    fn lower_access(
        &mut self,
        output: CellSlotId,
        inputs: &[ArtifactSource],
    ) -> Result<(), String> {
        if inputs.len() != 2 && inputs.len() != 3 {
            return Err(format!(
                "fixed matrix access requires a source and one or two static selectors, found {} inputs",
                inputs.len()
            ));
        }
        let source = self.source_shape(inputs[0])?;
        let result = self.shape(output)?;
        let (rows, columns) = if inputs.len() == 3 {
            (
                self.constant_indices(inputs[1], source.rows, "row")?,
                self.constant_indices(inputs[2], source.columns, "column")?,
            )
        } else {
            let selector =
                self.constant_indices(inputs[1], source.rows.max(source.columns), "matrix")?;
            let selects_columns = result.rows == source.rows && result.columns == selector.len();
            let selects_rows = result.rows == selector.len() && result.columns == source.columns;
            match (selects_rows, selects_columns) {
                (false, true) => ((0..source.rows).collect(), selector),
                (true, false) => (selector, (0..source.columns).collect()),
                (true, true) if source.rows == 1 || source.columns == 1 => {
                    if result.rows == source.rows {
                        ((0..source.rows).collect(), selector)
                    } else {
                        (selector, (0..source.columns).collect())
                    }
                }
                (true, true) => {
                    return Err(format!(
                        "matrix access selector is ambiguous for {}x{} -> {}x{}",
                        source.rows, source.columns, result.rows, result.columns
                    ));
                }
                (false, false) => {
                    return Err(format!(
                        "matrix access selector cannot produce {}x{} from {}x{}",
                        result.rows, result.columns, source.rows, source.columns
                    ));
                }
            }
        };
        if result.rows != rows.len() || result.columns != columns.len() {
            return Err(format!(
                "matrix access selected {}x{} elements but output is {}x{}",
                rows.len(),
                columns.len(),
                result.rows,
                result.columns
            ));
        }
        for (result_column, source_column) in columns.into_iter().enumerate() {
            for (result_row, source_row) in rows.iter().copied().enumerate() {
                self.emit(
                    output,
                    result.index(result_row, result_column),
                    ScalarComputation::Copy(
                        self.operand(inputs[0], source.index(source_row, source_column))?,
                    ),
                );
            }
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

    fn lower_solve(&mut self, output: CellSlotId, inputs: &[ArtifactSource]) -> Result<(), String> {
        if inputs.len() != 2 {
            return Err(
                "matrix solve requires a coefficient matrix and right-hand side".to_owned(),
            );
        }
        let coefficients = self.source_shape(inputs[0])?;
        let rhs = self.source_shape(inputs[1])?;
        let result = self.shape(output)?;
        if coefficients.rows != coefficients.columns
            || rhs.rows != coefficients.rows
            || result != rhs
        {
            return Err(format!(
                "invalid matrix solve {}x{} \\ {}x{} -> {}x{}",
                coefficients.rows,
                coefficients.columns,
                rhs.rows,
                rhs.columns,
                result.rows,
                result.columns
            ));
        }

        match coefficients.rows {
            1 => {
                let denominator = self.operand(inputs[0], 0)?;
                for component in 0..rhs.elements() {
                    let numerator = self.operand(inputs[1], component)?;
                    self.emit(
                        output,
                        component,
                        ScalarComputation::Elementwise {
                            operation: ElementwiseOperation::Binary(BinaryOperation::Divide),
                            inputs: vec![numerator, denominator],
                        },
                    );
                }
            }
            2 => {
                // A fixed 2x2 solve is scalarized once at compile time. The
                // resulting arithmetic is shared by CPU SIMD and WebGPU, so
                // ordinary Mech `A \\ b` source does not need to spell out an
                // inverse or depend on an accelerator-specific EKF primitive.
                let a00 = self.operand(inputs[0], coefficients.index(0, 0))?;
                let a01 = self.operand(inputs[0], coefficients.index(0, 1))?;
                let a10 = self.operand(inputs[0], coefficients.index(1, 0))?;
                let a11 = self.operand(inputs[0], coefficients.index(1, 1))?;
                let diagonal = self.emit_temporary_binary(BinaryOperation::Multiply, a00, a11);
                let off_diagonal = self.emit_temporary_binary(BinaryOperation::Multiply, a01, a10);
                let determinant =
                    self.emit_temporary_binary(BinaryOperation::Subtract, diagonal, off_diagonal);

                for column in 0..rhs.columns {
                    let b0 = self.operand(inputs[1], rhs.index(0, column))?;
                    let b1 = self.operand(inputs[1], rhs.index(1, column))?;
                    let numerator0_diagonal =
                        self.emit_temporary_binary(BinaryOperation::Multiply, a11, b0);
                    let numerator0_off_diagonal =
                        self.emit_temporary_binary(BinaryOperation::Multiply, a01, b1);
                    let numerator0 = self.emit_temporary_binary(
                        BinaryOperation::Subtract,
                        numerator0_diagonal,
                        numerator0_off_diagonal,
                    );
                    let numerator1_diagonal =
                        self.emit_temporary_binary(BinaryOperation::Multiply, a00, b1);
                    let numerator1_off_diagonal =
                        self.emit_temporary_binary(BinaryOperation::Multiply, a10, b0);
                    let numerator1 = self.emit_temporary_binary(
                        BinaryOperation::Subtract,
                        numerator1_diagonal,
                        numerator1_off_diagonal,
                    );
                    self.emit(
                        output,
                        result.index(0, column),
                        ScalarComputation::Elementwise {
                            operation: ElementwiseOperation::Binary(BinaryOperation::Divide),
                            inputs: vec![numerator0, determinant],
                        },
                    );
                    self.emit(
                        output,
                        result.index(1, column),
                        ScalarComputation::Elementwise {
                            operation: ElementwiseOperation::Binary(BinaryOperation::Divide),
                            inputs: vec![numerator1, determinant],
                        },
                    );
                }
            }
            rows => {
                return Err(format!(
                    "fixed-shape matrix solve currently supports 1x1 and 2x2 coefficient matrices, found {rows}x{rows}"
                ));
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

    fn emit_temporary_binary(
        &mut self,
        operation: BinaryOperation,
        left: ScalarOperand,
        right: ScalarOperand,
    ) -> ScalarOperand {
        let output = self.register_count;
        self.register_count += 1;
        self.instructions.push(ScalarInstruction {
            output,
            computation: ScalarComputation::Elementwise {
                operation: ElementwiseOperation::Binary(operation),
                inputs: vec![left, right],
            },
        });
        ScalarOperand::Register(output)
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

    fn constant_indices(
        &self,
        source: ArtifactSource,
        upper: usize,
        role: &str,
    ) -> Result<Vec<usize>, String> {
        let Some(one_based) = self.static_selector_indices(source)? else {
            return Err(format!(
                "matrix {role} selector must be compile-time constant"
            ));
        };
        one_based
            .into_iter()
            .map(|index| {
                let zero_based = index
                    .checked_sub(1)
                    .ok_or_else(|| format!("matrix {role} index must be at least 1"))?;
                let index = usize::try_from(zero_based)
                    .map_err(|_| format!("matrix {role} index does not fit usize"))?;
                if index >= upper {
                    return Err(format!(
                        "matrix {role} index {} is outside 1..={upper}",
                        index + 1
                    ));
                }
                Ok(index)
            })
            .collect()
    }

    fn static_selector_indices(&self, source: ArtifactSource) -> Result<Option<Vec<u64>>, String> {
        self.static_numeric_source(source)
    }

    fn static_numeric_source(&self, source: ArtifactSource) -> Result<Option<Vec<u64>>, String> {
        match source {
            ArtifactSource::Constant(constant) => {
                let value = self
                    .artifact
                    .constants()
                    .get(constant)
                    .ok_or_else(|| format!("constant {} does not exist", constant.get()))?;
                static_numeric_indices(value.data()).map(Some)
            }
            ArtifactSource::Slot(slot) => {
                let Some(declaration) = self.artifact.slots().get(slot.get() as usize) else {
                    return Err(format!("selector slot {} does not exist", slot.get()));
                };
                let ProducerReference::NodeOutput { node, .. } = declaration.producer else {
                    return Ok(None);
                };
                let Some(node) = self.artifact.nodes().get(node.get() as usize) else {
                    return Err(format!(
                        "selector producer node {} does not exist",
                        node.get()
                    ));
                };
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
                match display_operation(&node.operation).as_str() {
                    "access/index" => {
                        let [input] = inputs.as_slice() else {
                            return Err(
                                "static index conversion must have exactly one input".to_owned()
                            );
                        };
                        self.static_numeric_source(*input)
                    }
                    "range/inclusive" => {
                        let [from, to] = inputs.as_slice() else {
                            return Err("static inclusive range must have two inputs".to_owned());
                        };
                        let Some(from) = self.static_numeric_source(*from)? else {
                            return Ok(None);
                        };
                        let Some(to) = self.static_numeric_source(*to)? else {
                            return Ok(None);
                        };
                        let ([from], [to]) = (from.as_slice(), to.as_slice()) else {
                            return Err("static inclusive range bounds must be scalar".to_owned());
                        };
                        if from > to {
                            return Ok(Some(Vec::new()));
                        }
                        let count = to
                            .checked_sub(*from)
                            .and_then(|difference| difference.checked_add(1))
                            .ok_or_else(|| "static inclusive range is too large".to_owned())?;
                        let count = usize::try_from(count)
                            .map_err(|_| "static inclusive range is too large".to_owned())?;
                        Ok(Some((*from..=*to).take(count).collect()))
                    }
                    _ => Ok(None),
                }
            }
        }
    }

    fn static_selector_slot(&self, slot: CellSlotId) -> bool {
        if !self
            .static_selector_indices(ArtifactSource::Slot(slot))
            .is_ok_and(|indices| indices.is_some())
        {
            return false;
        }
        let Some(declaration) = self.artifact.slots().get(slot.get() as usize) else {
            return false;
        };
        let ProducerReference::NodeOutput { node, .. } = declaration.producer else {
            return false;
        };
        let Some(producer) = self.artifact.nodes().get(node.get() as usize) else {
            return false;
        };
        if display_operation(&producer.operation) == "access/index" {
            return true;
        }
        let consumers = self
            .artifact
            .bindings()
            .iter()
            .filter_map(|binding| match binding {
                BindingDeclaration::Input {
                    node,
                    source: ArtifactSource::Slot(source),
                    ..
                } if *source == slot => Some(*node),
                _ => None,
            })
            .collect::<Vec<_>>();
        !consumers.is_empty()
            && consumers.iter().all(|consumer| {
                self.artifact
                    .nodes()
                    .get(consumer.get() as usize)
                    .is_some_and(|node| display_operation(&node.operation) == "access/index")
            })
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
        "math/sqrt" => Some(ElementwiseOperation::Unary(UnaryOperation::Sqrt)),
        "math/ceil" => Some(ElementwiseOperation::Unary(UnaryOperation::Ceil)),
        "math/atan2" => Some(ElementwiseOperation::Atan2),
        _ => None,
    }
}

fn fixed_shape_program_error(detail: impl Into<String>) -> GpuAdmissionError {
    GpuAdmissionError {
        diagnostics: vec![GpuDiagnostic {
            code: GpuDiagnosticCode::OperationUnsupported,
            node: None,
            operation: None,
            detail: detail.into(),
        }],
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
        body => Err(format!(
            "schema {body:?} is not fixed-shape f32 numeric data"
        )),
    }
}

fn static_numeric_indices(data: &ValueData) -> Result<Vec<u64>, String> {
    macro_rules! unsigned_scalar {
        ($value:expr) => {
            u64::try_from(*$value)
                .map(|value| vec![value])
                .map_err(|_| "static selector exceeds the portable index range".to_owned())
        };
    }
    macro_rules! signed_scalar {
        ($value:expr) => {
            u64::try_from(*$value)
                .map(|value| vec![value])
                .map_err(|_| "static selector must be a nonnegative integer".to_owned())
        };
    }
    match data {
        ValueData::Index(value) | ValueData::U64(value) => Ok(vec![*value]),
        ValueData::U8(value) => Ok(vec![u64::from(*value)]),
        ValueData::U16(value) => Ok(vec![u64::from(*value)]),
        ValueData::U32(value) => Ok(vec![u64::from(*value)]),
        ValueData::U128(value) => unsigned_scalar!(value),
        ValueData::I8(value) => signed_scalar!(value),
        ValueData::I16(value) => signed_scalar!(value),
        ValueData::I32(value) => signed_scalar!(value),
        ValueData::I64(value) => signed_scalar!(value),
        ValueData::I128(value) => signed_scalar!(value),
        ValueData::F32(value) => {
            exact_float_index(f64::from(value.to_f32())).map(|value| vec![value])
        }
        ValueData::F64(value) => exact_float_index(value.to_f64()).map(|value| vec![value]),
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::Index(values) | SequenceView::U64(values) => Ok(values.to_vec()),
            SequenceView::U8(values) => Ok(values.iter().copied().map(u64::from).collect()),
            SequenceView::U16(values) => Ok(values.iter().copied().map(u64::from).collect()),
            SequenceView::U32(values) => Ok(values.iter().copied().map(u64::from).collect()),
            SequenceView::U128(values) => values
                .iter()
                .map(|value| {
                    u64::try_from(*value)
                        .map_err(|_| "static selector exceeds the portable index range".to_owned())
                })
                .collect(),
            SequenceView::I8(values) => signed_indices(values),
            SequenceView::I16(values) => signed_indices(values),
            SequenceView::I32(values) => signed_indices(values),
            SequenceView::I64(values) => signed_indices(values),
            SequenceView::I128(values) => signed_indices(values),
            SequenceView::F32(values) => values
                .iter()
                .map(|value| exact_float_index(f64::from(value.to_f32())))
                .collect(),
            SequenceView::F64(values) => values
                .iter()
                .map(|value| exact_float_index(value.to_f64()))
                .collect(),
            _ => Err("static matrix selector must contain real integers".to_owned()),
        },
        _ => Err("static selector must be a real integer or index".to_owned()),
    }
}

fn signed_indices<T>(values: &[T]) -> Result<Vec<u64>, String>
where
    T: Copy,
    u64: TryFrom<T>,
{
    values
        .iter()
        .map(|value| {
            u64::try_from(*value)
                .map_err(|_| "static selector must contain nonnegative integers".to_owned())
        })
        .collect()
}

fn exact_float_index(value: f64) -> Result<u64, String> {
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value >= u64::MAX as f64 {
        return Err("static selector must be a finite nonnegative integer".to_owned());
    }
    Ok(value as u64)
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
    generate_wgsl_with_turns(
        instances,
        register_offsets,
        instructions,
        inputs,
        states,
        constraints,
        None,
        true,
        false,
    )
}

fn generate_wgsl_unchecked(
    instances: u32,
    register_offsets: &BTreeMap<CellSlotId, usize>,
    instructions: &[ScalarInstruction],
    inputs: &[BatchedInput],
    states: &[BatchedState],
) -> String {
    generate_wgsl_with_turns(
        instances,
        register_offsets,
        instructions,
        inputs,
        states,
        &[],
        None,
        true,
        false,
    )
}

#[cfg(feature = "native")]
fn generate_wgsl_fused(
    instances: u32,
    register_offsets: &BTreeMap<CellSlotId, usize>,
    instructions: &[ScalarInstruction],
    inputs: &[BatchedInput],
    states: &[BatchedState],
    turns: u32,
) -> String {
    assert!(turns > 0, "fused GPU kernels require at least one turn");
    generate_wgsl_with_turns(
        instances,
        register_offsets,
        instructions,
        inputs,
        states,
        &[],
        Some(turns),
        true,
        false,
    )
}

#[cfg(feature = "native")]
fn generate_wgsl_unchecked_in_place(
    instances: u32,
    register_offsets: &BTreeMap<CellSlotId, usize>,
    instructions: &[ScalarInstruction],
    inputs: &[BatchedInput],
    states: &[BatchedState],
) -> String {
    generate_wgsl_with_turns(
        instances,
        register_offsets,
        instructions,
        inputs,
        states,
        &[],
        None,
        true,
        true,
    )
}

fn generate_wgsl_with_turns(
    instances: u32,
    register_offsets: &BTreeMap<CellSlotId, usize>,
    instructions: &[ScalarInstruction],
    inputs: &[BatchedInput],
    states: &[BatchedState],
    constraints: &[BatchedConstraint],
    fused_turns: Option<u32>,
    optimize_unchecked: bool,
    in_place_state: bool,
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
        if in_place_state {
            shader.push_str(&format!(
                "@group(0) @binding({}) var<storage, read_write> state_{}: array<f32>;\n",
                state.read_binding,
                state.slot.get()
            ));
        } else {
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
            let state_name = if in_place_state {
                format!("state_{}", state.slot.get())
            } else {
                format!("state_read_{}", state.slot.get())
            };
            shader.push_str(&format!(
                "  {} r{} = {}[index * {}u + {}u];\n",
                if fused_turns.is_some() { "var" } else { "let" },
                offset + component,
                state_name,
                state.shape.elements(),
                component
            ));
        }
    }
    if let Some(turns) = fused_turns {
        shader.push_str(&format!(
            "  for (var mech_turn = 0u; mech_turn < {turns}u; mech_turn = mech_turn + 1u) {{\n"
        ));
    }
    let mut aliases = BTreeMap::new();
    let mut expression_aliases = BTreeMap::<String, ScalarOperand>::new();
    for instruction in instructions {
        if optimize_unchecked {
            match fast_wgsl_instruction(&instruction.computation, &aliases) {
                FastWgslInstruction::Alias(operand) => {
                    aliases.insert(instruction.output, operand);
                    continue;
                }
                FastWgslInstruction::Expression(expression) => {
                    // All scalar computations are pure. Reuse an identical
                    // expression instead of asking the backend to rediscover
                    // common subexpressions across the lowered instruction
                    // stream (matrix products commonly repeat these terms).
                    if let Some(operand) = expression_aliases.get(&expression).copied() {
                        aliases.insert(instruction.output, operand);
                        continue;
                    }
                    expression_aliases.insert(
                        expression.clone(),
                        ScalarOperand::Register(instruction.output),
                    );
                    shader.push_str(&format!(
                        "  {}let r{} = {};\n",
                        if fused_turns.is_some() { "  " } else { "" },
                        instruction.output,
                        expression
                    ));
                }
            }
        } else {
            shader.push_str(&format!(
                "  {}let r{} = {};\n",
                if fused_turns.is_some() { "  " } else { "" },
                instruction.output,
                scalar_computation_wgsl(&instruction.computation)
            ));
        }
    }
    if fused_turns.is_some() {
        // Evaluate every component before assigning any state register. This
        // preserves whole-value assignment semantics when one component's
        // update reads another component from the previous turn.
        for state in states {
            for (component, source) in state.update.iter().enumerate() {
                shader.push_str(&format!(
                    "    let next_state_{}_{} = {};\n",
                    state.slot.get(),
                    component,
                    fast_operand_wgsl(*source, &aliases)
                ));
            }
        }
        for state in states {
            let offset = register_offsets[&state.slot];
            for component in 0..state.shape.elements() {
                shader.push_str(&format!(
                    "    r{} = next_state_{}_{};\n",
                    offset + component,
                    state.slot.get(),
                    component
                ));
            }
        }
        shader.push_str("  }\n");
        for state in states {
            for component in 0..state.shape.elements() {
                let state_name = if in_place_state {
                    format!("state_{}", state.slot.get())
                } else {
                    format!("state_write_{}", state.slot.get())
                };
                shader.push_str(&format!(
                    "  {}[index * {}u + {}u] = r{};\n",
                    state_name,
                    state.shape.elements(),
                    component,
                    register_offsets[&state.slot] + component
                ));
            }
        }
    } else {
        if !constraints.is_empty() {
            shader.push_str("  var integrity_code = 0u;\n");
            for (index, constraint) in constraints.iter().enumerate() {
                let code = index + 1;
                shader.push_str(&format!(
                    "  if (integrity_code == 0u && !{}) {{ integrity_code = {code}u; }}\n",
                    if optimize_unchecked {
                        scalar_predicate_wgsl_with_aliases(&constraint.predicate, &aliases)
                    } else {
                        scalar_predicate_wgsl(&constraint.predicate)
                    }
                ));
            }
            shader.push_str(
                "  if (integrity_code != 0u) { record_integrity_fault(integrity_code, index); }\n",
            );
        }
        for state in states {
            for (component, source) in state.update.iter().enumerate() {
                let state_name = if in_place_state {
                    format!("state_{}", state.slot.get())
                } else {
                    format!("state_write_{}", state.slot.get())
                };
                shader.push_str(&format!(
                    "  {}[index * {}u + {}u] = {};\n",
                    state_name,
                    state.shape.elements(),
                    component,
                    if optimize_unchecked {
                        fast_operand_wgsl(*source, &aliases)
                    } else {
                        scalar_operand_wgsl(*source)
                    }
                ));
            }
        }
    }
    shader.push_str("}\n");
    shader
}

#[cfg(feature = "native")]
mod native {
    use std::{
        collections::{BTreeMap, BTreeSet},
        env,
        sync::{Arc, mpsc},
        time::{Duration, Instant},
    };

    use mech_core::{CellSlotId, IntegrityConstraintId};
    use wgpu::util::DeviceExt;

    use super::{
        BatchedExecutionError, BatchedFaultRecorder, BatchedIntegrityFault, FixedShapeKernel,
        generate_wgsl_fused, generate_wgsl_unchecked_in_place,
    };
    use crate::{
        GpuBindingAccess, GpuExecutionBindingRole, GpuPlanConstraint, GpuPlanInitialValues,
    };

    const GPU_FAULT_WORDS: usize = 2;

    #[derive(Debug)]
    pub struct BatchedResidentGpuSession {
        adapter: String,
        device: wgpu::Device,
        queue: wgpu::Queue,
        pipeline: wgpu::ComputePipeline,
        bind_groups: [wgpu::BindGroup; 2],
        input_buffers: BTreeMap<CellSlotId, Arc<wgpu::Buffer>>,
        output_buffers: [BTreeMap<CellSlotId, Arc<wgpu::Buffer>>; 2],
        output_elements: BTreeMap<CellSlotId, usize>,
        output_elements_per_instance: BTreeMap<CellSlotId, usize>,
        constraints: Box<[GpuPlanConstraint]>,
        integrity_fault: Option<Arc<wgpu::Buffer>>,
        integrity_readback: Option<wgpu::Buffer>,
        workgroups: u32,
        fused_turns: Option<u32>,
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

    impl FixedShapeKernel {
        pub fn prepare_resident(
            &self,
            inputs: &BTreeMap<String, Vec<f32>>,
        ) -> Result<BatchedResidentGpuSession, BatchedExecutionError> {
            pollster::block_on(self.prepare_resident_async(inputs, None, false))
        }

        /// Prepares an explicitly unchecked resident GPU session. Integrity
        /// predicates and rollback publication are omitted from the generated
        /// device kernel; callers must opt into this weaker contract by
        /// constructing the kernel with [`FixedShapeKernel::without_integrity_constraints`].
        pub fn prepare_resident_unchecked(
            &self,
            inputs: &BTreeMap<String, Vec<f32>>,
        ) -> Result<BatchedResidentGpuSession, BatchedExecutionError> {
            if !self.constraints.is_empty() {
                return Err(BatchedExecutionError::Native(
                    "unchecked GPU preparation requires a kernel without integrity constraints"
                        .to_owned(),
                ));
            }
            pollster::block_on(self.prepare_resident_async(inputs, None, false))
        }

        /// Prepares an unchecked resident session with one read-write state
        /// binding. Since rollback is explicitly disabled, the shader can
        /// compute a complete candidate before writing it back in place.
        pub fn prepare_resident_unchecked_in_place(
            &self,
            inputs: &BTreeMap<String, Vec<f32>>,
        ) -> Result<BatchedResidentGpuSession, BatchedExecutionError> {
            if !self.constraints.is_empty() {
                return Err(BatchedExecutionError::Native(
                    "unchecked in-place GPU preparation requires a kernel without integrity constraints"
                        .to_owned(),
                ));
            }
            pollster::block_on(self.prepare_resident_async(inputs, None, true))
        }

        /// Prepares a single-dispatch unchecked kernel that advances a fixed
        /// number of resident turns inside each device invocation. Inputs and
        /// state are loaded once per lane, then kept in device-local values for
        /// the complete recurrence before one final state write.
        pub fn prepare_resident_unchecked_fused(
            &self,
            inputs: &BTreeMap<String, Vec<f32>>,
            turns: u32,
        ) -> Result<BatchedResidentGpuSession, BatchedExecutionError> {
            if !self.constraints.is_empty() {
                return Err(BatchedExecutionError::Native(
                    "unchecked fused GPU preparation requires a kernel without integrity constraints"
                        .to_owned(),
                ));
            }
            if turns == 0 {
                return Err(BatchedExecutionError::ZeroTurns);
            }
            pollster::block_on(self.prepare_resident_async(inputs, Some(turns), false))
        }

        async fn prepare_resident_async(
            &self,
            inputs: &BTreeMap<String, Vec<f32>>,
            fused_turns: Option<u32>,
            in_place_unchecked: bool,
        ) -> Result<BatchedResidentGpuSession, BatchedExecutionError> {
            let mut execution_plan = crate::GpuExecutionPlan::build(
                crate::GpuKernelPlanSource::FixedShape(self),
                inputs,
            )
            .map_err(|failure| BatchedExecutionError::Native(failure.to_string()))?;
            let instance = wgpu_instance_from_environment();
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
            let required_storage_buffers = execution_plan
                .bindings
                .iter()
                .filter(|binding| {
                    !(in_place_unchecked && binding.role == GpuExecutionBindingRole::StateWrite)
                })
                .count() as u32;
            let limits = adapter.limits();
            if required_storage_buffers > limits.max_storage_buffers_per_shader_stage {
                return Err(BatchedExecutionError::Native(format!(
                    "kernel requires {required_storage_buffers} storage buffers; adapter supports {}",
                    limits.max_storage_buffers_per_shader_stage
                )));
            }
            let workgroup_count = execution_plan
                .dispatch_elements
                .div_ceil(execution_plan.workgroup_size);
            if workgroup_count > limits.max_compute_workgroups_per_dimension {
                return Err(BatchedExecutionError::Native(format!(
                    "kernel requires {} workgroups; adapter supports {}",
                    workgroup_count, limits.max_compute_workgroups_per_dimension
                )));
            }
            let required_limits = wgpu::Limits {
                max_storage_buffers_per_shader_stage: required_storage_buffers,
                max_compute_workgroups_per_dimension: workgroup_count,
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

            let mut input_buffers = BTreeMap::new();
            for binding in execution_plan
                .bindings
                .iter()
                .filter(|binding| binding.role == GpuExecutionBindingRole::Input)
            {
                let Some(GpuPlanInitialValues::F32(values)) = &binding.initial_values else {
                    return Err(BatchedExecutionError::Native(format!(
                        "GPU input `{}` has no f32 initializer",
                        binding.name
                    )));
                };
                input_buffers.insert(
                    CellSlotId::new(binding.slot),
                    Arc::new(
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(&binding.name),
                            contents: bytemuck::cast_slice(values),
                            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                        }),
                    ),
                );
            }

            let mut state_buffers = BTreeMap::new();
            for state in &execution_plan.states {
                let slot = CellSlotId::new(state.slot);
                let initial = Arc::new(device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some("Mech fixed-shape initial state"),
                        contents: bytemuck::cast_slice(&state.initial_values),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    },
                ));
                let alternate = if in_place_unchecked {
                    initial.clone()
                } else {
                    Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("Mech fixed-shape alternate state"),
                        size: state.elements * std::mem::size_of::<f32>() as u64,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                        mapped_at_creation: false,
                    }))
                };
                state_buffers.insert(slot, [initial, alternate]);
            }

            let integrity_binding = execution_plan
                .bindings
                .iter()
                .find(|binding| binding.role == GpuExecutionBindingRole::IntegrityFault);
            let integrity_fault = integrity_binding.map(|binding| {
                Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Mech integrity-constraint fault"),
                    size: binding.elements * std::mem::size_of::<u32>() as u64,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }))
            });
            let integrity_readback = integrity_binding.map(|binding| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Mech integrity-constraint readback"),
                    size: binding.elements * std::mem::size_of::<u32>() as u64,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                })
            });

            let layout_entries = execution_plan
                .bindings
                .iter()
                .filter(|binding| {
                    !(in_place_unchecked && binding.role == GpuExecutionBindingRole::StateWrite)
                })
                .map(|binding| wgpu::BindGroupLayoutEntry {
                    binding: binding.binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: binding.access == GpuBindingAccess::Read
                                && !(in_place_unchecked
                                    && binding.role == GpuExecutionBindingRole::StateRead),
                        },
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
                let entries = execution_plan
                    .bindings
                    .iter()
                    .filter(|binding| {
                        !(in_place_unchecked && binding.role == GpuExecutionBindingRole::StateWrite)
                    })
                    .map(|binding| {
                        let resource = match binding.role {
                            GpuExecutionBindingRole::Input => {
                                input_buffers[&CellSlotId::new(binding.slot)].as_entire_binding()
                            }
                            GpuExecutionBindingRole::StateRead => state_buffers
                                [&CellSlotId::new(binding.slot)][group]
                                .as_entire_binding(),
                            GpuExecutionBindingRole::StateWrite => state_buffers
                                [&CellSlotId::new(binding.slot)][1 - group]
                                .as_entire_binding(),
                            GpuExecutionBindingRole::IntegrityFault => integrity_fault
                                .as_ref()
                                .expect("integrity plan has a fault buffer")
                                .as_entire_binding(),
                            GpuExecutionBindingRole::Output => {
                                unreachable!("fixed-shape outputs alias resident state buffers")
                            }
                        };
                        wgpu::BindGroupEntry {
                            binding: binding.binding,
                            resource,
                        }
                    })
                    .collect::<Vec<_>>();
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Mech fixed-shape batch bind group"),
                    layout: &bind_group_layout,
                    entries: &entries,
                })
            });
            if in_place_unchecked {
                execution_plan.wgsl = generate_wgsl_unchecked_in_place(
                    self.instances,
                    &self.register_offsets,
                    &self.instructions,
                    &self.inputs,
                    &self.states,
                );
            } else if let Some(turns) = fused_turns {
                execution_plan.wgsl = generate_wgsl_fused(
                    self.instances,
                    &self.register_offsets,
                    &self.instructions,
                    &self.inputs,
                    &self.states,
                    turns,
                );
            }
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Scalarized Mech fixed-shape batch"),
                source: wgpu::ShaderSource::Wgsl(execution_plan.wgsl.clone().into()),
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
                execution_plan
                    .physical_outputs
                    .iter()
                    .map(|output| {
                        let slot = CellSlotId::new(output.slot);
                        (slot, state_buffers[&slot][1 - group].clone())
                    })
                    .collect()
            });
            let output_elements = execution_plan
                .outputs
                .iter()
                .map(|output| (CellSlotId::new(output.slot), output.elements as usize))
                .collect();
            let output_elements_per_instance = execution_plan
                .outputs
                .iter()
                .map(|output| {
                    (
                        CellSlotId::new(output.slot),
                        output.elements_per_instance as usize,
                    )
                })
                .collect();
            Ok(BatchedResidentGpuSession {
                adapter: adapter_name,
                device,
                queue,
                pipeline,
                bind_groups,
                input_buffers,
                output_buffers,
                output_elements,
                output_elements_per_instance,
                constraints: execution_plan.constraints.into_boxed_slice(),
                integrity_fault,
                integrity_readback,
                workgroups: workgroup_count,
                fused_turns,
                next_group: 0,
                last_output_group: None,
                faults: BatchedFaultRecorder::default(),
            })
        }
    }

    fn wgpu_instance_from_environment() -> wgpu::Instance {
        let backends = match env::var("MECH_WGPU_BACKEND")
            .ok()
            .map(|value| value.to_ascii_lowercase())
            .as_deref()
        {
            None | Some("all") => wgpu::Backends::all(),
            Some("metal") => wgpu::Backends::METAL,
            Some("vulkan") => wgpu::Backends::VULKAN,
            Some("dx12") => wgpu::Backends::DX12,
            Some("gl") | Some("gles") => wgpu::Backends::GL,
            Some("webgpu") => wgpu::Backends::BROWSER_WEBGPU,
            Some(value) => panic!(
                "unsupported MECH_WGPU_BACKEND={value:?}; use all, metal, vulkan, dx12, gl, or webgpu"
            ),
        };
        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        })
    }

    impl BatchedResidentGpuSession {
        pub fn adapter(&self) -> &str {
            &self.adapter
        }

        pub fn update_inputs(
            &mut self,
            program: &FixedShapeKernel,
            updates: &BTreeMap<String, Vec<f32>>,
        ) -> Result<(), BatchedExecutionError> {
            for (name, values) in updates {
                let input = program
                    .inputs
                    .iter()
                    .find(|input| input.name == *name)
                    .ok_or_else(|| BatchedExecutionError::MissingInput(name.clone()))?;
                let values = program.expand_input(input, values)?;
                self.queue.write_buffer(
                    &self.input_buffers[&input.slot],
                    0,
                    bytemuck::cast_slice(&values),
                );
            }
            Ok(())
        }

        pub fn dispatch_turns(&mut self, turns: u32) -> Result<Duration, BatchedExecutionError> {
            if turns == 0 {
                return Err(BatchedExecutionError::ZeroTurns);
            }
            if self.fused_turns.is_some() {
                return Err(BatchedExecutionError::Native(
                    "fused unchecked GPU sessions must use dispatch_unchecked_fused".to_owned(),
                ));
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

        /// Dispatches the resident fused unchecked kernel exactly once. The
        /// prepared shader performs its configured number of turns on-device;
        /// no per-turn command encoding, synchronization, or state swap occurs
        /// on the host.
        pub fn dispatch_unchecked_fused(&mut self) -> Result<Duration, BatchedExecutionError> {
            let turns = self.fused_turns.ok_or_else(|| {
                BatchedExecutionError::Native(
                    "session was not prepared with a fused unchecked kernel".to_owned(),
                )
            })?;
            if !self.constraints.is_empty() {
                return Err(BatchedExecutionError::Native(
                    "fused GPU dispatch cannot carry integrity constraints".to_owned(),
                ));
            }
            let started = Instant::now();
            let group = self.next_group;
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Mech fused unchecked fixed-shape batch"),
                });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Mech fused unchecked fixed-shape compute pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_groups[group], &[]);
                pass.dispatch_workgroups(self.workgroups, 1, 1);
            }
            self.queue.submit(Some(encoder.finish()));
            self.device.poll(wgpu::Maintain::Wait);
            self.last_output_group = Some(group);
            self.next_group = 1 - group;
            let elapsed = started.elapsed();
            debug_assert!(turns > 0);
            Ok(elapsed)
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
                        constraint: IntegrityConstraintId::new(constraint.id),
                        constraint_name: constraint.name.clone().into_boxed_str(),
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
                drop(sender.send(result));
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

        pub const fn attempted_turns(&self) -> u64 {
            self.faults.attempted_turns()
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
            self.read_state_group(group, None, None)
        }

        /// Reads the currently published state, including the initial state or
        /// the estimate retained after a rejected candidate.
        pub fn read_published_state(
            &self,
        ) -> Result<(Duration, BTreeMap<CellSlotId, Vec<f32>>), BatchedExecutionError> {
            self.read_state_group(1 - self.next_group, None, None)
        }

        /// Reads one lane for only the selected published state buffers.
        /// This is the physical sampling boundary used by resident hosts: the
        /// full outer batch never crosses from device memory to the CPU.
        pub fn read_published_sample(
            &self,
            slots: &BTreeSet<CellSlotId>,
            instance: u32,
        ) -> Result<BTreeMap<CellSlotId, Vec<f32>>, BatchedExecutionError> {
            self.read_state_group(1 - self.next_group, Some(slots), Some(instance))
                .map(|(_, state)| state)
        }

        fn read_state_group(
            &self,
            group: usize,
            selected: Option<&BTreeSet<CellSlotId>>,
            sample_instance: Option<u32>,
        ) -> Result<(Duration, BTreeMap<CellSlotId, Vec<f32>>), BatchedExecutionError> {
            let started = Instant::now();
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Mech fixed-shape batch readback encoder"),
                });
            let mut readbacks = Vec::new();
            for (slot, buffer) in &self.output_buffers[group] {
                if selected.is_some_and(|selected| !selected.contains(slot)) {
                    continue;
                }
                let elements = if sample_instance.is_some() {
                    self.output_elements_per_instance[slot]
                } else {
                    self.output_elements[slot]
                };
                let size = (elements * std::mem::size_of::<f32>()) as u64;
                let source_offset =
                    sample_instance.map_or(0, |instance| u64::from(instance) * size);
                if source_offset.saturating_add(size) > buffer.size() {
                    return Err(BatchedExecutionError::Native(format!(
                        "sample instance {} exceeds the resident batch for slot {slot:?}",
                        sample_instance.expect("sample instance is present"),
                    )));
                }
                let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Mech fixed-shape batch readback"),
                    size,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                encoder.copy_buffer_to_buffer(buffer, source_offset, &readback, 0, size);
                readbacks.push((*slot, readback));
            }
            self.queue.submit(Some(encoder.finish()));

            let mut state = BTreeMap::new();
            for (slot, readback) in readbacks {
                let slice = readback.slice(..);
                let (sender, receiver) = mpsc::channel();
                slice.map_async(wgpu::MapMode::Read, move |result| {
                    drop(sender.send(result));
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
