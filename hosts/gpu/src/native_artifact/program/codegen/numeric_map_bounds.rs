//! One current readonly map range guard, never a serialized bounds certificate.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::compiler::backend;
use crate::native_artifact::program::prepare::numeric_maps::Region;
use cranelift_codegen::ir::{InstBuilder, Value, condcodes::IntCC};
use cranelift_frontend::FunctionBuilder;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum Index {
    Outer,
    Inner,
    Capture(u32),
}
fn classify(region: &Region, captures: &BTreeMap<u32, Value>, value: u32) -> Option<Index> {
    if region.range && value == region.element {
        Some(Index::Outer)
    } else if region.rectangle.as_ref().is_some_and(|r| {
        r.inner_element == value && r.rows > 0 && r.start.checked_add(r.rows - 1).is_some()
    }) {
        Some(Index::Inner)
    } else if captures.contains_key(&value) {
        Some(Index::Capture(value))
    } else {
        None
    }
}

/// Pure predicates choose a bounds-proved body or the unchanged checked body.
/// A false hypothesis never triggers early replay or evaluates an inactive read.
/// No source visit, output or logical debit is settled here.
/// The caller has checked current matrix geometry/span, count multiplication,
/// range overflow and the synchronous frame epoch. Bodies cannot mutate views.
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    region: &Region,
    matrices: &[(Value, Value, Value, Value)],
    captures: &BTreeMap<u32, Value>,
    start: Value,
    source_count: Value,
    count: Value,
) -> Result<(Value, BTreeSet<u32>), NativeCompileError> {
    let mut selected = BTreeSet::new();
    let mut all_valid = b.ins().iconst(cranelift_codegen::ir::types::I8, 1);
    let mut guarded = BTreeSet::new();
    let mut intervals = BTreeMap::new();
    let empty = b.ins().icmp_imm(IntCC::Equal, count, 0);
    for read in &region.reads {
        let Some(indices) = read
            .indices
            .iter()
            .map(|&v| classify(region, captures, v))
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        if !(1..=2).contains(&indices.len()) {
            continue;
        }
        let &(_, cells, rows, columns) = matrices
            .get(read.matrix)
            .ok_or_else(|| backend("map bounds lost current matrix view"))?;
        for (axis, index) in indices.iter().copied().enumerate() {
            let dimension = if indices.len() == 1 { 0 } else { axis + 1 };
            if !guarded.insert((index, read.matrix, dimension)) {
                continue;
            }
            let (low, high) = *intervals.entry(index).or_insert_with(|| match index {
                Index::Outer => {
                    // The caller already checked overflow for nonempty ranges.
                    // Empty maps never consume the wrapping last-word value.
                    let last = b.ins().iadd_imm(source_count, -1);
                    (start, b.ins().iadd(start, last))
                }
                Index::Inner => {
                    let r = region.rectangle.as_ref().unwrap();
                    (
                        b.ins()
                            .iconst(cranelift_codegen::ir::types::I64, r.start as i64),
                        b.ins()
                            .iconst(cranelift_codegen::ir::types::I64, r.end as i64),
                    )
                }
                Index::Capture(value) => (captures[&value], captures[&value]),
            });
            let limit = match dimension {
                0 => cells,
                1 => rows,
                _ => columns,
            };
            let nonzero = b.ins().icmp_imm(IntCC::NotEqual, low, 0);
            let within = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, high, limit);
            let valid = b.ins().band(nonzero, within);
            let valid = b.ins().bor(empty, valid);
            all_valid = b.ins().band(all_valid, valid);
        }
        selected.insert(read.site);
    }
    Ok((all_valid, selected))
}
