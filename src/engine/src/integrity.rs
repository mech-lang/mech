use crate::MechProgram;
use crate::{Interpreter, InterpreterRef};
use mech_core::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegrityConstraintFailureReason {
    EvaluatedFalse,
    ExpectedBool,
    BorrowConflict,
}

#[derive(Clone, Debug)]
pub struct IntegrityConstraintEvaluation {
    pub interpreter_id: u64,
    pub constraint_id: u64,
    pub name: String,
    pub expression: String,
    pub passed: bool,
    pub reason: Option<IntegrityConstraintFailureReason>,
    pub evaluated_kind: Option<ValueKind>,
    pub actual: Option<String>,
    pub operator: Option<FormulaOperator>,
    pub expected: Option<String>,
    pub tokens: Vec<Token>,
}

#[derive(Clone, Debug)]
pub struct IntegrityConstraintViolation {
    pub interpreter_id: u64,
    pub constraint_id: u64,
    pub name: String,
    pub expression: String,
    pub reason: IntegrityConstraintFailureReason,
    pub evaluated_kind: Option<ValueKind>,
    pub actual: Option<String>,
    pub operator: Option<FormulaOperator>,
    pub expected: Option<String>,
    pub tokens: Vec<Token>,
}

#[derive(Clone, Debug)]
pub struct IntegrityConstraintReport {
    pub checked: usize,
    pub evaluations: Vec<IntegrityConstraintEvaluation>,
    pub violations: Vec<IntegrityConstraintViolation>,
}

#[derive(Clone, Debug)]
pub struct IntegrityConstraintViolationSet {
    pub checked: usize,
    pub evaluations: Vec<IntegrityConstraintEvaluation>,
    pub violations: Vec<IntegrityConstraintViolation>,
}

impl IntegrityConstraintReport {
    pub fn from_evaluations(evaluations: Vec<IntegrityConstraintEvaluation>) -> Self {
        let violations = evaluations
            .iter()
            .filter_map(|evaluation| {
                let reason = evaluation.reason.clone()?;
                Some(IntegrityConstraintViolation {
                    interpreter_id: evaluation.interpreter_id,
                    constraint_id: evaluation.constraint_id,
                    name: evaluation.name.clone(),
                    expression: evaluation.expression.clone(),
                    reason,
                    evaluated_kind: evaluation.evaluated_kind.clone(),
                    actual: evaluation.actual.clone(),
                    operator: evaluation.operator.clone(),
                    expected: evaluation.expected.clone(),
                    tokens: evaluation.tokens.clone(),
                })
            })
            .collect::<Vec<_>>();
        Self {
            checked: evaluations.len(),
            evaluations,
            violations,
        }
    }

    pub fn into_violation_set(self) -> Option<IntegrityConstraintViolationSet> {
        if self.violations.is_empty() {
            return None;
        }
        Some(IntegrityConstraintViolationSet {
            checked: self.checked,
            evaluations: self.evaluations,
            violations: self.violations,
        })
    }
}

impl MechErrorKind for IntegrityConstraintViolationSet {
    fn name(&self) -> &str {
        "IntegrityConstraintViolationSet"
    }

