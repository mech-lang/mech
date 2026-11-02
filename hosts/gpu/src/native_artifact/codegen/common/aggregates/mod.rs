//! The one Pack/Copy/Project/Observe implementation for memory and SSA homes.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::common::word_access::{LocalWord, MemoryWords, WordAccess};
use crate::native_artifact::compiler::{backend, guard};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, Value, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeAggregateWord as Word, NativeMapOwnerWords, NativeMatchValue,
    NativeScalarAggregateAction as Action, NativeScalarAggregates,
};
use std::collections::BTreeSet;

pub(in crate::native_artifact) fn field(
    layout: &NativeScalarAggregates,
    value: NativeMatchValue,
    index: usize,
) -> Option<usize> {
    let (base, count) = layout.slot(value)?;
    (index < count).then(|| base + index)
}
fn at(
    layout: &NativeScalarAggregates,
    value: NativeMatchValue,
    index: usize,
) -> Result<LocalWord, NativeCompileError> {
    field(layout, value, index)
        .map(LocalWord)
        .ok_or_else(|| backend("aggregate field exceeds native storage"))
}

/// A strategy may consume its current-site proof here. Common emission never
/// accepts an unchecked skip flag or infers eligibility from a report count.
pub(in crate::native_artifact) trait GuardPolicy {
    fn projection<W: WordAccess>(
        &mut self,
        b: &mut FunctionBuilder<'_>,
        words: &mut W,
        word: LocalWord,
        source: NativeMatchValue,
        index: usize,
        ordinal: u32,
        error: Block,
    ) -> Result<(), NativeCompileError>;
}
pub(in crate::native_artifact) struct CheckedGuards;
impl GuardPolicy for CheckedGuards {
    fn projection<W: WordAccess>(
        &mut self,
        b: &mut FunctionBuilder<'_>,
        words: &mut W,
        word: LocalWord,
        _source: NativeMatchValue,
        _index: usize,
        ordinal: u32,
        error: Block,
    ) -> Result<(), NativeCompileError> {
        let tag = words.read(b, word)?;
        let valid = b.ins().icmp_imm(IntCC::Equal, tag, i64::from(ordinal));
        guard(b, valid, error, crate::native_artifact::abi::STATUS_ABI);
        Ok(())
    }
}

pub(in crate::native_artifact) fn emit<W: WordAccess, G: GuardPolicy>(
    b: &mut FunctionBuilder<'_>,
    words: &mut W,
    layout: &NativeScalarAggregates,
    action: &Action,
    error: Block,
    guards: &mut G,
) -> Result<(), NativeCompileError> {
    let (output, values) = match action {
        Action::Pack {
            output,
            words: inputs,
        } => {
            let values = inputs
                .iter()
                .map(|word| match word {
                    Word::Scalar(source) => words.read(b, LocalWord(source.get() as usize)),
                    Word::Field(source, index) => words.read(b, at(layout, *source, *index)?),
                    Word::Constant(word) => Ok(b.ins().iconst(types::I64, *word as i64)),
                })
                .collect::<Result<Vec<_>, _>>()?;
            (*output, values)
        }
        Action::Copy { source, output } => {
            let (_, count) = layout
                .slot(*output)
                .ok_or_else(|| backend("aggregate copy target missing"))?;
            // Every source is read before any overlapping destination is written.
            let values = (0..count)
                .map(|i| words.read(b, at(layout, *source, i)?))
                .collect::<Result<Vec<_>, _>>()?;
            (*output, values)
        }
        Action::Project {
            source,
            offset,
            guards: checks,
            output,
        } => {
            for &(index, ordinal) in checks {
                guards.projection(
                    b,
                    words,
                    at(layout, *source, index)?,
                    *source,
                    index,
                    ordinal,
                    error,
                )?;
            }
            let (_, count) = layout
                .slot(*output)
                .ok_or_else(|| backend("aggregate projection target missing"))?;
            let values = (0..count)
                .map(|i| {
                    let index = offset
                        .checked_add(i)
                        .ok_or_else(|| backend("aggregate projection offset overflow"))?;
                    words.read(b, at(layout, *source, index)?)
                })
                .collect::<Result<Vec<_>, _>>()?;
            (*output, values)
        }
        Action::Observe {
            source,
            offset,
            ordinal,
            guards: checks,
            output,
        } => {
            for &(index, ordinal) in checks {
                guards.projection(
                    b,
                    words,
                    at(layout, *source, index)?,
                    *source,
                    index,
                    ordinal,
                    error,
                )?;
            }
            let value = words.read(b, at(layout, *source, *offset)?)?;
            let value = if let Some(ordinal) = ordinal {
                let equal = b.ins().icmp_imm(IntCC::Equal, value, i64::from(*ordinal));
                b.ins().uextend(types::I64, equal)
            } else {
                value
            };
            words.write(b, LocalWord(output.get() as usize), value)?;
            return Ok(());
        }
    };
    for (index, value) in values.into_iter().enumerate() {
        words.write(b, at(layout, output, index)?, value)?;
    }
    Ok(())
}

