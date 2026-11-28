//! Closed, piecewise typed cell expressions. These source facts carry no
//! current owner, pointer, admission, scratch or publication authority.
//! The final F64 cell is the only FP observation: source control is U64-only,
//! every allowed FP primitive is total and NaN-propagating, and conversions,
//! FP predicates, extra pushes/effects or hidden checked work decline. This
//! permits result-boundary validation while retaining every source operation.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct CellPredicate {
    pub axis: u8,
    pub op: NativeScalarOp,
    pub bound: u32,
}
impl CellPredicate {
    pub fn inverted(self) -> Self {
        use NativeScalarOp::*;
        Self {
            op: match self.op {
                U64Eq => U64Ne,
                U64Ne => U64Eq,
                U64Lt => U64Ge,
                U64Le => U64Gt,
                U64Gt => U64Le,
                U64Ge => U64Lt,
                _ => unreachable!("proved unsigned axis comparison"),
            },
            ..self
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum CellValue {
    Constant(u64),
    Capture(u32),
    Read(usize),
    Unary(NativeScalarOp, usize),
    Binary(NativeScalarOp, [usize; 2]),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct CellPath {
    pub predicates: Vec<CellPredicate>,
    pub edges: Vec<(u32, u32)>,
    pub reads: Vec<Operand>,
    /// Source-ordered operations, including unused FP operations and reads.
    pub values: Vec<CellValue>,
    pub result: usize,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct CellProgram {
    pub paths: Vec<CellPath>,
}
#[derive(Clone, Copy)]
enum Tag {
    Axis(u8),
    Bound(u32),
    Bool(bool),
    Predicate(CellPredicate),
}
#[derive(Clone)]
struct Path {
    at: u32,
    sites: Vec<u32>,
    tags: BTreeMap<u32, Tag>,
    predicates: Vec<CellPredicate>,
    edges: Vec<(u32, u32)>,
    total: BTreeSet<u32>,
}
fn comparison(op: NativeScalarOp, reverse: bool) -> Option<NativeScalarOp> {
    use NativeScalarOp::*;
    Some(match (op, reverse) {
        (U64Eq, _) => U64Eq,
        (U64Ne, _) => U64Ne,
        (U64Lt, false) | (U64Gt, true) => U64Lt,
        (U64Le, false) | (U64Ge, true) => U64Le,
        (U64Gt, false) | (U64Lt, true) => U64Gt,
        (U64Ge, false) | (U64Le, true) => U64Ge,
        _ => return None,
    })
}
impl Path {
    fn normalize(&self, tag: Tag) -> Tag {
        if let Tag::Predicate(p) = tag {
            if self.predicates.contains(&p) {
                return Tag::Bool(true);
            }
            if self.predicates.contains(&p.inverted()) {
                return Tag::Bool(false);
            }
        }
        tag
    }
    fn tag(&self, id: u32) -> Option<Tag> {
        self.tags.get(&id).copied().map(|t| self.normalize(t))
    }
    fn scalar(&mut self, site: u32, action: &Action) {
        use NativeScalarOp::*;
        match action {
            Action::Scalar(Scalar::Copy { source, output }) => {
                let tag = self.tag(source.get());
                self.tags.remove(&output.get());
                if let Some(tag) = tag {
                    self.tags.insert(output.get(), tag);
                }
                self.total.insert(site);
            }
            Action::Scalar(Scalar::Primitive {
                op,
                operands,
                output,
            }) => {
                let tags = operands
                    .iter()
                    .map(|v| self.tag(v.get()))
                    .collect::<Option<Vec<_>>>();
                let tag = tags.as_deref().and_then(|tags| {
                    Some(match (*op, tags) {
                        (BoolNot, [Tag::Bool(v)]) => Tag::Bool(!v),
                        (BoolNot, [Tag::Predicate(p)]) => Tag::Predicate(p.inverted()),
                        (BoolEq | BoolNe, [Tag::Bool(a), Tag::Bool(b)]) => {
                            Tag::Bool((a == b) == (*op == BoolEq))
                        }
                        (
                            BoolEq | BoolNe,
                            [Tag::Predicate(p), Tag::Bool(v)] | [Tag::Bool(v), Tag::Predicate(p)],
                        ) => Tag::Predicate(if *v == (*op == BoolEq) {
                            *p
                        } else {
                            p.inverted()
                        }),
                        (op, [Tag::Axis(axis), Tag::Bound(bound)]) => {
                            Tag::Predicate(CellPredicate {
                                axis: *axis,
                                bound: *bound,
                                op: comparison(op, false)?,
                            })
                        }
                        (op, [Tag::Bound(bound), Tag::Axis(axis)]) => {
                            Tag::Predicate(CellPredicate {
                                axis: *axis,
                                bound: *bound,
                                op: comparison(op, true)?,
                            })
                        }
                        _ => return None,
                    })
                });
                self.tags.remove(&output.get());
                if let Some(tag) = tag {
                    self.tags.insert(output.get(), self.normalize(tag));
                    self.total.insert(site);
                }
            }
            Action::LibraryCall { output, .. } => {
                self.tags.remove(&output.get());
            }
            _ => {}
        }
    }
    fn condition(&self, term: Term) -> Option<(Tag, u32, u32)> {
        match term {
            Term::Branch { condition, yes, no } => Some((self.tag(condition.get())?, yes, no)),
            Term::LiteralBranch {
                left,
                right,
                yes,
                no,
            } => {
                let tag = match (self.tag(left.get())?, self.tag(right.get())?) {
                    (Tag::Bool(a), Tag::Bool(b)) => Tag::Bool(a == b),
                    (Tag::Predicate(p), Tag::Bool(v)) | (Tag::Bool(v), Tag::Predicate(p)) => {
                        Tag::Predicate(if v { p } else { p.inverted() })
                    }
                    _ => return None,
                };
                Some((self.normalize(tag), yes, no))
            }
            _ => None,
        }
    }
}
struct Expression<'a> {
    source: affine::Builder<'a>,
    nodes: BTreeMap<u32, usize>,
    values: Vec<CellValue>,
    reads: Vec<Operand>,
}
impl Expression<'_> {
    fn value(&mut self, value: u32, depth: usize) -> Option<usize> {
        if depth > 32
            || self.source.plan.values().get(value as usize)?.scalar_kind()
                != Some(NativeScalarKind::F64)
        {
            return None;
        }
        if let Some(&at) = self.nodes.get(&value) {
            return Some(at);
        }
        let node = if let Some(entry) = self
            .source
            .plan
            .constants()
            .iter()
            .find(|v| v.value().get() == value)
        {
            let mech_core::ValueData::F64(bits) = entry.constant().data() else {
                return None;
            };
            CellValue::Constant(bits.bits())
        } else if self.source.region.captures.contains(&value) {
            CellValue::Capture(value)
        } else {
            let &(site, action) = self.source.definitions.get(&value)?;
            self.source.used.insert(site);
            match action {
                Action::Scalar(Scalar::Copy { source, .. }) => {
                    let at = self.value(source.get(), depth + 1)?;
                    self.nodes.insert(value, at);
                    return Some(at);
                }
                Action::LibraryCall { .. } => {
                    let operand = self.source.read(value, depth + 1)?;
                    let at = self.reads.len();
                    self.reads.push(operand);
                    CellValue::Read(at)
                }
                Action::Scalar(Scalar::Primitive { op, operands, .. }) => {
                    use NativeScalarOp::*;
                    match (*op, &operands[..]) {
                        (F64Neg | F64Sqrt, [a]) => {
                            CellValue::Unary(*op, self.value(a.get(), depth + 1)?)
                        }
                        (F64Add | F64Sub | F64Mul | F64Div, [a, b]) => CellValue::Binary(
                            *op,
                            [
                                self.value(a.get(), depth + 1)?,
                                self.value(b.get(), depth + 1)?,
                            ],
                        ),
                        _ => return None,
                    }
                }
                _ => return None,
            }
        };
        let at = self.values.len();
        self.values.push(node);
        self.nodes.insert(value, at);
        Some(at)
    }
}
fn expression(plan: &Plan, region: &Region, path: &Path) -> Option<CellPath> {
    let mut selected = region.clone();
    selected.body = path.sites.clone();
    let mut e = Expression {
        source: affine::Builder::new(
            plan,
            &selected,
            region.rectangle.as_ref().map(|r| r.inner_element),
        )?,
        nodes: BTreeMap::new(),
        values: Vec::new(),
        reads: Vec::new(),
    };
    let owner_collection = region
        .rectangle
        .as_ref()
        .map_or(region.collection, |r| r.inner_collection);
    let mut pushed = false;
    for &site in &path.sites {
        match plan.blocks().get(site as usize)?.action() {
            Action::LibraryCall { output, .. }
            | Action::Scalar(Scalar::Primitive { output, .. })
                if plan.values().get(output.get() as usize)?.scalar_kind()
                    == Some(NativeScalarKind::F64) =>
            {
                e.value(output.get(), 0)?;
            }
            Action::CollectionPush { collection, value } => {
                if pushed || *collection != owner_collection || value.get() != region.result {
                    return None;
                }
                e.value(value.get(), 0)?;
                pushed = true;
            }
            _ => {}
        }
    }
    let result = e.value(region.result, 0)?;
    // A complete source closure is mandatory: unused checked integer work,
    // hidden library calls, allocations and effects cannot disappear.
    for &site in &path.sites {
        match plan.blocks().get(site as usize)?.action() {
            Action::Scalar(Scalar::None)
            | Action::CollectionElement { .. }
            | Action::CollectionPush { .. } => {}
            Action::Scalar(Scalar::Copy { .. } | Scalar::Primitive { .. })
            | Action::LibraryCall { .. }
                if path.total.contains(&site) || e.source.used.contains(&site) => {}
            _ => return None,
        }
    }
    pushed.then_some(CellPath {
        predicates: path.predicates.clone(),
        edges: path.edges.clone(),
        reads: e.reads,
        values: e.values,
        result,
    })
}
impl CellProgram {
    pub fn derive(plan: &Plan, region: &Region) -> Option<Self> {
        if !region.range
            || region.reduction.is_some()
            || !region.nested.is_empty()
            || region.output_width != 8
            || region.body.is_empty()
            || region.body.len() > 256
            || plan.values().get(region.result as usize)?.scalar_kind()
                != Some(NativeScalarKind::F64)
        {
            return None;
        }
        let latch = plan
            .iterations()
            .get(
                region
                    .rectangle
                    .as_ref()
                    .map_or(region.iteration, |r| r.inner_iteration) as usize,
            )?
            .as_ref()
            .latch();
        let mut tags = BTreeMap::from([(region.element, Tag::Axis(0))]);
        if let Some(r) = &region.rectangle {
            tags.insert(r.inner_element, Tag::Axis(1));
        }
        for &id in &region.captures {
            if plan.values().get(id as usize)?.scalar_kind() == Some(NativeScalarKind::U64) {
                tags.insert(id, Tag::Bound(id));
            }
        }
        for c in plan.constants() {
            match c.constant().data() {
                mech_core::ValueData::Bool(v) => {
                    tags.insert(c.value().get(), Tag::Bool(*v));
                }
                mech_core::ValueData::U64(_) => {
                    tags.insert(c.value().get(), Tag::Bound(c.value().get()));
                }
                _ => {}
            }
        }
        let mut pending = vec![Path {
            at: region.body[0],
            sites: Vec::new(),
            tags,
            predicates: Vec::new(),
            edges: Vec::new(),
            total: BTreeSet::new(),
        }];
        let mut paths = Vec::new();
        let mut covered = BTreeSet::new();
        while let Some(mut path) = pending.pop() {
            loop {
                if path.at == latch {
                    let closed = expression(plan, region, &path)?;
                    covered.extend(path.sites);
                    paths.push(closed);
                    break;
                }
                if !region.body.contains(&path.at) || path.sites.contains(&path.at) {
                    return None;
                }
                let site = path.at;
                path.sites.push(site);
                let block = plan.blocks().get(site as usize)?;
                path.scalar(site, block.action());
                match block.terminator() {
                    Terminator::Scalar(Term::Jump(to)) => path.at = to,
                    Terminator::Scalar(term) => {
                        let (condition, yes, no) = path.condition(term)?;
                        match condition {
                            Tag::Bool(v) => {
                                path.at = if v { yes } else { no };
                                path.edges.push((site, path.at));
                            }
                            Tag::Predicate(p) => {
                                if path.predicates.len() >= 4 || pending.len() + paths.len() >= 7 {
                                    return None;
                                }
                                let mut other = path.clone();
                                other.predicates.push(p.inverted());
                                other.at = no;
                                other.edges.push((site, no));
                                pending.push(other);
                                path.predicates.push(p);
                                path.at = yes;
                                path.edges.push((site, yes));
                            }
                            _ => return None,
                        }
                    }
                    _ => return None,
                }
            }
        }
        for &site in &region.body {
            if !covered.contains(&site) {
                let b = plan.blocks().get(site as usize)?;
                if !matches!(b.action(), Action::Scalar(Scalar::None))
                    || b.terminator() != Terminator::Scalar(Term::Reject)
                {
                    return None;
                }
            }
        }
        (!paths.is_empty()).then_some(Self { paths })
    }
}
#[cfg(test)]
#[path = "cell_tests.rs"]
mod tests;