    fn message(&self) -> String {
        format!(
            "{} integrity constraint{} failed out of {} checked.",
            self.violations.len(),
            if self.violations.len() == 1 { "" } else { "s" },
            self.checked,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrityConstraintHierarchyConflict {
    pub interpreter_id: u64,
}

impl MechErrorKind for IntegrityConstraintHierarchyConflict {
    fn name(&self) -> &str {
        "IntegrityConstraintHierarchyConflict"
    }

    fn message(&self) -> String {
        format!(
            "Interpreter {} appears more than once in the integrity-constraint hierarchy.",
            self.interpreter_id,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrityConstraintRegistryBorrowConflict {
    pub interpreter_id: u64,
    pub registry: &'static str,
}

impl MechErrorKind for IntegrityConstraintRegistryBorrowConflict {
    fn name(&self) -> &str {
        "IntegrityConstraintRegistryBorrowConflict"
    }

    fn message(&self) -> String {
        format!(
            "Cannot read the {} registry for interpreter {} while validating integrity constraints.",
            self.registry, self.interpreter_id,
        )
    }
}

struct ResolvedIntegrityValue {
    kind: ValueKind,
    scalar_bool: Option<bool>,
    formatted: String,
}

impl MechProgram {
    pub fn integrity_constraint_report(&self) -> MResult<IntegrityConstraintReport> {
        let mut evaluations = Vec::new();
        let mut visited = Vec::new();
        collect_interpreter_constraints(self.interpreter(), None, &mut visited, &mut evaluations)?;
        evaluations.sort_by_key(|evaluation| (evaluation.interpreter_id, evaluation.constraint_id));
        Ok(IntegrityConstraintReport::from_evaluations(evaluations))
    }

    pub fn validate_integrity_constraints(&self) -> MResult<()> {
        let report = self.integrity_constraint_report()?;
        let Some(failures) = report.into_violation_set() else {
            return Ok(());
        };
        let tokens = failures
            .violations
            .iter()
            .flat_map(|violation| violation.tokens.clone())
            .collect::<Vec<_>>();
        Err(MechError::new(failures, None)
            .with_compiler_loc()
            .with_tokens(tokens))
    }
}

fn collect_interpreter_constraints(
    interpreter: &Interpreter,
    handle: Option<&InterpreterRef>,
    visited: &mut Vec<InterpreterRef>,
    evaluations: &mut Vec<IntegrityConstraintEvaluation>,
) -> MResult<()> {
    if let Some(handle) = handle {
        if visited.iter().any(|seen| seen.same_handle(handle)) {
            return Err(MechError::new(
                IntegrityConstraintHierarchyConflict {
                    interpreter_id: interpreter.id,
                },
                None,
            )
            .with_compiler_loc());
        }
        visited.push(handle.clone());
    }

    let constraints = interpreter
        .state
        .try_borrow()
        .map_err(|_| {
            MechError::new(
                IntegrityConstraintRegistryBorrowConflict {
                    interpreter_id: interpreter.id,
                    registry: "program-state",
                },
                None,
            )
            .with_compiler_loc()
        })?
        .integrity_constraints
        .values()
        .cloned()
        .collect::<Vec<_>>();
    for constraint in constraints {
        evaluations.push(evaluate_constraint(interpreter.id, &constraint));
    }

    let child_handles = interpreter
        .sub_interpreters
        .try_borrow()
        .map_err(|_| {
            MechError::new(
                IntegrityConstraintRegistryBorrowConflict {
                    interpreter_id: interpreter.id,
                    registry: "child-interpreter",
                },
                None,
            )
            .with_compiler_loc()
        })?
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let mut children = Vec::with_capacity(child_handles.len());
    for child in child_handles {
        let child_id = child
            .try_borrow()
            .map_err(|_| {
                MechError::new(
                    IntegrityConstraintRegistryBorrowConflict {
                        interpreter_id: interpreter.id,
                        registry: "child-interpreter",
                    },
                    None,
                )
                .with_compiler_loc()
            })?
            .id;
        children.push((child_id, child));
    }
    children.sort_by_key(|(child_id, _)| *child_id);
    for (_, child) in children {
        let child_borrow = child.try_borrow().map_err(|_| {
            MechError::new(
                IntegrityConstraintRegistryBorrowConflict {
                    interpreter_id: interpreter.id,
                    registry: "child-interpreter",
                },
                None,
            )
            .with_compiler_loc()
        })?;
        collect_interpreter_constraints(&child_borrow, Some(&child), visited, evaluations)?;
    }
    Ok(())
}

fn evaluate_constraint(
    interpreter_id: u64,
    constraint: &IntegrityConstraint,
) -> IntegrityConstraintEvaluation {
    let resolved = constraint
        .result
        .try_borrow()
        .map_err(|_| ())
        .and_then(|value| resolve_value(&value));
    match resolved {
        Err(()) => constraint_evaluation(
            interpreter_id,
            constraint,
            false,
            Some(IntegrityConstraintFailureReason::BorrowConflict),
            None,
            None,
            None,
        ),
        Ok(resolved) if resolved.scalar_bool == Some(true) => constraint_evaluation(
            interpreter_id,
            constraint,
            true,
            None,
            Some(resolved.kind),
            Some("true".to_string()),
            Some("true".to_string()),
        ),
        Ok(resolved) if resolved.scalar_bool == Some(false) => {
            let lhs = format_operand(constraint.lhs.as_ref());
            let rhs = format_operand(constraint.rhs.as_ref());
            let (actual, expected) = if constraint.lhs.is_some() || constraint.rhs.is_some() {
                (lhs, rhs)
            } else {
                (Some("false".to_string()), Some("true".to_string()))
            };
            constraint_evaluation(
                interpreter_id,
                constraint,
                false,
                Some(IntegrityConstraintFailureReason::EvaluatedFalse),
                Some(resolved.kind),
                actual,
                expected,
            )
        }
        Ok(resolved) => constraint_evaluation(
            interpreter_id,
            constraint,
            false,
            Some(IntegrityConstraintFailureReason::ExpectedBool),
            Some(resolved.kind),
            Some(resolved.formatted),
            Some("scalar bool true".to_string()),
        ),
    }
}

fn constraint_evaluation(
    interpreter_id: u64,
    constraint: &IntegrityConstraint,
    passed: bool,
    reason: Option<IntegrityConstraintFailureReason>,
    evaluated_kind: Option<ValueKind>,
    actual: Option<String>,
    expected: Option<String>,
) -> IntegrityConstraintEvaluation {
    IntegrityConstraintEvaluation {
        interpreter_id,
        constraint_id: constraint.id,
        name: constraint.name.clone(),
        expression: constraint.expression.clone(),
        passed,
        reason,
        evaluated_kind,
        actual,
        operator: constraint.operator.clone(),
        expected,
        tokens: constraint.tokens.clone(),
    }
}

fn format_operand(operand: Option<&ValRef>) -> Option<String> {
    let operand = operand?;
    let value = operand.try_borrow().ok()?;
    resolve_value(&value)
        .ok()
        .map(|resolved| resolved.formatted)
}

fn resolve_value(value: &LegacyValue) -> Result<ResolvedIntegrityValue, ()> {
    match value {
        LegacyValue::MutableReference(reference) => {
            let value = reference.try_borrow().map_err(|_| ())?;
            resolve_value(&value)
        }
        LegacyValue::Typed(value, _) => resolve_value(value),
        #[cfg(any(feature = "bool", feature = "variable_define"))]
        LegacyValue::Bool(value) => {
            let value = *value.try_borrow().map_err(|_| ())?;
            Ok(ResolvedIntegrityValue {
                kind: ValueKind::Bool,
                scalar_bool: Some(value),
                formatted: value.to_string(),
            })
        }
        _ => Ok(ResolvedIntegrityValue {
            kind: stable_value_kind(value)?,
            scalar_bool: None,
            formatted: stable_value_string(value)?,
        }),
    }
}

fn stable_value_string(value: &LegacyValue) -> Result<String, ()> {
    match value {
        #[cfg(feature = "u8")]
        LegacyValue::U8(value) => Ok(value.try_borrow().map_err(|_| ())?.to_string()),
        #[cfg(feature = "u16")]
        LegacyValue::U16(value) => Ok(value.try_borrow().map_err(|_| ())?.to_string()),
        #[cfg(feature = "u32")]
        LegacyValue::U32(value) => Ok(value.try_borrow().map_err(|_| ())?.to_string()),
        #[cfg(feature = "u64")]
        LegacyValue::U64(value) => Ok(value.try_borrow().map_err(|_| ())?.to_string()),
        #[cfg(feature = "u128")]
        LegacyValue::U128(value) => Ok(value.try_borrow().map_err(|_| ())?.to_string()),
        #[cfg(feature = "i8")]
        LegacyValue::I8(value) => Ok(value.try_borrow().map_err(|_| ())?.to_string()),
        #[cfg(feature = "i16")]
        LegacyValue::I16(value) => Ok(value.try_borrow().map_err(|_| ())?.to_string()),
        #[cfg(feature = "i32")]
        LegacyValue::I32(value) => Ok(value.try_borrow().map_err(|_| ())?.to_string()),
        #[cfg(feature = "i64")]
        LegacyValue::I64(value) => Ok(value.try_borrow().map_err(|_| ())?.to_string()),
        #[cfg(feature = "i128")]
        LegacyValue::I128(value) => Ok(value.try_borrow().map_err(|_| ())?.to_string()),
        #[cfg(feature = "f32")]
        LegacyValue::F32(value) => Ok(value.try_borrow().map_err(|_| ())?.to_string()),
        #[cfg(feature = "f64")]
        LegacyValue::F64(value) => Ok(value.try_borrow().map_err(|_| ())?.to_string()),
        #[cfg(any(feature = "string", feature = "variable_define"))]
        LegacyValue::String(value) => Ok(format!("\"{}\"", value.try_borrow().map_err(|_| ())?)),
        #[cfg(feature = "complex")]
        LegacyValue::C64(value) => Ok(value.try_borrow().map_err(|_| ())?.to_string()),
        #[cfg(feature = "rational")]
        LegacyValue::R64(value) => Ok(value.try_borrow().map_err(|_| ())?.to_string()),
        #[cfg(feature = "atom")]
        LegacyValue::Atom(value) => {
            let atom = value.try_borrow().map_err(|_| ())?;
            let dictionary = atom.0.1.try_borrow().map_err(|_| ())?;
            let name = dictionary
                .get(&atom.0.0)
                .cloned()
                .unwrap_or_else(|| atom.0.0.to_string());
            Ok(format!(":{name}"))
        }
        LegacyValue::Id(value) => Ok(value.to_string()),
        LegacyValue::Index(value) => Ok(value.try_borrow().map_err(|_| ())?.to_string()),
        LegacyValue::Empty => Ok("_".to_string()),
        LegacyValue::EmptyKind(kind) => Ok(format!("<{}>", kind)),
        LegacyValue::Kind(kind) => Ok(format!("<{}>", kind)),
        LegacyValue::IndexAll => Ok(":".to_string()),
        _ => Ok(format!("<{}>", stable_value_kind(value)?)),
    }
}

fn stable_value_kind(value: &LegacyValue) -> Result<ValueKind, ()> {
    match value {
        #[cfg(feature = "complex")]
        LegacyValue::C64(_) => Ok(ValueKind::C64),
        #[cfg(feature = "rational")]
        LegacyValue::R64(_) => Ok(ValueKind::R64),
        #[cfg(feature = "u8")]
        LegacyValue::U8(_) => Ok(ValueKind::U8),
        #[cfg(feature = "u16")]
        LegacyValue::U16(_) => Ok(ValueKind::U16),
        #[cfg(feature = "u32")]
        LegacyValue::U32(_) => Ok(ValueKind::U32),
        #[cfg(feature = "u64")]
        LegacyValue::U64(_) => Ok(ValueKind::U64),
        #[cfg(feature = "u128")]
        LegacyValue::U128(_) => Ok(ValueKind::U128),
        #[cfg(feature = "i8")]
        LegacyValue::I8(_) => Ok(ValueKind::I8),
        #[cfg(feature = "i16")]
        LegacyValue::I16(_) => Ok(ValueKind::I16),
        #[cfg(feature = "i32")]
        LegacyValue::I32(_) => Ok(ValueKind::I32),
        #[cfg(feature = "i64")]
        LegacyValue::I64(_) => Ok(ValueKind::I64),
        #[cfg(feature = "i128")]
        LegacyValue::I128(_) => Ok(ValueKind::I128),
        #[cfg(feature = "f32")]
        LegacyValue::F32(_) => Ok(ValueKind::F32),
        #[cfg(feature = "f64")]
        LegacyValue::F64(_) => Ok(ValueKind::F64),
        #[cfg(any(feature = "string", feature = "variable_define"))]
        LegacyValue::String(_) => Ok(ValueKind::String),
        #[cfg(any(feature = "bool", feature = "variable_define"))]
        LegacyValue::Bool(_) => Ok(ValueKind::Bool),
        #[cfg(feature = "atom")]
        LegacyValue::Atom(value) => {
            let atom = value.try_borrow().map_err(|_| ())?;
            let dictionary = atom.0.1.try_borrow().map_err(|_| ())?;
            let name = dictionary
                .get(&atom.0.0)
                .cloned()
                .unwrap_or_else(|| atom.0.0.to_string());
            Ok(ValueKind::Atom(atom.0.0, name))
        }
        #[cfg(feature = "matrix")]
        LegacyValue::MatrixIndex(_) => {
            Ok(ValueKind::Matrix(Box::new(ValueKind::Index), Vec::new()))
        }
        #[cfg(all(feature = "matrix", feature = "bool"))]
        LegacyValue::MatrixBool(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::Bool), Vec::new())),
        #[cfg(all(feature = "matrix", feature = "u8"))]
        LegacyValue::MatrixU8(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::U8), Vec::new())),
        #[cfg(all(feature = "matrix", feature = "u16"))]
        LegacyValue::MatrixU16(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::U16), Vec::new())),
        #[cfg(all(feature = "matrix", feature = "u32"))]
        LegacyValue::MatrixU32(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::U32), Vec::new())),
        #[cfg(all(feature = "matrix", feature = "u64"))]
        LegacyValue::MatrixU64(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::U64), Vec::new())),
        #[cfg(all(feature = "matrix", feature = "u128"))]
        LegacyValue::MatrixU128(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::U128), Vec::new())),
        #[cfg(all(feature = "matrix", feature = "i8"))]
        LegacyValue::MatrixI8(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::I8), Vec::new())),
        #[cfg(all(feature = "matrix", feature = "i16"))]
        LegacyValue::MatrixI16(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::I16), Vec::new())),
        #[cfg(all(feature = "matrix", feature = "i32"))]
        LegacyValue::MatrixI32(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::I32), Vec::new())),
        #[cfg(all(feature = "matrix", feature = "i64"))]
        LegacyValue::MatrixI64(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::I64), Vec::new())),
        #[cfg(all(feature = "matrix", feature = "i128"))]
        LegacyValue::MatrixI128(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::I128), Vec::new())),
        #[cfg(all(feature = "matrix", feature = "f32"))]
        LegacyValue::MatrixF32(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::F32), Vec::new())),
        #[cfg(all(feature = "matrix", feature = "f64"))]
        LegacyValue::MatrixF64(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::F64), Vec::new())),
        #[cfg(all(feature = "matrix", feature = "string"))]
        LegacyValue::MatrixString(_) => {
            Ok(ValueKind::Matrix(Box::new(ValueKind::String), Vec::new()))
        }
        #[cfg(all(feature = "matrix", feature = "rational"))]
        LegacyValue::MatrixR64(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::R64), Vec::new())),
        #[cfg(all(feature = "matrix", feature = "complex"))]
        LegacyValue::MatrixC64(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::C64), Vec::new())),
        #[cfg(feature = "matrix")]
        LegacyValue::MatrixValue(_) => Ok(ValueKind::Matrix(Box::new(ValueKind::Any), Vec::new())),
        #[cfg(feature = "set")]
        LegacyValue::Set(_) => Ok(ValueKind::Set(Box::new(ValueKind::Any), None)),
        #[cfg(feature = "map")]
        LegacyValue::Map(_) => Ok(ValueKind::Map(
            Box::new(ValueKind::Any),
            Box::new(ValueKind::Any),
        )),
        #[cfg(feature = "record")]
        LegacyValue::Record(_) => Ok(ValueKind::Record(Vec::new())),
        #[cfg(feature = "table")]
        LegacyValue::Table(_) => Ok(ValueKind::Table(Vec::new(), 0)),
        #[cfg(feature = "tuple")]
        LegacyValue::Tuple(_) => Ok(ValueKind::Tuple(Vec::new())),
        #[cfg(feature = "enum")]
        LegacyValue::Enum(value) => {
            let enum_value = value.try_borrow().map_err(|_| ())?;
            let dictionary = enum_value.names.try_borrow().map_err(|_| ())?;
            let name = dictionary
                .get(&enum_value.id)
                .cloned()
                .unwrap_or_else(|| enum_value.id.to_string());
            Ok(ValueKind::Enum(enum_value.id, name))
        }
        LegacyValue::Id(_) => Ok(ValueKind::Id),
        LegacyValue::Index(_) => Ok(ValueKind::Index),
        LegacyValue::Empty => Ok(ValueKind::Empty),
        LegacyValue::EmptyKind(kind) | LegacyValue::Kind(kind) => Ok(kind.clone()),
        LegacyValue::IndexAll => Ok(ValueKind::Empty),
        LegacyValue::Typed(_, kind) => Ok(kind.clone()),
        LegacyValue::MutableReference(reference) => {
            let value = reference.try_borrow().map_err(|_| ())?;
            stable_value_kind(&value)
        }
    }
}

