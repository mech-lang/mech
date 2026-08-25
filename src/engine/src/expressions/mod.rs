#![forbid(unsafe_code)]

use crate::{
    Expression, FeatureNotEnabledError, InterpreterExecution, LegacyValue, MResult, MechError,
    literal, structure,
};

use std::collections::HashMap;

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
mod comprehensions;
mod environment;
mod errors;
mod formulas;
#[cfg(feature = "functions")]
mod functions;
mod matches;
#[cfg(any(
    feature = "range_inclusive",
    feature = "range_exclusive",
    feature = "range_inclusive_increment",
    feature = "range_exclusive_increment",
    feature = "subscript_range"
))]
mod ranges;
#[cfg(feature = "functions")]
mod registration;
#[cfg(any(
    all(feature = "subscript_slice", feature = "access"),
    feature = "subscript_formula",
    feature = "subscript_range",
    all(feature = "subscript", feature = "access")
))]
mod subscripts;
mod variables;

#[cfg(feature = "matrix_comprehensions")]
pub use comprehensions::{
    MatrixComprehensionDefine, ValueMatrixComprehension, matrix_comprehension,
};
#[cfg(feature = "set_comprehensions")]
pub use comprehensions::{SetComprehensionDefine, ValueSetComprehension, set_comprehension};
pub(crate) use environment::DeferredExpressionSolveScope;
pub use errors::{
    ArityMismatchError, ComprehensionGeneratorError, InvalidGuardExpressionError,
    InvalidIndexKindError, MatchArmKindMismatchError, MatchNoArmMatchedError,
    MatchNonExhaustiveError, MatchNonExhaustiveVariantsError, PatternExpectedTupleError,
    PatternMatchError, ReactiveComprehensionStructureUnsupported, UndefinedVariableError,
    UnhandledFormulaOperatorError,
};
pub use formulas::{factor, term};
#[cfg(feature = "functions")]
pub use functions::function_call;
pub use matches::match_expression;
pub(crate) use matches::validate_guard_expression_result;
#[cfg(any(
    feature = "range_inclusive",
    feature = "range_exclusive",
    feature = "range_inclusive_increment",
    feature = "range_exclusive_increment",
    feature = "subscript_range"
))]
pub use ranges::range;
#[cfg(feature = "functions")]
use registration::{register_expression_function_batch, register_initialized_expression_function};
#[cfg(all(feature = "subscript_slice", feature = "access"))]
pub use subscripts::slice;
#[cfg(all(feature = "subscript", feature = "access"))]
pub use subscripts::subscript;
#[cfg(feature = "subscript_range")]
pub use subscripts::subscript_range;
#[cfg(feature = "subscript_formula")]
pub(crate) use subscripts::{
    current_string_access_expression_live, mark_current_string_access_expression_live,
    mark_string_access_value_live, reset_current_string_access_expression_live,
    string_access_input_is_live, string_access_value_is_marked_live,
    take_current_string_access_expression_live,
};
#[cfg(feature = "subscript_formula")]
pub use subscripts::{subscript_formula, subscript_formula_ix};
#[cfg(feature = "symbol_table")]
pub use variables::var;

#[cfg(test)]
mod tests;

// Expressions
// ----------------------------------------------------------------------------

pub type Environment = HashMap<u64, LegacyValue>;

pub fn expression(
    expr: &Expression,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    match &expr {
        #[cfg(feature = "variables")]
        Expression::Var(v) => var(v, env, p),
        #[cfg(any(
            feature = "range_inclusive",
            feature = "range_exclusive",
            feature = "range_inclusive_increment",
            feature = "range_exclusive_increment"
        ))]
        Expression::Range(rng) => range(&rng, env, p),
        #[cfg(all(feature = "subscript_slice", feature = "access"))]
        Expression::Slice(slc) => slice(&slc, env, p),
        Expression::Formula(fctr) => factor(fctr, env, p),
        Expression::Structure(strct) => structure(strct, env, p),
        Expression::Literal(ltrl) => literal(&ltrl, p),
        #[cfg(feature = "functions")]
        Expression::FunctionCall(fxn_call) => function_call(fxn_call, env, p),
        #[cfg(feature = "set_comprehensions")]
        Expression::SetComprehension(set_comp) => set_comprehension(set_comp, p),
        #[cfg(feature = "matrix_comprehensions")]
        Expression::MatrixComprehension(matrix_comp) => matrix_comprehension(matrix_comp, p),
        Expression::Match(match_expr) => match_expression(match_expr, env, p),
        #[cfg(feature = "state_machines")]
        Expression::FsmPipe(fsm_pipe) => crate::state_machines::execute_fsm_pipe(fsm_pipe, env, p),
        x => Err(MechError::new(FeatureNotEnabledError, None)
            .with_compiler_loc()
            .with_tokens(x.tokens())),
    }
}
