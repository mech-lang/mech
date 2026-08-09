use super::{Environment, factor};
use crate::{InterpreterExecution, LegacyValue, MResult, OperationId, RangeExpression, RangeOp};

pub fn range(
    rng: &RangeExpression,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    use super::registration::register_initialized_expression_function;
    let plan = p.plan();
    let start = factor(&rng.start, env, p)?;
    let terminal = factor(&rng.terminal, env, p)?;
    let (function, arguments) = match &rng.increment {
        Some((_, increment)) => {
            let step = factor(increment, env, p)?;
            let arguments = vec![start, step, terminal];
            let canonical_name = match &rng.operator {
                #[cfg(feature = "range_exclusive")]
                RangeOp::Exclusive => "range/exclusive-increment",
                #[cfg(feature = "range_inclusive")]
                RangeOp::Inclusive => "range/inclusive-increment",
                _ => unreachable!(),
            };
            let function = p.specialize_visible_operation_named(
                OperationId::from_name(canonical_name),
                Some(canonical_name),
                &arguments,
            )?;
            (function, arguments)
        }
        None => {
            let arguments = vec![start, terminal];
            let canonical_name = match &rng.operator {
                #[cfg(feature = "range_exclusive")]
                RangeOp::Exclusive => "range/exclusive",
                #[cfg(feature = "range_inclusive")]
                RangeOp::Inclusive => "range/inclusive",
                _ => unreachable!(),
            };
            let function = p.specialize_visible_operation_named(
                OperationId::from_name(canonical_name),
                Some(canonical_name),
                &arguments,
            )?;
            (function, arguments)
        }
    };
    register_initialized_expression_function(&plan, function, &arguments)
}