#[cfg(all(test, feature = "source"))]
mod tests {
    use super::*;
    use crate::{MechProgramConfig, ProgramInputUpdate};
    use mech_syntax::parser;
    use std::{cell::Cell, rc::Rc};

    fn program_with_constraint(source: &str) -> MechProgram {
        let mut program = MechProgram::with_function_catalog(
            MechProgramConfig::default(),
            crate::test_support::catalog::function_catalog(),
        );
        program.run_string(source).unwrap();
        program
    }

    fn set_constraint_result(program: &MechProgram, name: &str, result: LegacyValue) {
        program
            .interpreter()
            .state
            .borrow_mut()
            .integrity_constraints
            .get_mut(&hash_str(name))
            .unwrap()
            .result = Ref::new(result);
    }

    fn install_constraint_result(program: &MechProgram, name: &str, result: Ref<bool>) {
        let id = hash_str(name);
        program
            .interpreter()
            .state
            .borrow_mut()
            .integrity_constraints
            .insert(
                id,
                IntegrityConstraint {
                    id,
                    name: name.to_string(),
                    expression: name.to_string(),
                    result: Ref::new(LegacyValue::Bool(result)),
                    lhs: None,
                    operator: None,
                    rhs: None,
                    tokens: Vec::new(),
                },
            );
    }

