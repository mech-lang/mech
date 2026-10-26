//! Bounded, current-traversal scalar facts over the original closed CFG.
//! Partitions cover every initial carrier word, including invalid enum tags.
//! No fact crosses the latch; unknown identities and joins lose information.
use crate::native_artifact::program::codegen::packed_loop::*;
use mech_engine::native_artifact::NativeScalarOp as Op;

const MAX_SITES: usize = 256;
const MAX_WORDS: usize = 512;
const MAX_PARTITIONS: usize = 9;
const MAX_TERMS: usize = 4096;
const MAX_RELATIONS: usize = 128;
type Id = usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Relation {
    Eq,
    Ne,
    Lt,
    Le,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Arithmetic {
    Add,
    Sub,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Expr {
    Constant(u64),
    Initial(usize),
    Definition(usize, usize),
    Phi(usize, usize),
    Arithmetic(Arithmetic, Id, Id),
    Compare(Relation, Id, Id),
    Not(Id),
}
#[derive(Default)]
struct Terms {
    terms: Vec<Expr>,
    interned: BTreeMap<Expr, Id>,
}
impl Terms {
    fn intern(&mut self, expr: Expr) -> Option<Id> {
        if let Some(&id) = self.interned.get(&expr) {
            return Some(id);
        }
        if self.terms.len() == MAX_TERMS {
            return None;
        }
        let id = self.terms.len();
        self.terms.push(expr.clone());
        self.interned.insert(expr, id);
        Some(id)
    }
    fn constant(&mut self, value: u64) -> Option<Id> {
        self.intern(Expr::Constant(value))
    }
    fn value(&self, id: Id) -> Option<u64> {
        match self.terms[id] {
            Expr::Constant(value) => Some(value),
            _ => None,
        }
    }
    fn boolean(&self, mut id: Id) -> bool {
        for _ in 0..MAX_TERMS {
            match self.terms[id] {
                Expr::Compare(..) | Expr::Constant(0 | 1) => return true,
                Expr::Not(value) => id = value,
                _ => return false,
            }
        }
        false
    }
}
#[derive(Clone, Default)]
struct State {
    words: BTreeMap<usize, Id>,
    relations: BTreeSet<(Relation, Id, Id)>,
    successful: BTreeSet<(Arithmetic, Id, Id)>,
}
impl State {
    fn read(&self, slot: usize) -> Option<Id> {
        self.words.get(&slot).copied()
    }
    fn constant(&self, terms: &Terms, value: Id) -> Option<u64> {
        terms.value(value).or_else(|| {
            self.relations.iter().find_map(|&(relation, a, b)| {
                if relation != Relation::Eq {
                    return None;
                }
                if a == value {
                    terms.value(b)
                } else if b == value {
                    terms.value(a)
                } else {
                    None
                }
            })
        })
    }
    fn truth(&self, terms: &Terms, relation: Relation, a: Id, b: Id) -> Option<bool> {
        if a == b {
            return Some(matches!(relation, Relation::Eq | Relation::Le));
        }
        if let (Some(a), Some(b)) = (self.constant(terms, a), self.constant(terms, b)) {
            return Some(match relation {
                Relation::Eq => a == b,
                Relation::Ne => a != b,
                Relation::Lt => a < b,
                Relation::Le => a <= b,
            });
        }
        if self.relations.contains(&(relation, a, b)) {
            return Some(true);
        }
        let known = |r, x, y| self.relations.contains(&(r, x, y));
        match relation {
            Relation::Eq => {
                if known(Relation::Eq, b, a) {
                    return Some(true);
                }
                if known(Relation::Ne, a, b)
                    || known(Relation::Ne, b, a)
                    || known(Relation::Lt, a, b)
                    || known(Relation::Lt, b, a)
                {
                    return Some(false);
                }
            }
            Relation::Ne => return self.truth(terms, Relation::Eq, a, b).map(|v| !v),
            Relation::Lt => {
                if known(Relation::Le, b, a)
                    || known(Relation::Lt, b, a)
                    || known(Relation::Eq, a, b)
                    || known(Relation::Eq, b, a)
                {
                    return Some(false);
                }
                if terms.value(b) == Some(0) {
                    return Some(false);
                }
            }
            Relation::Le => {
                if known(Relation::Lt, a, b)
                    || known(Relation::Eq, a, b)
                    || known(Relation::Eq, b, a)
                    || terms.value(a) == Some(0)
                    || terms.value(b) == Some(u64::MAX)
                {
                    return Some(true);
                }
                if known(Relation::Lt, b, a) {
                    return Some(false);
                }
            }
        }
        None
    }
    fn assume(&mut self, terms: &Terms, relation: Relation, a: Id, b: Id) -> Option<bool> {
        if self.truth(terms, relation, a, b) == Some(false) {
            return Some(false);
        }
        if self.relations.len() >= MAX_RELATIONS {
            return None;
        }
        self.relations.insert((relation, a, b));
        Some(true)
    }
    fn condition(&mut self, terms: &Terms, mut condition: Id, mut yes: bool) -> Option<bool> {
        // Not chains are acyclic interned expressions; bound the walk anyway.
        for _ in 0..MAX_TERMS {
            let Expr::Not(value) = terms.terms[condition] else {
                break;
            };
            condition = value;
            yes = !yes;
        }
        match terms.terms[condition] {
            Expr::Constant(value) => Some((value != 0) == yes),
            Expr::Not(_) => None,
            Expr::Compare(mut relation, mut a, mut b) => {
                if !yes {
                    relation = match relation {
                        Relation::Eq => Relation::Ne,
                        Relation::Ne => Relation::Eq,
                        Relation::Lt => {
                            core::mem::swap(&mut a, &mut b);
                            Relation::Le
                        }
                        Relation::Le => {
                            core::mem::swap(&mut a, &mut b);
                            Relation::Lt
                        }
                    };
                }
                self.assume(terms, relation, a, b)
            }
            _ => Some(true),
        }
    }
    fn upper(&self, terms: &Terms, value: Id) -> u64 {
        if let Some(value) = self.constant(terms, value) {
            return value;
        }
        // A single established strict upper edge suffices for x+1. Do not
        // recursively chase relations or infer a bound from one phi arm.
        let mut upper = u64::MAX;
        for &(relation, a, b) in &self.relations {
            if a == value {
                match relation {
                    Relation::Eq | Relation::Le => {
                        upper = upper.min(terms.value(b).unwrap_or(u64::MAX));
                    }
                    Relation::Lt => {
                        upper = upper.min(terms.value(b).unwrap_or(u64::MAX).saturating_sub(1));
                    }
                    Relation::Ne => {}
                }
            } else if b == value && relation == Relation::Eq {
                upper = upper.min(terms.value(a).unwrap_or(u64::MAX));
            }
        }
        upper
    }
    fn safe(&self, terms: &Terms, op: Arithmetic, a: Id, b: Id) -> bool {
        if self.successful.contains(&(op, a, b))
            || (op == Arithmetic::Add && self.successful.contains(&(op, b, a)))
        {
            return true;
        }
        match op {
            Arithmetic::Sub => self.truth(terms, Relation::Le, b, a) == Some(true),
            Arithmetic::Add => {
                if u128::from(self.upper(terms, a)) + u128::from(self.upper(terms, b))
                    <= u128::from(u64::MAX)
                {
                    return true;
                }
                // After a successful checked difference d=n-i, w<=d proves
                // i+w<=n. The exact i identity must match the subtraction.
                self.relations.iter().any(|&(relation, width, difference)| {
                    if !matches!(relation, Relation::Le | Relation::Lt | Relation::Eq) {
                        return false;
                    }
                    let Expr::Arithmetic(Arithmetic::Sub, n, index) = terms.terms[difference]
                    else {
                        return false;
                    };
                    self.successful.contains(&(Arithmetic::Sub, n, index))
                        && ((width == a && index == b) || (width == b && index == a))
                })
            }
        }
    }
    fn merge(&mut self, other: &Self, site: usize, terms: &mut Terms) -> Option<()> {
        for (&slot, value) in &mut self.words {
            if other.words.get(&slot) != Some(&*value) {
                *value = terms.intern(Expr::Phi(site, slot))?;
            }
        }
        self.relations.retain(|fact| other.relations.contains(fact));
        self.successful
            .retain(|fact| other.successful.contains(fact));
        Some(())
    }
}

#[derive(Default)]
pub(in crate::native_artifact::program::codegen::packed_loop) struct Proof {
    guards: BTreeSet<(usize, u32, usize, u32)>,
    arithmetic: BTreeMap<usize, (Op, Vec<NativeMatchValue>)>,
}
impl Proof {
    pub fn guard(
        &self,
        site: usize,
        source: NativeMatchValue,
        offset: usize,
        ordinal: u32,
    ) -> bool {
        self.guards.contains(&(site, source.get(), offset, ordinal))
    }
    pub fn arithmetic(&self, site: usize, op: Op, operands: &[NativeMatchValue]) -> bool {
        self.arithmetic
            .get(&site)
            .is_some_and(|(proved, inputs)| *proved == op && inputs == operands)
    }
}

fn successors(term: Term) -> Option<Vec<u32>> {
    Some(match term {
        Term::TailHeader { body, .. } | Term::Scalar(Branch::Jump(body)) => vec![body],
        Term::Scalar(Branch::Branch { yes, no, .. } | Branch::LiteralBranch { yes, no, .. }) => {
            vec![yes, no]
        }
        Term::Scalar(Branch::Reject) => Vec::new(),
        _ => return None,
    })
}
fn order(code: CodePlan<'_>, closed: &Layout, id: usize) -> Option<Vec<usize>> {
    let region = &closed.regions[id];
    let sites = closed
        .sites
        .iter()
        .enumerate()
        .filter_map(|(i, owner)| (*owner == Some(id)).then_some(i))
        .collect::<Vec<_>>();
    if sites.len() > MAX_SITES {
        return None;
    }
    let mut incoming = BTreeMap::<usize, usize>::new();
    for &site in &sites {
        incoming.insert(site, 0);
    }
    for &site in &sites {
        if site == region.tail.latch() as usize {
            continue;
        }
        for next in successors(code.blocks()[site].terminator())? {
            if next != region.tail.exit() {
                *incoming.get_mut(&(next as usize))? += 1;
            }
        }
    }
    let mut ready = incoming
        .iter()
        .filter_map(|(&site, &n)| (n == 0).then_some(site))
        .collect::<Vec<_>>();
    let mut ordered = Vec::new();
    while let Some(site) = ready.pop() {
        ordered.push(site);
        if site == region.tail.latch() as usize {
            continue;
        }
        for next in successors(code.blocks()[site].terminator())? {
            if next == region.tail.exit() {
                continue;
            }
            let count = incoming.get_mut(&(next as usize))?;
            *count = count.checked_sub(1)?;
            if *count == 0 {
                ready.push(next as usize);
            }
        }
    }
    (ordered.len() == sites.len()).then_some(ordered)
}

fn aggregate(
    state: &mut State,
    terms: &mut Terms,
    layout: &NativeScalarAggregates,
    action: &Aggregate,
) -> Option<()> {
    let at = |source, offset| field(layout, source, offset);
    let (output, values) = match action {
        Aggregate::Pack { output, words } => (
            *output,
            words
                .iter()
                .map(|word| match word {
                    Word::Scalar(value) => state.read(value.get() as usize),
                    Word::Field(value, offset) => state.read(at(*value, *offset)?),
                    Word::Constant(word) => terms.constant(*word),
                })
                .collect::<Option<Vec<_>>>()?,
        ),
        Aggregate::Copy { source, output } => {
            let count = layout.slot(*output)?.1;
            (
                *output,
                (0..count)
                    .map(|offset| state.read(at(*source, offset)?))
                    .collect::<Option<Vec<_>>>()?,
            )
        }
        Aggregate::Project {
            source,
            offset,
            output,
            ..
        } => {
            let count = layout.slot(*output)?.1;
            (
                *output,
                (0..count)
                    .map(|i| state.read(at(*source, offset.checked_add(i)?)?))
                    .collect::<Option<Vec<_>>>()?,
            )
        }
        Aggregate::Observe {
            source,
            offset,
            ordinal,
            output,
            ..
        } => {
            let value = state.read(at(*source, *offset)?)?;
            let value = if let Some(ordinal) = ordinal {
                let tag = terms.constant(u64::from(*ordinal))?;
                terms.intern(Expr::Compare(Relation::Eq, value, tag))?
            } else {
                value
            };
            state.words.insert(output.get() as usize, value);
            return Some(());
        }
    };
    for (i, value) in values.into_iter().enumerate() {
        state.words.insert(at(output, i)?, value);
    }
    Some(())
}

fn scalar(state: &mut State, terms: &mut Terms, site: usize, action: &Scalar) -> Option<()> {
    match action {
        Scalar::None => {}
        Scalar::Copy { source, output } => {
            state
                .words
                .insert(output.get() as usize, state.read(source.get() as usize)?);
        }
        Scalar::Primitive {
            op,
            operands,
            output,
        } => {
            let args = operands
                .iter()
                .map(|v| state.read(v.get() as usize))
                .collect::<Option<Vec<_>>>()?;
            let value = match *op {
                Op::U64Add | Op::U64Sub => {
                    let op = if *op == Op::U64Add {
                        Arithmetic::Add
                    } else {
                        Arithmetic::Sub
                    };
                    let [a, b] = args.as_slice() else {
                        return None;
                    };
                    if state.successful.len() == MAX_RELATIONS {
                        return None;
                    }
                    state.successful.insert((op, *a, *b));
                    terms.intern(Expr::Arithmetic(op, *a, *b))?
                }
                Op::U64Eq | Op::U64Ne | Op::U64Lt | Op::U64Le | Op::U64Gt | Op::U64Ge => {
                    let [mut a, mut b]: [Id; 2] = args.as_slice().try_into().ok()?;
                    let relation = match *op {
                        Op::U64Eq => Relation::Eq,
                        Op::U64Ne => Relation::Ne,
                        Op::U64Lt => Relation::Lt,
                        Op::U64Le => Relation::Le,
                        Op::U64Gt => {
                            core::mem::swap(&mut a, &mut b);
                            Relation::Lt
                        }
                        _ => {
                            core::mem::swap(&mut a, &mut b);
                            Relation::Le
                        }
                    };
                    terms.intern(Expr::Compare(relation, a, b))?
                }
                Op::BoolNot if terms.boolean(*args.first()?) => {
                    terms.intern(Expr::Not(*args.first()?))?
                }
                _ => terms.intern(Expr::Definition(site, output.get() as usize))?,
            };
            state.words.insert(output.get() as usize, value);
        }
    }
    Some(())
}

/// Compiler-owned facts are recomputed from this exact plan. Bounded analysis
/// failure declines all substitutions; nothing is accepted from a descriptor.
pub(in crate::native_artifact::program::codegen::packed_loop) fn derive(
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates,
    slots: &BTreeSet<usize>,
) -> Option<Proof> {
    if slots.len() > MAX_WORDS {
        return None;
    }
    let ordered = order(code, closed, id)?;
    let region = &closed.regions[id];
    let owner_index = region.map?;
    let mut terms = Terms::default();
    let mut initial = State::default();
    for &slot in slots {
        initial
            .words
            .insert(slot, terms.intern(Expr::Initial(slot))?);
    }
    for i in 0..code.constant_count() {
        let (value, constant) = code.constant(i).ok()?;
        if let Some(constant) = constant
            && slots.contains(&(value.get() as usize))
        {
            initial
                .words
                .insert(value.get() as usize, terms.constant(constant)?);
        }
    }
    let tag = owner
        .aggregates()
        .slot(region.tail.carrier())
        .and_then(|(slot, _)| initial.read(slot));
    let mut ordinals = BTreeSet::new();
    for &site in &ordered {
        if let Some((_, action)) = aggregate_at(owner, aggregates, owner_index, site) {
            match action {
                Aggregate::Observe {
                    ordinal: Some(ordinal),
                    ..
                } => {
                    ordinals.insert(*ordinal);
                }
                _ => {}
            }
        }
    }
    let mut partitions = Vec::new();
    if let Some(tag) = tag
        && !ordinals.is_empty()
        && ordinals.len() < MAX_PARTITIONS
    {
        for &ordinal in &ordinals {
            let mut state = initial.clone();
            let value = terms.constant(u64::from(ordinal))?;
            state.assume(&terms, Relation::Eq, tag, value)?;
            partitions.push(state);
        }
        // The native entry may contain a malformed tag. This exhaustive
        // remainder preserves its original Reject/ABI path and origin.
        for ordinal in ordinals {
            let value = terms.constant(u64::from(ordinal))?;
            initial.assume(&terms, Relation::Ne, tag, value)?;
        }
    }
    partitions.push(initial);
    let mut incoming: BTreeMap<usize, Vec<Option<State>>> = ordered
        .iter()
        .map(|&site| (site, vec![None; partitions.len()]))
        .collect();
    *incoming.get_mut(&(region.tail.header() as usize))? =
        partitions.into_iter().map(Some).collect();
    let mut proof = Proof::default();
    for site in ordered {
        let mut states = incoming.remove(&site)?;
        if states.iter().all(Option::is_none) {
            continue;
        }
        let block = &code.blocks()[site];
        if let Some((layout, action)) = aggregate_at(owner, aggregates, owner_index, site) {
            if let Aggregate::Project { source, guards, .. }
            | Aggregate::Observe { source, guards, .. } = action
            {
                for &(offset, ordinal) in guards {
                    let tag = field(layout, *source, offset)?;
                    let value = terms.constant(u64::from(ordinal))?;
                    if states.iter().flatten().all(|state| {
                        state.read(tag).is_some_and(|tag| {
                            state.truth(&terms, Relation::Eq, tag, value) == Some(true)
                        })
                    }) {
                        proof.guards.insert((site, source.get(), offset, ordinal));
                    }
                    for state in &mut states {
                        if let Some(current) = state {
                            let tag = current.read(tag)?;
                            if !current.assume(&terms, Relation::Eq, tag, value)? {
                                *state = None;
                            }
                        }
                    }
                }
            }
            for state in states.iter_mut().flatten() {
                aggregate(state, &mut terms, layout, action)?;
            }
        } else if let Some(span) = owner.span_site(site).map(|i| &owner.spans()[i]) {
            // The paired range/selection implementation writes only its
            // private packed Frame, never a source scalar or aggregate word.
            if span.region() != owner_index {
                return None;
            }
        } else if owner.call_region(site) == Some(owner_index) {
            let Action::LibraryCall {
                template, output, ..
            } = block.action()
            else {
                return None;
            };
            if !matches!(
                template.primitive(),
                P::MapContainsKey | P::MapKeyAccess | P::MapInsert
            ) {
                return None;
            }
            // These exact private operations are the only calls in this
            // emitted CFG. They change the Frame and their result word;
            // source carriers and other scalar words remain in SSA registers.
            let value = terms.intern(Expr::Definition(site, output.get() as usize))?;
            for state in states.iter_mut().flatten() {
                state.words.insert(output.get() as usize, value);
            }
        } else {
            match block.action() {
                Action::Scalar(action) => {
                    if let Scalar::Primitive { op, operands, .. } = action
                        && matches!(op, Op::U64Add | Op::U64Sub)
                        && operands.len() == 2
                    {
                        let operation = if *op == Op::U64Add {
                            Arithmetic::Add
                        } else {
                            Arithmetic::Sub
                        };
                        if states.iter().flatten().all(|state| {
                            match (
                                state.read(operands[0].get() as usize),
                                state.read(operands[1].get() as usize),
                            ) {
                                (Some(a), Some(b)) => state.safe(&terms, operation, a, b),
                                _ => false,
                            }
                        }) {
                            proof.arithmetic.insert(site, (*op, operands.to_vec()));
                        }
                    }
                    for state in states.iter_mut().flatten() {
                        scalar(state, &mut terms, site, action)?;
                    }
                }
                _ => return None,
            }
        }
        if site == region.tail.latch() as usize {
            continue;
        }
        for (branch, next) in successors(block.terminator())?.into_iter().enumerate() {
            if next == region.tail.exit() {
                continue;
            }
            let target = incoming.get_mut(&(next as usize))?;
            for (target, state) in target.iter_mut().zip(&states) {
                let Some(mut state) = state.clone() else {
                    continue;
                };
                let reachable = match block.terminator() {
                    Term::Scalar(Branch::Branch { condition, .. }) => {
                        let condition = state.read(condition.get() as usize)?;
                        state.condition(&terms, condition, branch == 0)?
                    }
                    Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                        // Float equality is not raw-word equality (NaNs/-0).
                        if code.kind(left) == Some(NativeScalarKind::F64) {
                            true
                        } else {
                            let (a, b) = (
                                state.read(left.get() as usize)?,
                                state.read(right.get() as usize)?,
                            );
                            state.assume(
                                &terms,
                                if branch == 0 {
                                    Relation::Eq
                                } else {
                                    Relation::Ne
                                },
                                a,
                                b,
                            )?
                        }
                    }
                    _ => true,
                };
                if !reachable {
                    continue;
                }
                if let Some(target) = target {
                    target.merge(&state, next as usize, &mut terms)?;
                } else {
                    *target = Some(state);
                }
            }
        }
    }
    Some(proof)
}

#[cfg(test)]
#[path = "tests.rs"]
pub(in crate::native_artifact::program::codegen::packed_loop::edge_facts) mod tests;
