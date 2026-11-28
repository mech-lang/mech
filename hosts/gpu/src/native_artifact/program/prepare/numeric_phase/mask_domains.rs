//! Conservative source-CFG identity buckets, factored into Cartesian domains.
//! No source error, mutable owner, resource fee, or wire authority is elided.
use super::{
    Action, Axis, Branch, Comparison, Mask, Plan, Primitive, Replacement, Tag, Tags, Term,
    abstract_scalar, axis_index, source_tags,
};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Constraint {
    bound: u32,
    comparison: Comparison,
}
impl Constraint {
    pub fn bound(&self) -> u32 {
        self.bound
    }
    pub fn comparison(&self) -> Comparison {
        self.comparison
    }
}
/// Reconstructed only from the original immutable Plan. Not serialized.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Domain {
    extents: [u64; 2],
    rectangular: bool,
    base: usize,
    axes: [Option<Constraint>; 2],
}
impl Domain {
    pub fn rows(&self) -> u64 {
        self.extents[0]
    }
    pub fn columns(&self) -> u64 {
        self.extents[1]
    }
    pub fn rectangular(&self) -> bool {
        self.rectangular
    }
    pub fn base(&self) -> usize {
        self.base
    }
    pub fn constraint(&self, axis: Axis) -> Option<Constraint> {
        self.axes[usize::from(axis == Axis::Column)]
    }
    pub fn description(&self) -> Vec<(u8, u32, u8)> {
        self.axes
            .iter()
            .enumerate()
            .filter_map(|(axis, constraint)| {
                constraint.map(|c| (axis as u8, c.bound, c.comparison.relation_bits()))
            })
            .collect()
    }
    /// Preserve the smaller existing emitter for equivalent single masks.
    pub fn strictly_refines(&self, old: Option<&Mask>) -> bool {
        let Some(old) = old else { return true };
        let Some(current) = self.constraint(old.axis()) else {
            return false;
        };
        let bits = current.comparison.relation_bits();
        let previous = old.comparison().relation_bits();
        if current.bound != old.bound() || bits & !previous != 0 || self.base != old.base() {
            return false;
        }
        bits != previous
            || self
                .constraint(match old.axis() {
                    Axis::Row => Axis::Column,
                    Axis::Column => Axis::Row,
                })
                .is_some()
    }
    pub fn derive(
        plan: &Plan,
        proof: &crate::native_artifact::program::prepare::readonly_numeric::Proof,
        replacement: &Replacement,
    ) -> Option<Self> {
        let (extents, rectangular, tags) = source_tags(plan, proof, replacement)?;
        let region = &replacement.source.region;
        if region.body.len() > 512 {
            return None;
        }
        let mut bounds = [None; 2];
        let mut active = [[true; 3]; 3];
        let mut base = None;
        for row in 0..3 {
            for column in 0..3 {
                match classify(
                    plan,
                    replacement,
                    tags.clone(),
                    [row, column],
                    &mut bounds,
                    extents,
                ) {
                    Outcome::Identity(identity) => {
                        if base.is_some_and(|base| base != identity) {
                            return None;
                        }
                        base = Some(identity);
                        active[row][column] = false;
                    }
                    Outcome::Active => {}
                }
            }
        }
        let rows = std::array::from_fn::<_, 3, _>(|i| active[i].iter().any(|&yes| yes));
        let columns = std::array::from_fn::<_, 3, _>(|j| active.iter().any(|row| row[j]));
        // A nonfactorable active union retains ordinary source execution.
        if (0..3).any(|i| (0..3).any(|j| active[i][j] != (rows[i] && columns[j]))) {
            return None;
        }
        let bits = |selected: [bool; 3]| {
            selected
                .iter()
                .enumerate()
                .fold(0u8, |bits, (at, &yes)| bits | (u8::from(yes) << at))
        };
        let row_bits = bits(rows);
        let column_bits = bits(columns);
        if row_bits == 0 || column_bits == 0 || (row_bits == 7 && column_bits == 7) {
            return None;
        }
        let mut axes = [None; 2];
        for (axis, bits) in [row_bits, column_bits].into_iter().enumerate() {
            if bits != 7 {
                axes[axis] = Some(Constraint {
                    bound: bounds[axis]?,
                    comparison: Comparison::from_relation_bits(bits)?,
                });
            }
        }
        Some(Self {
            extents,
            rectangular,
            base: base?,
            axes,
        })
    }
}
#[derive(Clone, Copy)]
enum Outcome {
    Identity(usize),
    Active,
}

