//! Source-only masks for same-cell identity alternatives in numeric maps.
//! These facts grant no loan, mutable owner, admission, or publication authority.
use super::{matrix_maps::Replacement, reductions::constant};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
    NativeIterationTerminator as Term, NativeMatchAction as Scalar,
    NativeMatchTerminator as Branch, NativeProgramArtifactPlan as Plan, NativeScalarKind,
    NativeScalarOp as Op,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum Axis {
    Row,
    Column,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum Comparison {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
impl Comparison {
    pub fn relation_bits(self) -> u8 {
        match self {
            Self::Lt => 1,
            Self::Eq => 2,
            Self::Le => 3,
            Self::Gt => 4,
            Self::Ne => 5,
            Self::Ge => 6,
        }
    }
    pub fn from_relation_bits(bits: u8) -> Option<Self> {
        Some(match bits {
            1 => Self::Lt,
            2 => Self::Eq,
            3 => Self::Le,
            4 => Self::Gt,
            5 => Self::Ne,
            6 => Self::Ge,
            _ => return None,
        })
    }
    pub fn inverted(self) -> Self {
        match self {
            Self::Eq => Self::Ne,
            Self::Ne => Self::Eq,
            Self::Lt => Self::Ge,
            Self::Le => Self::Gt,
            Self::Gt => Self::Le,
            Self::Ge => Self::Lt,
        }
    }
    fn reversed(self) -> Self {
        match self {
            Self::Lt => Self::Gt,
            Self::Le => Self::Ge,
            Self::Gt => Self::Lt,
            Self::Ge => Self::Le,
            x => x,
        }
    }
    fn from_op(op: Op) -> Option<Self> {
        Some(match op {
            Op::U64Eq => Self::Eq,
            Op::U64Ne => Self::Ne,
            Op::U64Lt => Self::Lt,
            Op::U64Le => Self::Le,
            Op::U64Gt => Self::Gt,
            Op::U64Ge => Self::Ge,
            _ => return None,
        })
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Predicate {
    axis: Axis,
    bound: u32,
    comparison: Comparison,
}
impl Predicate {
    fn inverted(self) -> Self {
        Self {
            comparison: self.comparison.inverted(),
            ..self
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Tag {
    Axis(Axis),
    Bound(u32),
    Predicate(Predicate),
    Bool(bool),
    Cell(usize),
}
type Tags = BTreeMap<u32, Tag>;
#[path = "mask_domains.rs"]
pub(in crate::native_artifact) mod domains;

/// A pure description rederived from the actual selected CFG. It is absent
/// from every image/ABI; callers cannot supply a wire mask or current sample.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Mask {
    predicate: Predicate,
    selector: u32,
    active: u32,
    rows: u64,
    columns: u64,
    rectangular: bool,
    base: usize,
}
impl Mask {
    pub fn axis(&self) -> Axis {
        self.predicate.axis
    }
    pub fn comparison(&self) -> Comparison {
        self.predicate.comparison
    }
    pub fn bound(&self) -> u32 {
        self.predicate.bound
    }
    pub fn selector(&self) -> u32 {
        self.selector
    }
    pub fn active(&self) -> u32 {
        self.active
    }
    pub fn rows(&self) -> u64 {
        self.rows
    }
    pub fn columns(&self) -> u64 {
        self.columns
    }
    pub fn rectangular(&self) -> bool {
        self.rectangular
    }
    pub fn base(&self) -> usize {
        self.base
    }

    pub fn derive(
        plan: &Plan,
        proof: &crate::native_artifact::program::prepare::readonly_numeric::Proof,
        replacement: &Replacement,
    ) -> Option<Self> {
        let ([rows, columns], rectangular, mut tags) = source_tags(plan, proof, replacement)?;
        let region = &replacement.source.region;
        let mut at = *region.body.first()?;
        let mut visited = BTreeSet::new();
        while region.body.contains(&at) && visited.insert(at) {
            // Every skipped prefix action must be total, scalar and effect-free.
            abstract_scalar(plan, at, &mut tags, None)?;
            match plan.blocks().get(at as usize)?.terminator() {
                Term::Scalar(Branch::Jump(next)) => at = next,
                Term::Scalar(branch) => {
                    let (predicate, yes, no) = branch_predicate(branch, &tags)?;
                    if !region.captures.contains(&predicate.bound) {
                        return None;
                    }
                    for (identity, truth) in [(no, false), (yes, true)] {
                        let mut state = tags.clone();
                        apply_assumption(&mut state, predicate, truth);
                        if let Some(base) = identity_tail(
                            plan,
                            replacement,
                            identity,
                            state,
                            (predicate, truth),
                            [rows, columns],
                        ) {
                            return Some(Self {
                                predicate: if truth {
                                    predicate.inverted()
                                } else {
                                    predicate
                                },
                                selector: at,
                                active: if truth { no } else { yes },
                                rows,
                                columns,
                                rectangular,
                                base,
                            });
                        }
                    }
                    return None;
                }
                _ => return None,
            }
        }
        None
    }
}
fn source_tags(
    plan: &Plan,
    proof: &crate::native_artifact::program::prepare::readonly_numeric::Proof,
    replacement: &Replacement,
) -> Option<([u64; 2], bool, Tags)> {
    proof.arrays.get(replacement.target)?;
    let source = &replacement.source;
    let region = &source.region;
    let [rows, columns] = source.extents;
    if !region.range
        || region.reduction.is_some()
        || rows == 0
        || columns == 0
        || rows.checked_mul(columns)? != source.elements
        || replacement.arrays.len() != region.matrices.len()
        || constant(plan, source.first) != Some(1)
    {
        return None;
    }
    for (index, matrix) in region.matrices.iter().enumerate() {
        if proof.arrays.get(*replacement.arrays.get(index)?)?.width != matrix.width {
            return None;
        }
    }
    let mut tags = Tags::new();
    for &id in &region.captures {
        if plan.values().get(id as usize)?.scalar_kind() == Some(NativeScalarKind::U64) {
            tags.insert(id, Tag::Bound(id));
        }
    }
    for value in plan.constants() {
        if let mech_core::ValueData::Bool(value_data) = value.constant().data() {
            tags.insert(value.value().get(), Tag::Bool(*value_data));
        }
    }
    let rectangular = if let Some(rectangle) = &region.rectangle {
        let [first, _, inner_rows] = rectangle.fixed()?;
        if first != 1 || inner_rows != rows || source.count != columns || region.transpose.is_some()
        {
            return None;
        }
        tags.insert(rectangle.inner_element, Tag::Axis(Axis::Row));
        tags.insert(region.element, Tag::Axis(Axis::Column));
        true
    } else {
        if source.count != source.elements {
            return None;
        }
        let axis = if columns == 1 {
            Axis::Row
        } else if rows == 1 {
            Axis::Column
        } else {
            return None;
        };
        tags.insert(region.element, Tag::Axis(axis));
        false
    };
    Some(([rows, columns], rectangular, tags))
}
fn normalize(tag: Tag, assumption: Option<(Predicate, bool)>) -> Tag {
    match (tag, assumption) {
        (Tag::Predicate(p), Some((known, truth))) if p == known => Tag::Bool(truth),
        (Tag::Predicate(p), Some((known, truth))) if p == known.inverted() => Tag::Bool(!truth),
        _ => tag,
    }
}
fn apply_assumption(tags: &mut Tags, predicate: Predicate, truth: bool) {
    for tag in tags.values_mut() {
        *tag = normalize(*tag, Some((predicate, truth)));
    }
}
fn abstract_scalar(
    plan: &Plan,
    site: u32,
    tags: &mut Tags,
    assumption: Option<(Predicate, bool)>,
) -> Option<()> {
    match plan.blocks().get(site as usize)?.action() {
        Action::Scalar(Scalar::None) | Action::CollectionElement { .. } => {}
        Action::Scalar(Scalar::Copy { source, output }) => {
            let value = tags.get(&source.get()).copied();
            tags.remove(&output.get());
            if let Some(value) = value {
                tags.insert(output.get(), value);
            }
        }
        Action::Scalar(Scalar::Primitive {
            op,
            operands,
            output,
        }) => {
            let operands = operands
                .iter()
                .map(|v| tags.get(&v.get()).copied())
                .collect::<Option<Vec<_>>>()?;
            let tag = match (*op, operands.as_slice()) {
                (Op::BoolNot, [Tag::Bool(value)]) => Tag::Bool(!value),
                (Op::BoolNot, [Tag::Predicate(p)]) => Tag::Predicate(p.inverted()),
                (Op::BoolEq | Op::BoolNe, [Tag::Bool(a), Tag::Bool(b)]) => {
                    Tag::Bool((a == b) == (*op == Op::BoolEq))
                }
                (
                    Op::BoolEq | Op::BoolNe,
                    [Tag::Predicate(p), Tag::Bool(value)] | [Tag::Bool(value), Tag::Predicate(p)],
                ) => Tag::Predicate(if *value == (*op == Op::BoolEq) {
                    *p
                } else {
                    p.inverted()
                }),
                (op, [Tag::Axis(axis), Tag::Bound(bound)]) => Tag::Predicate(Predicate {
                    axis: *axis,
                    bound: *bound,
                    comparison: Comparison::from_op(op)?,
                }),
                (op, [Tag::Bound(bound), Tag::Axis(axis)]) => Tag::Predicate(Predicate {
                    axis: *axis,
                    bound: *bound,
                    comparison: Comparison::from_op(op)?.reversed(),
                }),
                _ => return None,
            };
            tags.insert(output.get(), normalize(tag, assumption));
        }
        _ => return None,
    };
    Some(())
}
fn branch_predicate(branch: Branch, tags: &Tags) -> Option<(Predicate, u32, u32)> {
    match branch {
        Branch::Branch { condition, yes, no } => {
            let Tag::Predicate(p) = tags.get(&condition.get())? else {
                return None;
            };
            Some((*p, yes, no))
        }
        Branch::LiteralBranch {
            left,
            right,
            yes,
            no,
        } => {
            let (Tag::Predicate(p), Tag::Bool(value)) =
                (tags.get(&left.get())?, tags.get(&right.get())?)
            else {
                let (Tag::Bool(value), Tag::Predicate(p)) =
                    (tags.get(&left.get())?, tags.get(&right.get())?)
                else {
                    return None;
                };
                return Some((if *value { *p } else { p.inverted() }, yes, no));
            };
            Some((if *value { *p } else { p.inverted() }, yes, no))
        }
        _ => None,
    }
}
fn axis_index(plan: &Plan, tags: &Tags, value: u32, axis: Axis, extents: [u64; 2]) -> bool {
    if tags.get(&value) == Some(&Tag::Axis(axis)) {
        return true;
    }
    if extents[usize::from(axis == Axis::Column)] != 1 {
        return false;
    }
    match tags.get(&value) {
        Some(Tag::Bound(original)) => constant(plan, *original) == Some(1),
        Some(Tag::Axis(other)) => extents[usize::from(*other == Axis::Column)] == 1,
        _ => constant(plan, value) == Some(1),
    }
}

fn identity_tail(
    plan: &Plan,
    replacement: &Replacement,
    mut at: u32,
    mut tags: Tags,
    assumption: (Predicate, bool),
    extents: [u64; 2],
) -> Option<usize> {
    let region = &replacement.source.region;
    let iteration = region
        .rectangle
        .as_ref()
        .map_or(region.iteration, |r| r.inner_iteration);
    let Some(control) = plan.iterations().get(iteration as usize) else {
        return None;
    };
    let latch = control.as_ref().latch();
    let mut seen = BTreeSet::new();
    let mut pushed = false;
    let mut base = None;
    loop {
        if at == latch {
            return if pushed { base } else { None };
        }
        if !region.body.contains(&at) || !seen.insert(at) {
            return None;
        }
        let Some(block) = plan.blocks().get(at as usize) else {
            return None;
        };
        match block.action() {
            Action::LibraryCall {
                template,
                operands,
                output,
            } => {
                let Some(read) = region.reads.iter().find(|r| r.site == at) else {
                    return None;
                };
                let Some(matrix) = region.matrices.get(read.matrix) else {
                    return None;
                };
                let array = *replacement.arrays.get(read.matrix)?;
                if base.is_some_and(|base| base != array) || matrix.width != region.output_width {
                    return None;
                }
                base = Some(array);
                if operands.first().map(|v| v.get()) != Some(matrix.value)
                    || output.get() != read.output
                    || operands[1..].iter().map(|v| v.get()).collect::<Vec<_>>() != read.indices
                {
                    return None;
                }
                let same = match (template.primitive(), read.indices.as_slice()) {
                    (Primitive::MatrixCellAccess, [row, column]) => {
                        axis_index(plan, &tags, *row, Axis::Row, extents)
                            && axis_index(plan, &tags, *column, Axis::Column, extents)
                    }
                    (Primitive::MatrixScalarAccess, [index]) if region.rectangle.is_none() => {
                        let axis = if extents[1] == 1 {
                            Axis::Row
                        } else {
                            Axis::Column
                        };
                        axis_index(plan, &tags, *index, axis, extents)
                    }
                    _ => false,
                };
                if !same {
                    return None;
                }
                tags.insert(read.output, Tag::Cell(array));
            }
            Action::CollectionPush { value, .. } => {
                if pushed
                    || value.get() != region.result
                    || tags.get(&value.get()).copied() != base.map(Tag::Cell)
                {
                    return None;
                }
                pushed = true;
            }
            _ => {
                if abstract_scalar(plan, at, &mut tags, Some(assumption)).is_none() {
                    return None;
                }
            }
        }
        match block.terminator() {
            Term::Scalar(Branch::Jump(next)) => at = next,
            Term::Scalar(Branch::Branch { condition, yes, no }) => {
                let Some(Tag::Bool(value)) = tags.get(&condition.get()) else {
                    return None;
                };
                at = if *value { yes } else { no };
            }
            Term::Scalar(Branch::LiteralBranch {
                left,
                right,
                yes,
                no,
            }) => {
                let (Some(Tag::Bool(left)), Some(Tag::Bool(right))) =
                    (tags.get(&left.get()), tags.get(&right.get()))
                else {
                    return None;
                };
                at = if left == right { yes } else { no };
            }
            _ => return None,
        }
    }
}
