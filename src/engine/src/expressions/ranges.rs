use super::{Environment, factor};
#[cfg(feature = "range_exclusive")]
use crate::RangeExclusive;
#[cfg(feature = "range_inclusive")]
use crate::RangeInclusive;
#[cfg(feature = "range_exclusive")]
use crate::RangeIncrementExclusive;
#[cfg(feature = "range_inclusive")]
use crate::RangeIncrementInclusive;
use crate::{InterpreterExecution, MResult, RangeExpression, RangeOp, Value};

#[cfg(feature = "range")]
pub fn range(
    rng: &RangeExpression,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<Value> {
    use super::registration::register_initialized_expression_function;
    use crate::NativeFunctionCompiler;

    let plan = p.plan();
    let start = factor(&rng.start, env, p)?;
    let terminal = factor(&rng.terminal, env, p)?;
    let (function, arguments) = match &rng.increment {
        Some((_, increment)) => {
            let step = factor(increment, env, p)?;
            let arguments = vec![start, step, terminal];
            let function = match &rng.operator {
                #[cfg(feature = "range_exclusive")]
                RangeOp::Exclusive => RangeIncrementExclusive {}.compile(&arguments)?,
                #[cfg(feature = "range_inclusive")]
                RangeOp::Inclusive => RangeIncrementInclusive {}.compile(&arguments)?,
                _ => unreachable!(),
            };
            (function, arguments)
        }
        None => {
            let arguments = vec![start, terminal];
            let function = match &rng.operator {
                #[cfg(feature = "range_exclusive")]
                RangeOp::Exclusive => RangeExclusive {}.compile(&arguments)?,
                #[cfg(feature = "range_inclusive")]
                RangeOp::Inclusive => RangeInclusive {}.compile(&arguments)?,
                _ => unreachable!(),
            };
            (function, arguments)
        }
    };
    register_initialized_expression_function(&plan, function, &arguments)
}
