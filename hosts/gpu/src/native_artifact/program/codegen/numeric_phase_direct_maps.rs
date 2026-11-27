//! Compiler-local totality and current-view guards for direct whole maps.
//! No metadata flag grants ownership. The caller supplies an already admitted
//! exclusive destination; the original map recipe and emitter remain current.
use super::*;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Terminator,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term, NativeScalarOp as Op,
};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Axis {
    Outer,
    Inner,
    Capture(u32),
    Constant(u64),
}
struct Read {
    matrix: usize,
    indices: Vec<Axis>,
    same_cell: bool,
}

// These semantic operations have no arithmetic-failure edge for admitted
// typed words. Checked integer arithmetic and narrowing conversions stay on
// the original scratch path. Emission still uses the shared scalar emitter.
fn total(op: Op) -> bool {
    use Op::*;
    matches!(
        op,
        U64BitAnd
            | U64BitOr
            | U64BitXor
            | U64BitNot
            | U64ShlWrap
            | U64ShrWrap
            | U64Clz
            | U64Ctz
            | U64WrappingAdd
            | U64WrappingSub
            | U64WrappingMul
            | U64Eq
            | U64Ne
            | U64Lt
            | U64Le
            | U64Gt
            | U64Ge
            | U8Eq
            | U8Ne
            | U8Lt
            | U8Le
            | U8Gt
            | U8Ge
            | U16Eq
            | U16Ne
            | U16Lt
            | U16Le
            | U16Gt
            | U16Ge
            | U32Eq
            | U32Ne
            | U32Lt
            | U32Le
            | U32Gt
            | U32Ge
            | I64Eq
            | I64Ne
            | I64Lt
            | I64Le
            | I64Gt
            | I64Ge
            | BoolAnd
            | BoolOr
            | BoolXor
            | BoolNot
            | BoolEq
            | BoolNe
            | F64Add
            | F64Sub
            | F64Mul
            | F64Div
            | F64Neg
            | F64Sqrt
            | F64Eq
            | F64Ne
            | F64Lt
            | F64Le
            | F64Gt
            | F64Ge
            | U64ToF64
            | I64ToF64
            | U8ToU16
            | U8ToU32
            | U8ToU64
            | U16ToU32
            | U16ToU64
            | U32ToU64
    )
}
fn root(plan: &Plan, mut value: u32) -> Option<u32> {
    let facts = plan.facts();
    let mut seen = BTreeSet::new();
    while seen.len() < 64 && seen.insert(value) {
        let definitions = facts.definition_sites(value).ok()?;
        let [definition] = definitions else {
            return Some(value);
        };
        let Some(site) = definition.site() else {
            return Some(value);
        };
        match plan.blocks().get(site as usize)?.action() {
            Action::Scalar(Scalar::Copy { output, .. }) if output.get() == value => {
                let Some(source) = facts.unique_identity_copy(*output) else {
                    return Some(value);
                };
                value = source.get()
            }
            _ => return Some(value),
        }
    }
    None
}
fn axis(plan: &Plan, source: &MatrixMap, value: u32) -> Option<Axis> {
    let value = root(plan, value)?;
    if value == source.region.element {
        return Some(Axis::Outer);
    }
    if source
        .region
        .rectangle
        .as_ref()
        .is_some_and(|r| r.inner_element == value)
    {
        return Some(Axis::Inner);
    }
    if source.region.captures.contains(&value) {
        return Some(Axis::Capture(value));
    }
    let entry = plan
        .constants()
        .iter()
        .find(|constant| constant.value().get() == value)?;
    match entry.constant().data() {
        mech_core::ValueData::U64(n) => Some(Axis::Constant(*n)),
        _ => None,
    }
}
// Reconstruct current Bool alternatives for this acyclic element body. All
// facts start fresh per element; source predicates never carry truth across a
// traversal. Reuse the engine's exact branch polarity and identity-copy facts.
fn reachable_body(plan: &Plan, source: &MatrixMap) -> Option<BTreeSet<u32>> {
    use mech_engine::native_artifact::NativeScalarKind;
    type State = BTreeMap<u32, u8>;
    let region = &source.region;
    let first = *region.body.first()?;
    let facts = plan.facts();
    let order = region
        .body
        .iter()
        .enumerate()
        .map(|(at, &site)| (site, at))
        .collect::<BTreeMap<_, _>>();
    let mut incoming = BTreeMap::from([(first, State::new())]);
    let mut reached = BTreeSet::new();
    let mask = |state: &State, value: u32| -> Option<u8> {
        if plan.values().get(value as usize)?.scalar_kind() != Some(NativeScalarKind::Bool) {
            return None;
        }
        let value = root(plan, value)?;
        let literal = plan.constants().iter().find_map(|c| {
            if c.value().get() == value {
                if let mech_core::ValueData::Bool(value) = c.constant().data() {
                    return Some(if *value { 2 } else { 1 });
                }
            }
            None
        });
        Some(literal.unwrap_or(3) & state.get(&value).copied().unwrap_or(3))
    };
    for &site in &region.body {
        let Some(mut state) = incoming.remove(&site) else {
            continue;
        };
        reached.insert(site);
        let block = &plan.blocks()[site as usize];
        match block.action() {
            Action::Scalar(Scalar::Copy { source, output })
                if plan.values()[output.get() as usize].scalar_kind()
                    == Some(NativeScalarKind::Bool) =>
            {
                let value = mask(&state, source.get())?;
                let key = root(plan, output.get())?;
                if value == 3 {
                    state.remove(&key);
                } else {
                    state.insert(key, value);
                }
            }
            Action::Scalar(Scalar::Primitive { op, output, .. })
                if op.signature().1 == NativeScalarKind::Bool =>
            {
                state.remove(&root(plan, output.get())?);
            }
            Action::LibraryCall { output, .. }
                if plan.values()[output.get() as usize].scalar_kind()
                    == Some(NativeScalarKind::Bool) =>
            {
                state.remove(&root(plan, output.get())?);
            }
            _ => {}
        }
        let mut edges = Vec::new();
        let predicates = match block.terminator() {
            Terminator::Scalar(Term::Branch { condition, .. }) => vec![condition],
            Terminator::Scalar(Term::LiteralBranch { left, right, .. }) => vec![left, right],
            _ => Vec::new(),
        };
        let boolean = predicates.into_iter().find_map(|predicate| {
            facts
                .boolean_branch_edges(site, predicate)
                .map(|edges| (predicate, edges))
        });
        if let Some((predicate, (no, yes))) = boolean {
            let current = mask(&state, predicate.get())?;
            let key = root(plan, predicate.get())?;
            for (target, value) in [(no, 1), (yes, 2)] {
                if current & value != 0 {
                    let mut next = state.clone();
                    next.insert(key, value);
                    edges.push((target, next));
                }
            }
        } else {
            match block.terminator() {
                Terminator::Scalar(Term::Jump(to)) => edges.push((to, state)),
                Terminator::Scalar(
                    Term::Branch { yes, no, .. } | Term::LiteralBranch { yes, no, .. },
                ) => {
                    edges.push((yes, state.clone()));
                    edges.push((no, state));
                }
                _ => {}
            }
        }
        for (target, next) in edges {
            let Some(&position) = order.get(&target) else {
                continue;
            }; // the independently checked source latch
            if position <= order[&site] {
                return None;
            }
            if let Some(previous) = incoming.get_mut(&target) {
                let keys = previous
                    .keys()
                    .chain(next.keys())
                    .copied()
                    .collect::<BTreeSet<_>>();
                for key in keys {
                    let joined = previous.get(&key).copied().unwrap_or(3)
                        | next.get(&key).copied().unwrap_or(3);
                    if joined == 3 {
                        previous.remove(&key);
                    } else {
                        previous.insert(key, joined);
                    }
                }
            } else {
                incoming.insert(target, next);
            }
        }
    }
    Some(reached)
}