pub(in crate::native_artifact) fn emit_memory(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    layout: &NativeScalarAggregates,
    site: usize,
    error: Block,
) -> Result<bool, NativeCompileError> {
    let Some(action) = layout.action(site) else {
        return Ok(false);
    };
    emit(
        b,
        &mut MemoryWords(locals),
        layout,
        action,
        error,
        &mut CheckedGuards,
    )?;
    Ok(true)
}
pub(in crate::native_artifact) fn constants(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    layout: &NativeScalarAggregates,
) -> Result<(), NativeCompileError> {
    let mut words = MemoryWords(locals);
    for (target, values) in layout.constants() {
        for (index, &word) in values.iter().enumerate() {
            let value = b.ins().iconst(types::I64, word as i64);
            words.write(b, at(layout, *target, index)?, value)?;
        }
    }
    Ok(())
}

pub(in crate::native_artifact) fn aggregate_words(
    layout: &NativeScalarAggregates,
    action: &Action,
    words: &mut BTreeSet<usize>,
) -> Option<()> {
    let output = match action {
        Action::Pack {
            output,
            words: inputs,
        } => {
            for input in inputs {
                match input {
                    Word::Scalar(value) => {
                        words.insert(value.get() as usize);
                    }
                    Word::Field(value, index) => {
                        words.insert(field(layout, *value, *index)?);
                    }
                    Word::Constant(_) => {}
                }
            }
            *output
        }
        Action::Copy { source, output } => {
            let (_, count) = layout.slot(*output)?;
            for index in 0..count {
                words.insert(field(layout, *source, index)?);
            }
            *output
        }
        Action::Project {
            source,
            offset,
            guards,
            output,
        } => {
            for &(index, _) in guards {
                words.insert(field(layout, *source, index)?);
            }
            let (_, count) = layout.slot(*output)?;
            for index in 0..count {
                words.insert(field(layout, *source, offset.checked_add(index)?)?);
            }
            *output
        }
        Action::Observe {
            source,
            offset,
            guards,
            output,
            ..
        } => {
            for &(index, _) in guards {
                words.insert(field(layout, *source, index)?);
            }
            words.insert(field(layout, *source, *offset)?);
            words.insert(output.get() as usize);
            return Some(());
        }
    };
    let (base, count) = layout.slot(output)?;
    words.extend(base..base + count);
    Some(())
}

pub(in crate::native_artifact) fn aggregate_at<'a>(
    owner: &'a NativeMapOwnerWords,
    ordinary: &'a NativeScalarAggregates,
    owner_index: usize,
    site: usize,
) -> Option<(&'a NativeScalarAggregates, &'a Action)> {
    if owner.action_region(site) == Some(owner_index) {
        owner
            .aggregates()
            .action(site)
            .map(|action| (owner.aggregates(), action))
    } else {
        ordinary.action(site).map(|action| (ordinary, action))
    }
}

/// Additional emission proof: every action in the already closed, single-entry
/// Tail has a complete word interpretation. Unsupported actions keep the old
/// generated CFG. The existing owner proof supplies dominance, lifetime and
/// single-backedge invariants; no spelling of a source function is recognized.

#[cfg(test)]
mod map_owner_tests;
