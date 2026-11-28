use super::*;
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Predicate {
    /// Normalized comparison: current outer cursor is always the left operand.
    pub op: NativeScalarOp,
    pub bound: u32,
    pub dot_when_true: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Batch {
    pub dot: Dot,
    pub matrices: Vec<usize>,
    pub first: u32,
    pub last: u32,
    pub inclusive: bool,
    pub maximum: u64,
    pub constant_count: bool,
    pub predicate: Option<Predicate>,
    pub alternative: Option<u32>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Symbol {
    Cursor,
    Word(u32),
    Bool(bool),
    Dot,
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
fn boolean(plan: &Plan, value: u32) -> Option<bool> {
    plan.constants()
        .iter()
        .find(|c| c.value().get() == value)
        .and_then(|c| match c.constant().data() {
            mech_core::ValueData::Bool(v) => Some(*v),
            _ => None,
        })
}
fn symbol(
    plan: &Plan,
    region: &Region,
    values: &BTreeMap<u32, Symbol>,
    value: u32,
) -> Option<Symbol> {
    values
        .get(&value)
        .copied()
        .or_else(|| boolean(plan, value).map(Symbol::Bool))
        .or_else(|| {
            region
                .captures
                .contains(&value)
                .then_some(Symbol::Word(value))
        })
}
struct PathResult {
    result: Symbol,
    predicate: Option<(NativeScalarOp, u32)>,
    sites: BTreeSet<u32>,
}
fn path(plan: &Plan, region: &Region, truth: bool) -> Option<PathResult> {
    let child = &region.nested[0];
    let latch = plan
        .iterations()
        .get(region.iteration as usize)?
        .as_ref()
        .latch();
    let body: BTreeSet<_> = region.body.iter().copied().collect();
    let mut sites = BTreeSet::new();
    let mut at = *region.body.first()?;
    let mut values = BTreeMap::from([(region.element, Symbol::Cursor)]);
    let mut predicate = None;
    let mut result = None;
    let mut children = 0;
    for _ in 0..256 {
        if at == latch {
            return Some(PathResult {
                result: result?,
                predicate,
                sites,
            });
        }
        if !body.contains(&at) || !sites.insert(at) {
            return None;
        }
        if at == child.source.entry {
            children += 1;
            if children > 1 {
                return None;
            }
            values.insert(child.output(plan).get(), Symbol::Dot);
            at = child.source.after;
            continue;
        }
        let block = plan.blocks().get(at as usize)?;
        match block.action() {
            Action::Scalar(Scalar::None) | Action::CollectionElement { .. } => {}
            Action::Scalar(Scalar::Copy { source, output }) => {
                let v = symbol(plan, region, &values, source.get())?;
                values.insert(output.get(), v);
            }
            Action::Scalar(Scalar::Primitive {
                op,
                operands,
                output,
            }) if operands.len() == 2 => {
                let left = symbol(plan, region, &values, operands[0].get())?;
                let right = symbol(plan, region, &values, operands[1].get())?;
                let (bound, reverse) = match (left, right) {
                    (Symbol::Cursor, Symbol::Word(bound)) => (bound, false),
                    (Symbol::Word(bound), Symbol::Cursor) => (bound, true),
                    _ => return None,
                };
                if plan.values().get(bound as usize)?.scalar_kind() != Some(NativeScalarKind::U64)
                    || predicate.is_some()
                {
                    return None;
                }
                predicate = Some((comparison(*op, reverse)?, bound));
                values.insert(output.get(), Symbol::Bool(truth));
            }
            Action::CollectionPush { collection, value }
                if *collection == region.collection && result.is_none() =>
            {
                result = Some(symbol(plan, region, &values, value.get())?);
            }
            _ => return None,
        }
        at = match block.terminator() {
            Terminator::Scalar(Term::Jump(to)) => to,
            Terminator::Scalar(Term::Branch { condition, yes, no }) => {
                match symbol(plan, region, &values, condition.get())? {
                    Symbol::Bool(true) => yes,
                    Symbol::Bool(false) => no,
                    _ => return None,
                }
            }
            Terminator::Scalar(Term::LiteralBranch {
                left,
                right,
                yes,
                no,
            }) => match (
                symbol(plan, region, &values, left.get())?,
                symbol(plan, region, &values, right.get())?,
            ) {
                (Symbol::Bool(a), Symbol::Bool(b)) => {
                    if a == b {
                        yes
                    } else {
                        no
                    }
                }
                _ => return None,
            },
            _ => return None,
        };
    }
    None
}
impl Batch {
    pub fn derive(plan: &Plan, region: &Region) -> Option<Self> {
        if !region.range
            || region.rectangle.is_some()
            || region.reduction.is_some()
            || region.nested.len() != 1
            || region.output_width != 8
            || region.body.len() > 256
        {
            return None;
        }
        if plan.values().get(region.result as usize)?.scalar_kind() != Some(NativeScalarKind::F64) {
            return None;
        }
        let child = &region.nested[0];
        let independent = |id: u32| {
            id != region.element && (region.captures.contains(&id) || constant(plan, id).is_some())
        };
        if !independent(child.source.first) || !independent(child.source.last) {
            return None;
        }
        let dot = Dot::derive(plan, &child.source.region, Some(region.element))?;
        if child.matrices.len() != child.source.region.matrices.len() {
            return None;
        }
        for (index, &parent) in child.matrices.iter().enumerate() {
            if region.matrices.get(parent)? != child.source.region.matrices.get(index)? {
                return None;
            }
        }
        let yes = path(plan, region, true)?;
        let no = path(plan, region, false)?;
        if yes.predicate != no.predicate {
            return None;
        }
        // All reachable Boolean paths are checked. The sole remaining source
        // reject arm must be a pure Boolean exhaustiveness failure.
        let covered = yes.sites.union(&no.sites).copied().collect::<BTreeSet<_>>();
        for &site in &region.body {
            if !covered.contains(&site) {
                let b = plan.blocks().get(site as usize)?;
                if !matches!(b.action(), Action::Scalar(Scalar::None))
                    || !matches!(b.terminator(), Terminator::Scalar(Term::Reject))
                {
                    return None;
                }
            }
        }
        let (predicate, alternative) = match (yes.result, no.result, yes.predicate) {
            (Symbol::Dot, Symbol::Dot, None) => (None, None),
            (Symbol::Dot, Symbol::Word(other), Some((op, bound))) => (
                Some(Predicate {
                    op,
                    bound,
                    dot_when_true: true,
                }),
                Some(other),
            ),
            (Symbol::Word(other), Symbol::Dot, Some((op, bound))) => (
                Some(Predicate {
                    op,
                    bound,
                    dot_when_true: false,
                }),
                Some(other),
            ),
            _ => return None,
        };
        if alternative.is_some_and(|v| {
            !region.captures.contains(&v)
                || plan.values().get(v as usize).and_then(|t| t.scalar_kind())
                    != Some(NativeScalarKind::F64)
        }) {
            return None;
        }
        Some(Self {
            dot,
            matrices: child.matrices.clone(),
            first: child.source.first,
            last: child.source.last,
            inclusive: child.source.inclusive,
            maximum: child.source.maximum,
            constant_count: child.source.constant_count,
            predicate,
            alternative,
        })
    }
}
