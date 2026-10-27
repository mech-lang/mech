//! Native words for proved nonescaping canonical aggregate values. No helper
//! interprets these actions; the ordinary CFG still owns every branch/backedge.
use super::super::{
    NativeCompileError,
    compiler::{backend, guard, offset},
};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeAggregateWord as Word, NativeScalarAggregateAction as Action, NativeScalarAggregates,
};

fn field(
    plan: &NativeScalarAggregates,
    value: mech_engine::native_artifact::NativeMatchValue,
    index: usize,
) -> Result<usize, NativeCompileError> {
    let (base, count) = plan
        .slot(value)
        .ok_or_else(|| backend("aggregate field without native storage"))?;
    if index >= count {
        return Err(backend("aggregate field exceeds native storage"));
    }
    Ok(base + index)
}
fn load(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    index: usize,
) -> Result<Value, NativeCompileError> {
    Ok(b.ins()
        .load(types::I64, MemFlags::new(), locals, offset(index)?))
}

#[cfg(test)]
#[path = "program_map_owner_word_tests.rs"]
mod map_owner_tests;
pub(in crate::native_artifact) fn constants(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    plan: &NativeScalarAggregates,
) -> Result<(), NativeCompileError> {
    for (target, words) in plan.constants() {
        for (i, &word) in words.iter().enumerate() {
            let value = b.ins().iconst(types::I64, word as i64);
            b.ins().store(
                MemFlags::new(),
                value,
                locals,
                offset(field(plan, *target, i)?)?,
            );
        }
    }
    Ok(())
}
pub(in crate::native_artifact) fn emit(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    plan: &NativeScalarAggregates,
    site: usize,
    error: Block,
) -> Result<bool, NativeCompileError> {
    let Some(action) = plan.action(site) else {
        return Ok(false);
    };
    let (target, values) = match action {
        Action::Pack { output, words } => {
            let values = words
                .iter()
                .map(|word| match word {
                    Word::Scalar(source) => load(b, locals, source.get() as usize),
                    Word::Field(source, index) => load(b, locals, field(plan, *source, *index)?),
                    Word::Constant(word) => Ok(b.ins().iconst(types::I64, *word as i64)),
                })
                .collect::<Result<Vec<_>, _>>()?;
            (*output, values)
        }
        Action::Copy { source, output } => {
            let (_, count) = plan
                .slot(*output)
                .ok_or_else(|| backend("aggregate copy target missing"))?;
            // Read all words before any write, including carrier aliases.
            let values = (0..count)
                .map(|i| load(b, locals, field(plan, *source, i)?))
                .collect::<Result<Vec<_>, _>>()?;
            (*output, values)
        }
        Action::Project {
            source,
            offset: start,
            guards,
            output,
        } => {
            for &(index, ordinal) in guards {
                let tag = load(b, locals, field(plan, *source, index)?)?;
                let valid = b.ins().icmp_imm(IntCC::Equal, tag, i64::from(ordinal));
                guard(b, valid, error, super::super::abi::STATUS_ABI);
            }
            let (_, count) = plan
                .slot(*output)
                .ok_or_else(|| backend("aggregate projection target missing"))?;
            let values = (0..count)
                .map(|i| {
                    let index = start
                        .checked_add(i)
                        .ok_or_else(|| backend("aggregate projection offset overflow"))?;
                    load(b, locals, field(plan, *source, index)?)
                })
                .collect::<Result<Vec<_>, _>>()?;
            (*output, values)
        }
        Action::Observe {
            source,
            offset: index,
            ordinal,
            guards,
            output,
        } => {
            for &(index, ordinal) in guards {
                let tag = load(b, locals, field(plan, *source, index)?)?;
                let valid = b.ins().icmp_imm(IntCC::Equal, tag, i64::from(ordinal));
                guard(b, valid, error, super::super::abi::STATUS_ABI);
            }
            let word = load(b, locals, field(plan, *source, *index)?)?;
            let word = if let Some(ordinal) = ordinal {
                let equal = b.ins().icmp_imm(IntCC::Equal, word, i64::from(*ordinal));
                b.ins().uextend(types::I64, equal)
            } else {
                word
            };
            b.ins().store(
                MemFlags::new(),
                word,
                locals,
                offset(output.get() as usize)?,
            );
            return Ok(true);
        }
    };
    for (index, value) in values.into_iter().enumerate() {
        b.ins().store(
            MemFlags::new(),
            value,
            locals,
            offset(field(plan, target, index)?)?,
        );
    }
    Ok(true)
}
