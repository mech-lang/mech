use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::ElementwiseOperation;
use mech_core::{CellSlotId, IntegrityConstraintId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedShape {
    pub rows: usize,
    pub columns: usize,
}

impl FixedShape {
    pub const fn scalar() -> Self {
        Self {
            rows: 1,
            columns: 1,
        }
    }

    pub const fn elements(self) -> usize {
        self.rows * self.columns
    }

    pub const fn index(self, row: usize, column: usize) -> usize {
        row + column * self.rows
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ScalarOperand {
    Register(usize),
    Constant(f32),
}

impl ScalarOperand {
    pub fn evaluate(self, registers: &[f32]) -> f32 {
        match self {
            Self::Register(register) => registers[register],
            Self::Constant(value) => value,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComparisonOperation {
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
}

impl ComparisonOperation {
    pub fn apply(self, left: f32, right: f32) -> bool {
        match self {
            Self::Equal => left == right,
            Self::NotEqual => left != right,
            Self::Less => left < right,
            Self::Greater => left > right,
            Self::LessEqual => left <= right,
            Self::GreaterEqual => left >= right,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicOperation {
    And,
    Or,
    Xor,
    Not,
}

impl LogicOperation {
    pub fn apply(self, left: bool, right: Option<bool>) -> bool {
        match self {
            Self::And => left && right.expect("binary logic operation has a right operand"),
            Self::Or => left || right.expect("binary logic operation has a right operand"),
            Self::Xor => left ^ right.expect("binary logic operation has a right operand"),
            Self::Not => !left,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ScalarComputation {
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

impl ScalarComputation {
    pub fn evaluate(&self, registers: &[f32]) -> f32 {
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
                operation.apply(&values[..inputs.len()])
            }
            Self::SumProducts(terms) => terms.iter().fold(0.0, |sum, (left, right)| {
                left.evaluate(registers)
                    .mul_add(right.evaluate(registers), sum)
            }),
        }
    }

    pub fn collect_registers(&self, registers: &mut BTreeSet<usize>) {
        match self {
            Self::Copy(input)
            | Self::Negate(input)
            | Self::Absolute(input)
            | Self::IsFinite(input) => collect_operand_register(*input, registers),
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

#[derive(Clone, Debug)]
pub enum ScalarPredicate {
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
    pub fn evaluate(&self, registers: &[f32]) -> bool {
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

    pub fn collect_registers(&self, registers: &mut BTreeSet<usize>) {
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

pub fn collect_operand_register(operand: ScalarOperand, registers: &mut BTreeSet<usize>) {
    if let ScalarOperand::Register(register) = operand {
        registers.insert(register);
    }
}

#[derive(Clone, Debug)]
pub struct ScalarInstruction {
    pub output: usize,
    pub computation: ScalarComputation,
}

#[derive(Clone, Debug, Default)]
pub struct FixedShapeIr {
    pub register_count: usize,
    pub instructions: Box<[ScalarInstruction]>,
}

/// Backend-neutral resident storage for a scalarized fixed-shape region.
/// Physical backends may assign bindings or convert layouts once when they
/// compile this plan, but they do not consult the source artifact again.
#[derive(Clone, Debug, Default)]
pub struct FixedShapeStoragePlan {
    pub instances: u32,
    pub register_offsets: BTreeMap<CellSlotId, usize>,
    pub inputs: Box<[FixedShapeInputStorage]>,
    pub states: Box<[FixedShapeStateStorage]>,
    pub constraints: Box<[FixedShapeConstraint]>,
}

#[derive(Clone, Debug)]
pub struct FixedShapeInputStorage {
    pub slot: CellSlotId,
    pub name: Box<str>,
    pub shape: FixedShape,
}

#[derive(Clone, Debug)]
pub struct FixedShapeStateStorage {
    pub slot: CellSlotId,
    pub shape: FixedShape,
    pub initializer: Arc<[f32]>,
    pub update: Box<[ScalarOperand]>,
}

#[derive(Clone, Debug)]
pub struct FixedShapeConstraint {
    pub id: IntegrityConstraintId,
    pub name: Box<str>,
    pub predicate: ScalarPredicate,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BinaryOperation;

    #[test]
    fn fixed_shape_ir_evaluates_without_backend_state() {
        let computation = ScalarComputation::Elementwise {
            operation: ElementwiseOperation::Binary(BinaryOperation::Add),
            inputs: vec![ScalarOperand::Register(0), ScalarOperand::Constant(2.0)],
        };
        assert_eq!(computation.evaluate(&[40.0]), 42.0);
    }

    #[test]
    fn integrity_predicates_share_the_same_scalar_semantics() {
        let predicate = ScalarPredicate::All(vec![
            ScalarPredicate::IsFinite(ScalarOperand::Register(0)),
            ScalarPredicate::Compare {
                operation: ComparisonOperation::Greater,
                left: ScalarOperand::Register(0),
                right: ScalarOperand::Constant(0.0),
            },
        ]);
        assert!(predicate.evaluate(&[1.0]));
        assert!(!predicate.evaluate(&[f32::NAN]));
        assert!(!predicate.evaluate(&[-1.0]));
    }
}