fn classify(
    plan: &Plan,
    replacement: &Replacement,
    mut tags: Tags,
    bucket: [usize; 2],
    bounds: &mut [Option<u32>; 2],
    extents: [u64; 2],
) -> Outcome {
    let region = &replacement.source.region;
    let iteration = region
        .rectangle
        .as_ref()
        .map_or(region.iteration, |r| r.inner_iteration);
    let Some(control) = plan.iterations().get(iteration as usize) else {
        return Outcome::Active;
    };
    let latch = control.as_ref().latch();
    let Some(&first) = region.body.first() else {
        return Outcome::Active;
    };
    let mut at = first;
    let mut seen = BTreeSet::new();
    let mut pushed = false;
    let mut base = None;
    loop {
        if at == latch {
            return match (pushed, base) {
                (true, Some(base)) => Outcome::Identity(base),
                _ => Outcome::Active,
            };
        }
        if !region.body.contains(&at) || !seen.insert(at) || seen.len() > 512 {
            return Outcome::Active;
        }
        let Some(block) = plan.blocks().get(at as usize) else {
            return Outcome::Active;
        };
        match block.action() {
            Action::LibraryCall {
                template,
                operands,
                output,
            } => {
                let Some(read) = region.reads.iter().find(|r| r.site == at) else {
                    return Outcome::Active;
                };
                let Some(matrix) = region.matrices.get(read.matrix) else {
                    return Outcome::Active;
                };
                let Some(&array) = replacement.arrays.get(read.matrix) else {
                    return Outcome::Active;
                };
                if base.is_some_and(|base| base != array)
                    || matrix.width != region.output_width
                    || operands.first().map(|v| v.get()) != Some(matrix.value)
                    || output.get() != read.output
                    || operands[1..].iter().map(|v| v.get()).collect::<Vec<_>>() != read.indices
                {
                    return Outcome::Active;
                }
                let same = match (template.primitive(), read.indices.as_slice()) {
                    (Primitive::MatrixCellAccess, [row, column]) => {
                        axis_index(plan, &tags, *row, Axis::Row, extents)
                            && axis_index(plan, &tags, *column, Axis::Column, extents)
                    }
                    (Primitive::MatrixScalarAccess, [index]) if region.rectangle.is_none() => {
                        axis_index(
                            plan,
                            &tags,
                            *index,
                            if extents[1] == 1 {
                                Axis::Row
                            } else {
                                Axis::Column
                            },
                            extents,
                        )
                    }
                    _ => false,
                };
                if !same {
                    return Outcome::Active;
                }
                base = Some(array);
                tags.insert(read.output, Tag::Cell(array));
            }
            Action::CollectionPush { value, .. } => {
                if pushed
                    || value.get() != region.result
                    || tags.get(&value.get()).copied() != base.map(Tag::Cell)
                {
                    return Outcome::Active;
                }
                pushed = true;
            }
            _ => {
                if abstract_scalar(plan, at, &mut tags, None).is_none() {
                    return Outcome::Active;
                }
            }
        }
        // Every understood predicate is decided only by its original axis and
        // one captured bound. Unknown/fallible work cannot establish identity.
        for tag in tags.values_mut() {
            if let Tag::Predicate(predicate) = *tag {
                let axis = usize::from(predicate.axis == Axis::Column);
                if !region.captures.contains(&predicate.bound) {
                    return Outcome::Active;
                }
                if bounds[axis].is_some_and(|bound| bound != predicate.bound) {
                    // This second bound is unknown in the current trichotomy.
                    // Retain the whole bucket, including any checked errors;
                    // do not discard identity proofs from independent buckets.
                    return Outcome::Active;
                }
                bounds[axis] = Some(predicate.bound);
                *tag = Tag::Bool(predicate.comparison.relation_bits() & (1 << bucket[axis]) != 0);
            }
        }
        at = match block.terminator() {
            Term::Scalar(Branch::Jump(next)) => next,
            Term::Scalar(Branch::Branch { condition, yes, no }) => {
                let Some(Tag::Bool(truth)) = tags.get(&condition.get()) else {
                    return Outcome::Active;
                };
                if *truth { yes } else { no }
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
                    return Outcome::Active;
                };
                if left == right { yes } else { no }
            }
            _ => return Outcome::Active,
        };
    }
}
