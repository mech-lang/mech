//! Source-only totality and pointwise dependence facts for private map writes.
//! These facts grant no owner, mutable loan, budget credit or publication.
use super::matrix_maps::Replacement;
use mech_core::{DimensionExpr, SchemaBody, ValueData};
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Terminator,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term, NativeProgramArtifactPlan as Plan,
    NativeScalarKind as Kind, NativeScalarOp as Op,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Axis {
    Outer,
    Inner,
}
#[derive(Clone, Copy, Debug)]
struct Span {
    lo: u64,
    hi: u64,
    axis: Option<Axis>,
}
impl Span {
    fn new(lo: u64, hi: u64) -> Self {
        Self { lo, hi, axis: None }
    }
    fn singleton(value: u64) -> Self {
        Self::new(value, value)
    }
    fn union(self, other: Self) -> Self {
        Self {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
            axis: self.axis.filter(|axis| Some(*axis) == other.axis),
        }
    }
}
type Facts = BTreeMap<u32, Span>;
#[path = "total_map_columns.rs"]
mod columns;
/// Source bounds grant no memory authority. Only the maintained map preflight
/// may combine this private witness with actual current views and backing.
pub(in crate::native_artifact) struct ReadBounds {
    sites: BTreeSet<u32>,
}
impl ReadBounds {
    pub(in crate::native_artifact) fn contains(&self, site: u32) -> bool {
        self.sites.contains(&site)
    }
}
/// Every path to these identity sites has a pure row-invariant prefix.
pub(in crate::native_artifact) struct RetainedColumns {
    sites: BTreeSet<u32>,
}
impl RetainedColumns {
    pub(in crate::native_artifact) fn contains(&self, site: u32) -> bool {
        self.sites.contains(&site)
    }
}

