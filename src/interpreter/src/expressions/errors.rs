use crate::{FormulaOperator, MechErrorKind, ValueKind};

#[derive(Debug, Clone)]
pub struct UnhandledFormulaOperatorError {
    pub operator: FormulaOperator,
}
impl MechErrorKind for UnhandledFormulaOperatorError {
    fn name(&self) -> &str {
        "UnhandledFormulaOperator"
    }
    fn message(&self) -> String {
        format!("Unhandled formula operator: {:#?}", self.operator)
    }
}

#[derive(Debug, Clone)]
pub struct UndefinedVariableError {
    pub id: u64,
    pub name: String,
}
impl MechErrorKind for UndefinedVariableError {
    fn name(&self) -> &str {
        "UndefinedVariable"
    }

    fn message(&self) -> String {
        format!("Undefined variable `{}` (id: {})", self.name, self.id)
    }
}
#[derive(Debug, Clone)]
pub struct InvalidIndexKindError {
    pub(super) kind: ValueKind,
}
impl MechErrorKind for InvalidIndexKindError {
    fn name(&self) -> &str {
        "InvalidIndexKind"
    }
    fn message(&self) -> String {
        "Invalid index kind".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct ComprehensionGeneratorError {
    pub(super) found: ValueKind,
}

impl MechErrorKind for ComprehensionGeneratorError {
    fn name(&self) -> &str {
        "ComprehensionGenerator"
    }
    fn message(&self) -> String {
        format!(
            "Comprehension generator must produce a set or matrix, found kind: {:?}",
            self.found
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SetComprehensionOutputKindMismatchError {
    pub(super) found: ValueKind,
}

impl MechErrorKind for SetComprehensionOutputKindMismatchError {
    fn name(&self) -> &str {
        "SetComprehensionOutputKindMismatch"
    }

    fn message(&self) -> String {
        format!(
            "Set comprehension bytecode output must be a set, but found {:?}.",
            self.found
        )
    }
}

#[derive(Debug, Clone)]
pub struct PatternExpectedTupleError {
    found: ValueKind,
}
impl MechErrorKind for PatternExpectedTupleError {
    fn name(&self) -> &str {
        "PatternExpectedTuple"
    }
    fn message(&self) -> String {
        format!("Pattern expected a tuple, found kind: {:?}", self.found)
    }
}

#[derive(Debug, Clone)]
pub struct ArityMismatchError {
    expected: usize,
    found: usize,
}
impl MechErrorKind for ArityMismatchError {
    fn name(&self) -> &str {
        "ArityMismatch"
    }
    fn message(&self) -> String {
        format!(
            "Arity mismatch: expected {}, found {}",
            self.expected, self.found
        )
    }
}

#[derive(Debug, Clone)]
pub struct PatternMatchError {
    pub var: String,
    pub expected: String,
    pub found: String,
}

#[derive(Debug, Clone)]
pub struct MatchNoArmMatchedError;
impl MechErrorKind for MatchNoArmMatchedError {
    fn name(&self) -> &str {
        "MatchNoArmMatched"
    }
    fn message(&self) -> String {
        format!("No match arm matched the provided value.")
    }
}

#[derive(Debug, Clone)]
pub struct MatchArmKindMismatchError {
    pub(super) expected: ValueKind,
    pub(super) found: ValueKind,
}
impl MechErrorKind for MatchArmKindMismatchError {
    fn name(&self) -> &str {
        "MatchArmKindMismatch"
    }
    fn message(&self) -> String {
        format!("Expected {:?}, found {:?}", self.expected, self.found)
    }
}

#[derive(Debug, Clone)]
pub struct MatchNonExhaustiveError;
impl MechErrorKind for MatchNonExhaustiveError {
    fn name(&self) -> &str {
        "MatchNonExhaustive"
    }
    fn message(&self) -> String {
        "Match expression must include a wildcard (`*`) arm.".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct MatchNonExhaustiveVariantsError {
    pub enum_name: String,
    pub missing_patterns: Vec<String>,
}
impl MechErrorKind for MatchNonExhaustiveVariantsError {
    fn name(&self) -> &str {
        "MatchNonExhaustive"
    }
    fn message(&self) -> String {
        format!(
            "Match over enum '{}' is non-exhaustive. Missing variants: {}. Handle the missing variants or add a wildcard (`*`) arm to catch all cases.",
            self.enum_name,
            self.missing_patterns.join(", ")
        )
    }
}

impl MechErrorKind for PatternMatchError {
    fn name(&self) -> &str {
        "PatternMatchError"
    }
    fn message(&self) -> String {
        format!(
            "Pattern match error for variable '{}': expected value {}, found value {}",
            self.var, self.expected, self.found
        )
    }
}

#[derive(Debug, Clone)]
pub struct InvalidGuardExpressionError {
    pub(super) found: ValueKind,
}

impl MechErrorKind for InvalidGuardExpressionError {
    fn name(&self) -> &str {
        "InvalidGuardExpression"
    }
    fn message(&self) -> String {
        format!(
            "Guard expressions must evaluate to a boolean value. Found kind: {:?}",
            self.found
        )
    }
}
