use super::{Environment, factor};
use crate::{
    InterpreterExecution, MResult, OperationId, RangeExpression, RangeOp, SpecializationInvocation,
    ValueCell,
};

pub fn range(
    rng: &RangeExpression,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    use super::registration::register_initialized_expression_function;
    let plan = p.plan();
    let start = factor(&rng.start, env, p)?;
    let terminal = factor(&rng.terminal, env, p)?;
    let function = match &rng.increment {
        Some((_, increment)) => {
            let step = factor(increment, env, p)?;
            let arguments = vec![start, step, terminal];
            let canonical_name = match &rng.operator {
                #[cfg(feature = "range_exclusive")]
                RangeOp::Exclusive => "range/exclusive-increment",
                #[cfg(feature = "range_inclusive")]
                RangeOp::Inclusive => "range/inclusive-increment",
                #[cfg(not(all(feature = "range_exclusive", feature = "range_inclusive")))]
                _ => unreachable!(),
            };
            let invocation =
                SpecializationInvocation::from_cells(arguments.clone().into_boxed_slice());
            let function = p.specialize_visible_invocation_named(
                OperationId::from_name(canonical_name),
                Some(canonical_name),
                &invocation,
            )?;
            function
        }
        None => {
            let arguments = vec![start, terminal];
            let canonical_name = match &rng.operator {
                #[cfg(feature = "range_exclusive")]
                RangeOp::Exclusive => "range/exclusive",
                #[cfg(feature = "range_inclusive")]
                RangeOp::Inclusive => "range/inclusive",
                #[cfg(not(all(feature = "range_exclusive", feature = "range_inclusive")))]
                _ => unreachable!(),
            };
            let invocation =
                SpecializationInvocation::from_cells(arguments.clone().into_boxed_slice());
            let function = p.specialize_visible_invocation_named(
                OperationId::from_name(canonical_name),
                Some(canonical_name),
                &invocation,
            )?;
            function
        }
    };
    register_initialized_expression_function(&plan, function)
}
