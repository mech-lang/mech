use super::super::word_ranges::{Facts, Range};
use cranelift_codegen::ir::{Block, Function, Type, Value, ValueDef};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug)]
pub(super) enum Word {
    Value(Value, Option<Range>),
    Constant(Type, u64),
}

pub(super) fn owner(f: &Function, value: Value) -> Option<Block> {
    match f.dfg.value_def(f.dfg.resolve_aliases(value)) {
        ValueDef::Param(block, _) => Some(block),
        ValueDef::Result(inst, _) => f.layout.inst_block(inst),
        _ => None,
    }
}

pub(super) fn source(f: &Function, value: Value, known: &BTreeMap<Value, Range>) -> Word {
    Word::Value(value, Facts::selected(f, known).value_range(value))
}

pub(super) fn range(word: Word) -> Option<Range> {
    match word {
        Word::Constant(_, n) => Some(Range(n, n)),
        Word::Value(_, r) => r,
    }
}

pub(super) fn forward(
    f: &Function,
    value: Value,
    words: &BTreeMap<Value, Word>,
    known: &BTreeMap<Value, Range>,
    skipped: &BTreeSet<Block>,
    source_known: &BTreeMap<Value, Range>,
) -> Option<Word> {
    let value = f.dfg.resolve_aliases(value);
    if let Some(&word) = words.get(&value) {
        return Some(word);
    }
    if owner(f, value).is_some_and(|block| skipped.contains(&block)) {
        let ty = f.dfg.value_type(value);
        let r = *known.get(&value)?;
        return (r.0 == r.1 && ty.is_int() && !ty.is_vector()).then_some(Word::Constant(ty, r.0));
    }
    Some(source(f, value, source_known))
}
