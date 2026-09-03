use crate::{FormulaOperator, MechErrorKind, ResolvedType};

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
pub struct ComprehensionGeneratorError {
    pub(super) found: ResolvedType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactiveComprehensionStructureUnsupported {
    pub qualifier: &'static str,
}

impl MechErrorKind for ReactiveComprehensionStructureUnsupported {
    fn name(&self) -> &str {
        "ReactiveComprehensionStructureUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "A live comprehension {} can change resident matrix membership, but resident matrix shapes are fixed during activation. Keep generator and filter membership stable; live values remain supported in let qualifiers and the comprehension result.",
            self.qualifier,
        )
    }
}

impl MechErrorKind for ComprehensionGeneratorError {
    fn name(&self) -> &str {
        "ComprehensionGenerator"
    }
    fn message(&self) -> String {
        format!(
            "Comprehension generator must produce a set or matrix, found type: {}",
            self.found.semantic_name()
        )
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
    pub(super) expected: ResolvedType,
    pub(super) found: ResolvedType,
}
impl MechErrorKind for MatchArmKindMismatchError {
    fn name(&self) -> &str {
        "MatchArmKindMismatch"
    }
    fn message(&self) -> String {
        format!(
            "Expected type {}, found {}",
            self.expected.semantic_name(),
            self.found.semantic_name()
        )
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
    pub(super) found: ResolvedType,
}

impl MechErrorKind for InvalidGuardExpressionError {
    fn name(&self) -> &str {
        "InvalidGuardExpression"
    }
    fn message(&self) -> String {
        format!(
            "Guard expressions must evaluate to a boolean value. Found type: {}",
            self.found.semantic_name()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ComprehensionGeneratorError, InvalidGuardExpressionError, MatchArmKindMismatchError,
    };
    use crate::{
        BuiltinScalarKind, CannotConvertToTypeError, MechError, MechErrorKind, ResolvedType,
    };

    fn scalar(kind: BuiltinScalarKind) -> ResolvedType {
        ResolvedType::new(kind.kind_expr(), Box::new([])).unwrap()
    }

    #[test]
    fn conversion_errors_preserve_the_core_public_type_identity() {
        let error = MechError::new(
            CannotConvertToTypeError {
                target_type: "canonical-test-type",
            },
            None,
        );

        assert!(
            error
                .kind_as::<mech_core::CannotConvertToTypeError>()
                .is_some()
        );
    }

    #[test]
    fn source_expression_diagnostics_use_semantic_type_names() {
        assert_eq!(
            ComprehensionGeneratorError {
                found: scalar(BuiltinScalarKind::String),
            }
            .message(),
            "Comprehension generator must produce a set or matrix, found type: string",
        );
        assert_eq!(
            MatchArmKindMismatchError {
                expected: scalar(BuiltinScalarKind::String),
                found: scalar(BuiltinScalarKind::Bool),
            }
            .message(),
            "Expected type string, found bool",
        );
        assert_eq!(
            InvalidGuardExpressionError {
                found: scalar(BuiltinScalarKind::String),
            }
            .message(),
            "Guard expressions must evaluate to a boolean value. Found type: string",
        );
    }
}
