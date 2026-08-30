use crate::CompilerPlanningProgram;
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
    pub evaluated_schema: Option<SchemaBody>,
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
    pub evaluated_schema: Option<SchemaBody>,
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
                    evaluated_schema: evaluation.evaluated_schema.clone(),
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
    schema: SchemaBody,
    scalar_bool: Option<bool>,
    formatted: String,
}

impl CompilerPlanningProgram {
    pub fn integrity_constraint_report(&self) -> MResult<IntegrityConstraintReport> {
        let mut evaluations = Vec::new();
        let mut visited = Vec::new();
        collect_interpreter_constraints(&self.interpreter, None, &mut visited, &mut evaluations)?;
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
    let resolved = resolve_cell(&constraint.result);
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
            Some(resolved.schema),
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
                Some(resolved.schema),
                actual,
                expected,
            )
        }
        Ok(resolved) => constraint_evaluation(
            interpreter_id,
            constraint,
            false,
            Some(IntegrityConstraintFailureReason::ExpectedBool),
            Some(resolved.schema),
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
    evaluated_schema: Option<SchemaBody>,
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
        evaluated_schema,
        actual,
        operator: constraint.operator.clone(),
        expected,
        tokens: constraint.tokens.clone(),
    }
}

fn format_operand(operand: Option<&ValueCell>) -> Option<String> {
    let operand = operand?;
    resolve_cell(operand)
        .ok()
        .map(|resolved| resolved.formatted)
}

fn resolve_cell(cell: &ValueCell) -> Result<ResolvedIntegrityValue, ()> {
    let value = cell.snapshot().map_err(|_| ())?;
    let schema = cell.closed_schema_body().map_err(|_| ())?;
    let scalar_bool = match value.data() {
        ValueData::Bool(value) => Some(*value),
        _ => None,
    };
    let formatted = stable_value_string(value.data(), &schema);
    Ok(ResolvedIntegrityValue {
        schema,
        scalar_bool,
        formatted,
    })
}

fn stable_value_string(value: &ValueData, schema: &SchemaBody) -> String {
    match value {
        ValueData::U8(value) => value.to_string(),
        ValueData::U16(value) => value.to_string(),
        ValueData::U32(value) => value.to_string(),
        ValueData::U64(value) => value.to_string(),
        ValueData::U128(value) => value.to_string(),
        ValueData::I8(value) => value.to_string(),
        ValueData::I16(value) => value.to_string(),
        ValueData::I32(value) => value.to_string(),
        ValueData::I64(value) => value.to_string(),
        ValueData::I128(value) => value.to_string(),
        ValueData::F32(value) => value.to_f32().to_string(),
        ValueData::F64(value) => value.to_f64().to_string(),
        ValueData::Bool(value) => value.to_string(),
        ValueData::String(value) => format!("\"{value}\""),
        ValueData::Id(value) | ValueData::Index(value) => value.to_string(),
        _ => format!("<{schema:?}>"),
    }
}

#[cfg(all(test, feature = "source"))]
mod tests {
    use super::*;
    use crate::CompilerPlanningConfig;
    use mech_syntax::parser;

    fn program_with_constraint(source: &str) -> CompilerPlanningProgram {
        let mut program = CompilerPlanningProgram::with_function_catalog(
            CompilerPlanningConfig::default(),
            crate::test_support::catalog::function_catalog(),
        );
        program.plan_source_for_test(source).unwrap();
        program
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
        assert_eq!(
            failure.evaluated_schema,
            Some(SchemaBody::FloatingPoint(FloatWidth::W64))
        );
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
        let program = program_with_constraint("shared! := false");
        let root_id = program.interpreter.id;
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
            .interpreter
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
        let program = CompilerPlanningProgram::new(CompilerPlanningConfig::default());
        let child_id = program.interpreter.id.wrapping_add(1);
        let child = Ref::new(Box::new(Interpreter::new(child_id, 10_000)));
        program
            .interpreter
            .sub_interpreters
            .borrow_mut()
            .insert(1, child.clone());
        program
            .interpreter
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
            .interpreter
            .state
            .borrow()
            .integrity_constraints
            .values()
            .next()
            .unwrap()
            .result
            .clone();
        let result = FunctionInvocation::nullary(result)
            .expect_nullary()
            .unwrap()
            .try_ref::<bool>()
            .unwrap();
        let _borrow = result.borrow_mut();

        let report = program.integrity_constraint_report().unwrap();

        assert_eq!(report.violations.len(), 1);
        assert_eq!(
            report.violations[0].reason,
            IntegrityConstraintFailureReason::BorrowConflict,
        );
        assert_eq!(report.violations[0].evaluated_schema, None);
    }

    #[test]
    fn operand_borrow_conflict_preserves_evaluated_false_reason() {
        let program =
            program_with_constraint("target := 2.0\nmaximum := 1.0\nsafe! := target <= maximum");
        let lhs = program
            .interpreter
            .state
            .borrow()
            .integrity_constraints
            .get(&hash_str("safe!"))
            .unwrap()
            .lhs
            .clone()
            .unwrap();
        let lhs = FunctionInvocation::nullary(lhs)
            .expect_nullary()
            .unwrap()
            .try_ref::<f64>()
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
        let plan_handle = program.interpreter.plan().0.id();
        let pending_before = program.interpreter.has_pending_reactive_registers();
        let state_len = program
            .interpreter
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
        assert_eq!(program.interpreter.plan().0.id(), plan_handle);
        assert_eq!(
            program.interpreter.has_pending_reactive_registers(),
            pending_before,
        );
        assert_eq!(
            program
                .interpreter
                .state
                .borrow()
                .integrity_constraints
                .len(),
            state_len,
        );
    }
}