fn derive(plan: &Plan, source: &MatrixMap) -> Option<Vec<Read>> {
    let region = &source.region;
    if !region.range
        || region.reduction.is_some()
        || !region.nested.is_empty()
        || source.count == 0
        || source.elements == 0
    {
        return None;
    }
    let reachable = reachable_body(plan, source)?;
    for &site in &region.body {
        if !reachable.contains(&site) {
            continue;
        }
        let block = plan.blocks().get(site as usize)?;
        if !region.reads.iter().any(|read| read.site == site) {
            match block.action() {
                Action::Scalar(Scalar::None | Scalar::Copy { .. })
                | Action::CollectionPush { .. }
                | Action::CollectionElement { .. } => {}
                Action::Scalar(Scalar::Primitive { op, .. }) if total(*op) => {}
                _ => return None,
            }
        }
        if !matches!(
            block.terminator(),
            Terminator::Scalar(Term::Jump(_) | Term::Branch { .. } | Term::LiteralBranch { .. })
        ) {
            return None;
        }
    }
    region
        .reads
        .iter()
        .map(|read| {
            let indices = read
                .indices
                .iter()
                .map(|&value| axis(plan, source, value))
                .collect::<Option<Vec<_>>>()?;
            let same_cell = match (region.rectangle.as_ref(), &indices[..]) {
                (Some(rectangle), [Axis::Inner, Axis::Outer]) => {
                    rectangle.start == 1 && source.extents == [rectangle.rows, source.count]
                }
                (None, [Axis::Outer]) => {
                    source.elements == source.count
                        && (source.extents[0] == 1 || source.extents[1] == 1)
                }
                (None, [Axis::Outer, Axis::Constant(1)]) => source.extents == [source.count, 1],
                (None, [Axis::Constant(1), Axis::Outer]) => source.extents == [1, source.count],
                _ => false,
            };
            Some(Read {
                matrix: read.matrix,
                indices,
                same_cell,
            })
        })
        .collect()
}
fn both(b: &mut FunctionBuilder<'_>, good: Value, next: Value) -> Value {
    b.ins().band(good, next)
}
fn bound(b: &mut FunctionBuilder<'_>, good: Value, lo: Value, hi: Value, extent: Value) -> Value {
    let nonzero = b.ins().icmp_imm(IntCC::NotEqual, lo, 0);
    let bounded = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, hi, extent);
    let valid = both(b, nonzero, bounded);
    both(b, good, valid)
}
fn limits(
    b: &mut FunctionBuilder<'_>,
    axis: Axis,
    outer: (Value, Value),
    inner: Option<(u64, u64)>,
    captures: &BTreeMap<u32, Value>,
) -> Option<(Value, Value)> {
    match axis {
        Axis::Outer => Some(outer),
        Axis::Inner => {
            let (lo, hi) = inner?;
            Some((
                b.ins().iconst(types::I64, lo as i64),
                b.ins().iconst(types::I64, hi as i64),
            ))
        }
        Axis::Capture(value) => {
            let value = *captures.get(&value)?;
            Some((value, value))
        }
        Axis::Constant(value) => {
            let value = b.ins().iconst(types::I64, value as i64);
            Some((value, value))
        }
    }
}
pub(super) fn guard(
    b: &mut FunctionBuilder<'_>,
    source: &MatrixMap,
    plan: &Plan,
    base: Value,
    first: Value,
    matrices: &[[Value; 4]],
    captures: &BTreeMap<u32, Value>,
) -> Option<Value> {
    let reads = derive(plan, source)?;
    if matrices.len() != source.region.matrices.len() {
        return None;
    }
    let inner = if let Some(rectangle) = &source.region.rectangle {
        Some((
            rectangle.start,
            rectangle
                .start
                .checked_add(rectangle.rows.checked_sub(1)?)?,
        ))
    } else {
        None
    };
    let span = b.ins().iconst(types::I64, (source.count - 1) as i64);
    let (last, overflow) = b.ins().uadd_overflow(first, span);
    let mut allowed = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
    let out_bytes = source
        .elements
        .checked_mul(u64::from(source.region.output_width))?;
    let out_bytes = b.ins().iconst(types::I64, out_bytes as i64);
    let (out_end, overflow) = b.ins().uadd_overflow(base, out_bytes);
    let valid = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
    allowed = both(b, allowed, valid);
    for read in reads {
        let [pointer, cells, rows, columns] = matrices[read.matrix];
        let axes = read
            .indices
            .iter()
            .map(|&axis| limits(b, axis, (first, last), inner, captures))
            .collect::<Option<Vec<_>>>()?;
        match &axes[..] {
            [(lo, hi)] => allowed = bound(b, allowed, *lo, *hi, cells),
            [(rlo, rhi), (clo, chi)] => {
                allowed = bound(b, allowed, *rlo, *rhi, rows);
                allowed = bound(b, allowed, *clo, *chi, columns);
            }
            _ => return None,
        }
        let width = source.region.matrices[read.matrix].width;
        let width_value = b.ins().iconst(types::I64, i64::from(width));
        let (bytes, overflow) = b.ins().umul_overflow(cells, width_value);
        let valid = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
        allowed = both(b, allowed, valid);
        let (end, overflow) = b.ins().uadd_overflow(pointer, bytes);
        let valid = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
        allowed = both(b, allowed, valid);
        let before = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, base);
        let after = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, out_end, pointer);
        let mut aliases = b.ins().bor(before, after);
        if read.same_cell && width == source.region.output_width {
            let same = b.ins().icmp(IntCC::Equal, pointer, base);
            let row = b
                .ins()
                .icmp_imm(IntCC::Equal, rows, source.extents[0] as i64);
            let column = b
                .ins()
                .icmp_imm(IntCC::Equal, columns, source.extents[1] as i64);
            let start = b.ins().icmp_imm(IntCC::Equal, first, 1);
            let geometry = both(b, row, column);
            let geometry = both(b, geometry, start);
            let same = both(b, same, geometry);
            aliases = b.ins().bor(aliases, same);
        }
        allowed = both(b, allowed, aliases);
    }
    Some(allowed)
}