    #[test]
    fn scalar_constraint_results_are_classified_without_mutation() {
        let passing = program_with_constraint("safe! := true");
        let report = passing.integrity_constraint_report().unwrap();
        assert_eq!(report.checked, 1);
        assert!(report.violations.is_empty());
        assert!(report.evaluations[0].passed);

        let false_result = program_with_constraint("safe! := false");
        let error = false_result.validate_integrity_constraints().unwrap_err();
        let failures = error.kind_as::<IntegrityConstraintViolationSet>().unwrap();
        assert_eq!(failures.checked, 1);
        assert_eq!(failures.evaluations.len(), 1);
        assert_eq!(
            failures.violations[0].reason,
            IntegrityConstraintFailureReason::EvaluatedFalse,
        );

        let non_bool = program_with_constraint("safe! := 42.0");
        let failure = non_bool
            .integrity_constraint_report()
            .unwrap()
            .violations
            .remove(0);
        assert_eq!(
            failure.reason,
            IntegrityConstraintFailureReason::ExpectedBool,
        );
        assert_eq!(failure.evaluated_kind, Some(ValueKind::F64));
        assert_eq!(failure.actual.as_deref(), Some("42"));
    }

    #[test]
    fn reports_derive_complete_violation_sets_without_reordering_evaluations() {
        let passing = program_with_constraint("first! := true")
            .integrity_constraint_report()
            .unwrap()
            .evaluations
            .remove(0);
        let failing = program_with_constraint("second! := false")
            .integrity_constraint_report()
            .unwrap()
            .evaluations
            .remove(0);

        let report = IntegrityConstraintReport::from_evaluations(vec![passing, failing]);
        assert_eq!(report.checked, 2);
        assert_eq!(report.evaluations[0].name, "first!");
        assert_eq!(report.evaluations[1].name, "second!");
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].name, "second!");

        let failures = report.into_violation_set().unwrap();
        assert_eq!(failures.checked, 2);
        assert_eq!(failures.evaluations.len(), 2);
        assert_eq!(failures.evaluations[0].name, "first!");
        assert_eq!(failures.evaluations[1].name, "second!");
    }

    #[test]
    fn typed_and_mutable_reference_results_are_resolved() {
        for (wrapped, passes) in [
            (
                LegacyValue::Typed(Box::new(LegacyValue::Bool(Ref::new(true))), ValueKind::Bool),
                true,
            ),
            (
                LegacyValue::Typed(
                    Box::new(LegacyValue::Bool(Ref::new(false))),
                    ValueKind::Bool,
                ),
                false,
            ),
            (
                LegacyValue::MutableReference(Ref::new(LegacyValue::Bool(Ref::new(true)))),
                true,
            ),
            (
                LegacyValue::MutableReference(Ref::new(LegacyValue::Bool(Ref::new(false)))),
                false,
            ),
        ] {
            let program = program_with_constraint("wrapped! := true");
            set_constraint_result(&program, "wrapped!", wrapped);
            let report = program.integrity_constraint_report().unwrap();
            assert_eq!(report.checked, 1);
            assert_eq!(report.evaluations[0].passed, passes);
            assert_eq!(report.violations.is_empty(), passes);
        }
    }

    #[test]
    fn violations_are_aggregated_in_stable_constraint_order() {
        let program = program_with_constraint("later! := false\nearlier! := 7.0\npassing! := true");
        let report = program.integrity_constraint_report().unwrap();
        assert_eq!(report.checked, 3);
        assert_eq!(report.violations.len(), 2);
        assert!(
            report
                .evaluations
                .windows(2)
                .all(|pair| pair[0].constraint_id < pair[1].constraint_id)
        );
        assert!(
            report
                .violations
                .windows(2)
                .all(|pair| pair[0].constraint_id < pair[1].constraint_id)
        );
    }

    #[test]
    fn hierarchy_validation_is_complete_and_keyed_by_interpreter() {
        let mut program = program_with_constraint("shared! := false");
        let root_id = program.interpreter().id;
        let child_id = root_id.wrapping_add(101);
        let grandchild_id = root_id.wrapping_add(202);
        let mut child = Interpreter::with_function_catalog(
            child_id,
            10_000,
            crate::test_support::catalog::function_catalog(),
        );
        child
            .interpret(&parser::parse("shared! := false").unwrap())
            .unwrap();
        let mut grandchild = Interpreter::with_function_catalog(
            grandchild_id,
            10_000,
            crate::test_support::catalog::function_catalog(),
        );
        grandchild
            .interpret(&parser::parse("nested! := false").unwrap())
            .unwrap();
        child
            .sub_interpreters
            .borrow_mut()
            .insert(grandchild_id, Ref::new(Box::new(grandchild)));
        program
            .interpreter_mut()
            .sub_interpreters
            .borrow_mut()
            .insert(child_id, Ref::new(Box::new(child)));

        let report = program.integrity_constraint_report().unwrap();

        assert_eq!(report.checked, 3);
        assert_eq!(report.violations.len(), 3);
        assert_eq!(
            report
                .violations
                .iter()
                .map(|failure| failure.interpreter_id)
                .collect::<Vec<_>>(),
            vec![root_id, child_id, grandchild_id],
        );
        let shared_id = hash_str("shared!");
        assert_eq!(
            report
                .violations
                .iter()
                .filter(|failure| failure.constraint_id == shared_id)
                .count(),
            2,
        );
    }

    #[test]
    fn repeated_interpreter_handle_is_an_infrastructure_error() {
        let mut program = MechProgram::new(MechProgramConfig::default());
        let child_id = program.interpreter().id.wrapping_add(1);
        let child = Ref::new(Box::new(Interpreter::new(child_id, 10_000)));
        program
            .interpreter_mut()
            .sub_interpreters
            .borrow_mut()
            .insert(1, child.clone());
        program
            .interpreter_mut()
            .sub_interpreters
            .borrow_mut()
            .insert(2, child);

        let error = program.integrity_constraint_report().unwrap_err();

        assert_eq!(error.kind_name(), "IntegrityConstraintHierarchyConflict");
        assert!(!error.kind_message().contains("0x"));
    }

    #[test]
    fn result_borrow_conflict_is_an_aggregated_constraint_failure() {
        let program = program_with_constraint("safe! := true");
        let result = program
            .interpreter()
            .state
            .borrow()
            .integrity_constraints
            .values()
            .next()
            .unwrap()
            .result
            .clone();
        let _borrow = result.borrow_mut();

        let report = program.integrity_constraint_report().unwrap();

        assert_eq!(report.violations.len(), 1);
        assert_eq!(
            report.violations[0].reason,
            IntegrityConstraintFailureReason::BorrowConflict,
        );
        assert_eq!(report.violations[0].evaluated_kind, None);
    }

    #[test]
    fn operand_borrow_conflict_preserves_evaluated_false_reason() {
        let program =
            program_with_constraint("target := 2.0\nmaximum := 1.0\nsafe! := target <= maximum");
        let lhs = program
            .interpreter()
            .state
            .borrow()
            .integrity_constraints
            .get(&hash_str("safe!"))
            .unwrap()
            .lhs
            .clone()
            .unwrap();
        let _borrow = lhs.borrow_mut();

        let report = program.integrity_constraint_report().unwrap();

        assert_eq!(
            report.violations[0].reason,
            IntegrityConstraintFailureReason::EvaluatedFalse,
        );
        assert_eq!(report.violations[0].actual, None);
        assert_eq!(report.violations[0].expected.as_deref(), Some("1"),);
    }

    #[test]
    fn reporting_is_repeatable_and_does_not_change_program_state() {
        let program =
            program_with_constraint("target := 1.0\nmaximum := 2.0\nsafe! := target <= maximum");
        let plan_handle = program.interpreter().plan().0.id();
        let pending_before = program.interpreter().has_pending_reactive_registers();
        let state_len = program
            .interpreter()
            .state
            .borrow()
            .integrity_constraints
            .len();

        let first = program.integrity_constraint_report().unwrap();
        let second = program.integrity_constraint_report().unwrap();

        let summary = |report: &IntegrityConstraintReport| {
            report
                .evaluations
                .iter()
                .map(|evaluation| {
                    (
                        evaluation.interpreter_id,
                        evaluation.constraint_id,
                        evaluation.passed,
                        evaluation.reason.clone(),
                        evaluation.actual.clone(),
                        evaluation.expected.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(summary(&first), summary(&second));
        assert_eq!(program.interpreter().plan().0.id(), plan_handle);
        assert_eq!(
            program.interpreter().has_pending_reactive_registers(),
            pending_before,
        );
        assert_eq!(
            program
                .interpreter()
                .state
                .borrow()
                .integrity_constraints
                .len(),
            state_len,
        );
    }

    struct IntegrityOperationFunction {
        next_result: Rc<Cell<bool>>,
        result: Ref<bool>,
        output: Ref<usize>,
        hidden: Ref<usize>,
    }

    impl MechFunctionImpl for IntegrityOperationFunction {
        fn solve_result(&self) -> MResult<()> {
            *self.result.borrow_mut() = self.next_result.get();
            *self.output.borrow_mut() += 1;
            *self.hidden.borrow_mut() += 1;
            Ok(())
        }

        fn out(&self) -> LegacyValue {
            LegacyValue::Bool(self.result.clone())
        }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
            Ok(vec![
                LegacyValue::Bool(self.result.clone()),
                LegacyValue::Index(self.output.clone()),
                LegacyValue::Index(self.hidden.clone()),
            ])
        }

        fn to_string(&self) -> String {
            "integrity-operation".to_string()
        }
    }

    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for IntegrityOperationFunction {
        fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    fn integrity_operation_function(
        next_result: Rc<Cell<bool>>,
        result: Ref<bool>,
        output: Ref<usize>,
        hidden: Ref<usize>,
    ) -> IntegrityOperationFunction {
        IntegrityOperationFunction {
            next_result,
            result,
            output,
            hidden,
        }
    }

    fn assert_step_integrity_rollback(step_id: u64) {
        let mut program = MechProgram::new(MechProgramConfig::default());
        let next_result = Rc::new(Cell::new(false));
        let result = Ref::new(true);
        let output = Ref::new(10usize);
        let hidden = Ref::new(20usize);
        program
            .interpreter()
            .plan()
            .add_function(Box::new(integrity_operation_function(
                next_result.clone(),
                result.clone(),
                output.clone(),
                hidden.clone(),
            )));
        install_constraint_result(&program, "step-safe!", result.clone());

        let error = program.step(step_id).unwrap_err();

        assert_eq!(error.kind_name(), "IntegrityConstraintViolationSet");
        assert_eq!(
            (*result.borrow(), *output.borrow(), *hidden.borrow()),
            (true, 10, 20),
        );
        next_result.set(true);
        program.step(step_id).unwrap();
        assert_eq!(
            (*result.borrow(), *output.borrow(), *hidden.borrow()),
            (true, 11, 21),
        );
    }

    #[test]
    fn whole_plan_step_rolls_back_an_invalid_candidate() {
        assert_step_integrity_rollback(0);
    }

    #[test]
    fn selected_step_rolls_back_an_invalid_candidate() {
        assert_step_integrity_rollback(1);
    }

    #[test]
    fn advance_reactive_turn_rolls_back_an_invalid_candidate() {
        let mut program = MechProgram::new(MechProgramConfig::default());
        let interpreter_id = program.interpreter().id;
        let trigger = Ref::new(0usize);
        let next_result = Rc::new(Cell::new(false));
        let result = Ref::new(true);
        let output = Ref::new(10usize);
        let hidden = Ref::new(20usize);
        program
            .interpreter()
            .plan()
            .0
            .borrow_mut()
            .register(
                Box::new(integrity_operation_function(
                    next_result.clone(),
                    result.clone(),
                    output.clone(),
                    hidden.clone(),
                )),
                &[LegacyValue::Index(trigger.clone())],
            )
            .unwrap();
        install_constraint_result(&program, "advance-safe!", result.clone());
        let dirty_cells = LegacyValue::Index(trigger).reactive_root_cell_ids();

        let error = program
            .advance_reactive_turn(interpreter_id, &dirty_cells)
            .unwrap_err();

        assert_eq!(error.kind_name(), "IntegrityConstraintViolationSet");
        assert_eq!(
            (*result.borrow(), *output.borrow(), *hidden.borrow()),
            (true, 10, 20),
        );
        assert!(!program.interpreter().has_pending_reactive_registers());
        next_result.set(true);
        program
            .advance_reactive_turn(interpreter_id, &dirty_cells)
            .unwrap();
        assert_eq!(
            (*result.borrow(), *output.borrow(), *hidden.borrow()),
            (true, 11, 21),
        );
    }

    #[test]
    fn invalid_reactive_candidate_rolls_back_and_later_valid_turn_succeeds() {
        let mut program = MechProgram::with_function_catalog(
            MechProgramConfig::default(),
            crate::test_support::catalog::function_catalog(),
        );
        let interpreter_id = program.interpreter().id;
        let input = program
            .ensure_input(
                interpreter_id,
                hash_str("input"),
                "input",
                LegacyValue::F64(Ref::new(1.0)),
            )
            .unwrap();
        program
            .run_string("output := input * 2.0\nsafe! := output <= 10.0")
            .unwrap();

        let error = program
            .update_inputs_and_advance_turn(&[ProgramInputUpdate {
                input,
                value: LegacyValue::F64(Ref::new(6.0)),
            }])
            .unwrap_err();
        assert_eq!(error.kind_name(), "IntegrityConstraintViolationSet");
        assert_eq!(
            *program
                .interpreter()
                .symbols()
                .borrow()
                .get(hash_str("input"))
                .unwrap()
                .borrow()
                .as_f64()
                .unwrap()
                .borrow(),
            1.0,
        );
        assert_eq!(
            *program
                .interpreter()
                .symbols()
                .borrow()
                .get(hash_str("output"))
                .unwrap()
                .borrow()
                .as_f64()
                .unwrap()
                .borrow(),
            2.0,
        );
        program.validate_integrity_constraints().unwrap();

        program
            .update_inputs_and_advance_turn(&[ProgramInputUpdate {
                input,
                value: LegacyValue::F64(Ref::new(4.0)),
            }])
            .unwrap();
        program.validate_integrity_constraints().unwrap();
    }
}