pub(in crate::native_artifact) struct Total {
    pub first: u64,
    pub literals: Vec<(u32, u64)>,
    pub matrix_bytes: Vec<u64>,
    pub identity_sites: BTreeSet<u32>,
    pub read_bounds: ReadBounds,
    pub retained_columns: RetainedColumns,
}
fn literal(plan: &Plan, value: u32) -> Option<u64> {
    plan.constants()
        .iter()
        .find(|c| c.value().get() == value)
        .and_then(|c| match c.constant().data() {
            ValueData::U64(n) => Some(*n),
            ValueData::Bool(n) => Some(u64::from(*n)),
            _ => None,
        })
}
fn shape(plan: &Plan, value: u32) -> Option<[u64; 2]> {
    let SchemaBody::Matrix { dimensions, .. } =
        plan.values().get(value as usize)?.definition().body()
    else {
        return None;
    };
    let [
        DimensionExpr::Constant(rows),
        DimensionExpr::Constant(columns),
    ] = &dimensions[..]
    else {
        return None;
    };
    (*rows > 0 && *columns > 0).then_some([*rows, *columns])
}
fn merge(states: &mut BTreeMap<u32, Facts>, target: u32, incoming: Facts) {
    if let Some(current) = states.get_mut(&target) {
        current.retain(|value, range| {
            if let Some(other) = incoming.get(value) {
                *range = range.union(*other);
                true
            } else {
                false
            }
        });
    } else {
        states.insert(target, incoming);
    }
}
fn set(facts: &mut Facts, value: u32, range: Option<Span>) {
    if let Some(range) = range {
        facts.insert(value, range);
    } else {
        facts.remove(&value);
    }
}
fn primitive(
    op: Op,
    operands: &[mech_engine::native_artifact::NativeMatchValue],
    facts: &Facts,
) -> Option<Option<Span>> {
    use Op::*;
    let arg = |at: usize| facts.get(&operands.get(at)?.get()).copied();
    let boolean = || Some(Some(Span::new(0, 1)));
    match op {
        F64Add | F64Sub | F64Mul | F64Div | F64Neg | F64Sqrt | U64ToF64 | I64ToF64 => Some(None),
        U8Eq | U8Ne | U8Lt | U8Le | U8Gt | U8Ge | U16Eq | U16Ne | U16Lt | U16Le | U16Gt | U16Ge
        | U32Eq | U32Ne | U32Lt | U32Le | U32Gt | U32Ge | U64Eq | U64Ne | U64Lt | U64Le | U64Gt
        | U64Ge | I64Eq | I64Ne | I64Lt | I64Le | I64Gt | I64Ge | F64Eq | F64Ne | F64Lt | F64Le
        | F64Gt | F64Ge | BoolEq | BoolNe => boolean(),
        BoolAnd | BoolOr | BoolXor | BoolNot => operands
            .iter()
            .all(|v| facts.get(&v.get()).is_some_and(|r| r.hi <= 1))
            .then_some(Some(Span::new(0, 1))),
        U64BitAnd | U64BitOr | U64BitXor | U64BitNot | U64ShlWrap | U64ShrWrap | U64WrappingAdd
        | U64WrappingSub | U64WrappingMul => Some(Some(Span::new(0, u64::MAX))),
        U64Clz | U64Ctz => Some(Some(Span::new(0, 64))),
        U8Add | U16Add | U32Add | U64Add | U16Sub | U32Sub | U64Sub | U16Mul | U32Mul | U64Mul
        | U16Div | U32Div | U64Div | U16Rem | U32Rem | U64Rem => {
            let a = arg(0)?;
            let c = arg(1)?;
            let maximum = op.signature().1.unsigned_maximum()?;
            if a.hi > maximum || c.hi > maximum {
                return None;
            }
            let mut result = match op {
                U8Add | U16Add | U32Add | U64Add => {
                    Span::new(a.lo.checked_add(c.lo)?, a.hi.checked_add(c.hi)?)
                }
                U16Sub | U32Sub | U64Sub => {
                    Span::new(a.lo.checked_sub(c.hi)?, a.hi.checked_sub(c.lo)?)
                }
                U16Mul | U32Mul | U64Mul => {
                    Span::new(a.lo.checked_mul(c.lo)?, a.hi.checked_mul(c.hi)?)
                }
                U16Div | U32Div | U64Div if c.lo > 0 => Span::new(a.lo / c.hi, a.hi / c.lo),
                U16Rem | U32Rem | U64Rem if c.lo > 0 => Span::new(0, a.hi.min(c.hi - 1)),
                _ => return None,
            };
            if result.hi > maximum {
                return None;
            }
            if matches!(op, U64Add | U64Sub) && c.lo == 0 && c.hi == 0 {
                result.axis = a.axis;
            }
            Some(Some(result))
        }
        _ => None,
    }
}
fn pointwise(indices: &[Span], dimensions: [u64; 2], replacement: &Replacement) -> bool {
    if dimensions != replacement.source.extents {
        return false;
    }
    match (replacement.source.region.rectangle.as_ref(), indices) {
        (Some(_), [row, column]) => {
            row.axis == Some(Axis::Inner) && column.axis == Some(Axis::Outer)
        }
        (None, [linear]) => linear.axis == Some(Axis::Outer),
        (None, [row, column]) if dimensions[0] == 1 => {
            row.lo == 1 && row.hi == 1 && column.axis == Some(Axis::Outer)
        }
        (None, [row, column]) if dimensions[1] == 1 => {
            column.lo == 1 && column.hi == 1 && row.axis == Some(Axis::Outer)
        }
        _ => false,
    }
}
fn identity_tail(plan: &Plan, start: u32, output: u32, latch: u32) -> bool {
    let mut at = start;
    let mut value = output;
    let mut pushed = false;
    let mut seen = BTreeSet::new();
    loop {
        if !seen.insert(at) || seen.len() > 256 {
            return false;
        }
        let Terminator::Scalar(Term::Jump(next)) = plan.blocks()[at as usize].terminator() else {
            return false;
        };
        if next == latch {
            return pushed;
        }
        at = next;
        match plan.blocks()[at as usize].action() {
            Action::Scalar(Scalar::None) | Action::CollectionElement { .. } => {}
            Action::Scalar(Scalar::Copy { source, output }) if !pushed && source.get() == value => {
                value = output.get()
            }
            Action::CollectionPush { value: output, .. } if !pushed && output.get() == value => {
                pushed = true
            }
            _ => return false,
        }
    }
}
// There is one output store on every complete source path, and no fallible
// action or branch may follow it. Earlier cells can change only after every
// operation for their own cell is finished.
fn pushes_once(plan: &Plan, body: &[u32], latch: u32) -> bool {
    let mut states = BTreeMap::from([(body[0], BTreeSet::from([false]))]);
    let sites: BTreeSet<_> = body.iter().copied().collect();
    for &site in body {
        let Some(incoming) = states.remove(&site) else {
            continue;
        };
        for mut pushed in incoming {
            let action = plan.blocks()[site as usize].action();
            if pushed
                && !matches!(
                    action,
                    Action::Scalar(Scalar::None) | Action::CollectionElement { .. }
                )
            {
                return false;
            }
            if matches!(action, Action::CollectionPush { .. }) {
                pushed = true;
            }
            let targets = match plan.blocks()[site as usize].terminator() {
                Terminator::Scalar(Term::Jump(to)) => vec![to],
                Terminator::Scalar(
                    Term::Branch { yes, no, .. } | Term::LiteralBranch { yes, no, .. },
                ) if !pushed => vec![yes, no],
                Terminator::Scalar(Term::Reject | Term::IntegrityFailure) if !pushed => vec![],
                _ => return false,
            };
            for to in targets {
                if to == latch {
                    if !pushed {
                        return false;
                    }
                } else if sites.contains(&to) {
                    states.entry(to).or_default().insert(pushed);
                } else {
                    return false;
                }
            }
        }
    }
    states.is_empty()
}
/// Derive totality for every reachable acyclic source path, including the
/// otherwise impossible Bool-pattern rejection arms. Unknown facts decline.
pub(in crate::native_artifact) fn derive(plan: &Plan, replacement: &Replacement) -> Option<Total> {
    let source = &replacement.source;
    let region = &source.region;
    if !region.range
        || region.reduction.is_some()
        || !region.nested.is_empty()
        || source.count == 0
        || region.body.is_empty()
        || replacement.arrays.len() != region.matrices.len()
    {
        return None;
    }
    if plan.values()[source.first as usize].scalar_kind() != Some(Kind::U64)
        || plan.values()[source.last as usize].scalar_kind() != Some(Kind::U64)
    {
        return None;
    }
    let first = literal(plan, source.first)?;
    let bound = literal(plan, source.last)?;
    let expected_count = bound
        .checked_sub(first)?
        .checked_add(u64::from(source.inclusive))?;
    let rows = region.rectangle.as_ref().map_or(1, |r| r.rows);
    let expected_extents = if region.rectangle.is_some() || region.transpose.is_none() {
        [rows, expected_count]
    } else {
        [expected_count, 1]
    };
    if source.count != expected_count
        || source.elements != rows.checked_mul(expected_count)?
        || source.extents != expected_extents
    {
        return None;
    }
    let last = first.checked_add(source.count - 1)?;
    let mut facts = Facts::new();
    let mut literals = Vec::new();
    for &capture in &region.captures {
        if let Some(value) = literal(plan, capture) {
            literals.push((capture, value));
            facts.insert(capture, Span::singleton(value));
        } else if plan.values()[capture as usize]
            .scalar_kind()
            .is_some_and(|k| k.unsigned_maximum().is_some())
        {
            facts.insert(capture, Span::new(0, u64::MAX));
        }
    }
    facts.insert(
        region.element,
        Span {
            lo: first,
            hi: last,
            axis: (first == 1).then_some(Axis::Outer),
        },
    );
    if let Some(rectangle) = &region.rectangle {
        facts.insert(
            rectangle.inner_element,
            Span {
                lo: rectangle.start,
                hi: rectangle
                    .start
                    .checked_add(rectangle.rows.checked_sub(1)?)?,
                axis: (rectangle.start == 1).then_some(Axis::Inner),
            },
        );
    }
    let shapes = region
        .matrices
        .iter()
        .map(|matrix| shape(plan, matrix.value))
        .collect::<Option<Vec<_>>>()?;
    let matrix_bytes = shapes
        .iter()
        .zip(&region.matrices)
        .map(|([rows, columns], matrix)| {
            rows.checked_mul(*columns)?
                .checked_mul(u64::from(matrix.width))
        })
        .collect::<Option<Vec<_>>>()?;
    if matrix_bytes.iter().any(|&bytes| bytes > isize::MAX as u64) {
        return None;
    }
    let latch = plan.iterations()[region
        .rectangle
        .as_ref()
        .map_or(region.iteration, |r| r.inner_iteration)
        as usize]
        .as_ref()
        .latch();
    if !pushes_once(plan, &region.body, latch) {
        return None;
    }
    let mut states = BTreeMap::from([(region.body[0], facts)]);
    let mut identity_sites = BTreeSet::new();
    let mut bounded_reads = BTreeSet::new();
    let mut completed = false;
    let body: BTreeSet<_> = region.body.iter().copied().collect();
    for &site in &region.body {
        let Some(mut facts) = states.remove(&site) else {
            continue;
        };
        if let Some(read) = region.reads.iter().find(|r| r.site == site) {
            let dimensions = shapes[read.matrix];
            let indices = read
                .indices
                .iter()
                .map(|v| facts.get(v).copied())
                .collect::<Option<Vec<_>>>()?;
            let bounds = if indices.len() == 1 {
                vec![dimensions[0].checked_mul(dimensions[1])?]
            } else if indices.len() == 2 {
                dimensions.to_vec()
            } else {
                return None;
            };
            if indices
                .iter()
                .zip(bounds)
                .any(|(r, bound)| r.lo < 1 || r.hi > bound)
            {
                return None;
            }
            bounded_reads.insert(site);
            if replacement.arrays[read.matrix] == replacement.target {
                if !pointwise(&indices, dimensions, replacement) {
                    return None;
                }
                if identity_tail(plan, site, read.output, latch) {
                    identity_sites.insert(site);
                }
            }
            let range = plan.values()[read.output as usize]
                .scalar_kind()
                .and_then(|k| k.unsigned_maximum())
                .map(|max| Span::new(0, max));
            set(&mut facts, read.output, range);
        } else {
            match plan.blocks()[site as usize].action() {
                Action::Scalar(Scalar::Copy { source, output }) => {
                    let range = facts.get(&source.get()).copied();
                    set(&mut facts, output.get(), range);
                }
                Action::Scalar(Scalar::Primitive {
                    op,
                    operands,
                    output,
                }) => {
                    let range = primitive(*op, operands, &facts)?;
                    set(&mut facts, output.get(), range);
                }
                Action::CollectionPush { value, .. } => {
                    if plan.values()[value.get() as usize].scalar_kind() == Some(Kind::Bool)
                        && !facts.get(&value.get()).is_some_and(|r| r.hi <= 1)
                    {
                        return None;
                    }
                }
                Action::Scalar(Scalar::None) | Action::CollectionElement { .. } => {}
                _ => return None,
            }
        }
        let mut edges = Vec::new();
        match plan.blocks()[site as usize].terminator() {
            Terminator::Scalar(Term::Jump(to)) => edges.push((to, facts)),
            Terminator::Scalar(Term::Branch { condition, yes, no }) => {
                let range = facts.get(&condition.get());
                if !range.is_some_and(|r| r.hi == 0) {
                    edges.push((yes, facts.clone()));
                }
                if !range.is_some_and(|r| r.lo > 0) {
                    edges.push((no, facts));
                }
            }
            Terminator::Scalar(Term::LiteralBranch {
                left,
                right,
                yes,
                no,
            }) => {
                let kind = plan.values()[left.get() as usize].scalar_kind()?;
                let pair = (matches!(
                    kind,
                    Kind::Bool | Kind::U8 | Kind::U16 | Kind::U32 | Kind::U64
                ))
                .then(|| {
                    facts
                        .get(&right.get())
                        .filter(|r| r.lo == r.hi)
                        .map(|r| (left.get(), r.lo))
                        .or_else(|| {
                            facts
                                .get(&left.get())
                                .filter(|r| r.lo == r.hi)
                                .map(|r| (right.get(), r.lo))
                        })
                })
                .flatten();
                if let Some((value, literal)) = pair {
                    let range = facts.get(&value).copied().unwrap_or(Span::new(0, u64::MAX));
                    if range.lo <= literal && literal <= range.hi {
                        let mut branch = facts.clone();
                        branch.insert(value, Span::singleton(literal));
                        edges.push((yes, branch));
                    }
                    if range.lo != literal || range.hi != literal {
                        let mut branch = facts;
                        let mut refined = range;
                        if refined.lo == literal {
                            refined.lo = literal.checked_add(1)?;
                        }
                        if refined.hi == literal {
                            refined.hi = literal.checked_sub(1)?;
                        }
                        branch.insert(value, refined);
                        edges.push((no, branch));
                    }
                } else {
                    edges.push((yes, facts.clone()));
                    edges.push((no, facts));
                }
            }
            _ => return None,
        }
        for (target, facts) in edges {
            if target == latch {
                completed = true;
            } else if body.contains(&target) {
                merge(&mut states, target, facts);
            } else {
                return None;
            }
        }
    }
    let retained_columns = RetainedColumns {
        sites: columns::derive(plan, region, &identity_sites),
    };
    (completed && states.is_empty()).then_some(Total {
        first,
        literals,
        matrix_bytes,
        identity_sites,
        read_bounds: ReadBounds {
            sites: bounded_reads,
        },
        retained_columns,
    })
}
