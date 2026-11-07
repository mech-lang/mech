//! Upper envelopes for current, immutable source operands. Overflow in an
//! envelope declines admission; it is not source arithmetic and cannot trap.
use super::*;
pub(super) struct Upper<'a, 'p> {
    pub flow: &'a Flow<'p>,
    pub resolver: &'a Resolver<'p>,
    pub prior: &'a [Trips],
    pub calls: &'a [Option<NativePreparedLibraryCall>],
    pub tag: Option<u64>,
    pub active: BTreeSet<(Atom, usize)>,
}
impl Upper<'_, '_> {
    pub fn value(&mut self, original: Atom, context: usize) -> Option<Bound> {
        let atom = self.resolver.atom(original)?;
        if !self.active.insert((atom, context)) {
            return None;
        }
        let result = self.inner(atom, context);
        self.active.remove(&(atom, context));
        result
    }
    fn inner(&mut self, atom: Atom, context: usize) -> Option<Bound> {
        match atom {
            Atom::Constant(n) => Some(Bound::Constant(n)),
            Atom::Scalar(value) => {
                if let Some((op, args)) = primitive(self.flow.plan, value) {
                    match (op, args.as_slice()) {
                        (NativeScalarOp::U64BitAnd, [a, b]) => {
                            for v in [a, b] {
                                if let Some(Atom::Constant(n)) =
                                    self.resolver.atom(Atom::Scalar(*v))
                                {
                                    return Some(Bound::Constant(n));
                                }
                            }
                        }
                        (NativeScalarOp::U64Sub, [a, _]) => {
                            return self.value(Atom::Scalar(*a), context);
                        }
                        (NativeScalarOp::U64Add | NativeScalarOp::U64WrappingAdd, [a, b]) => {
                            return Some(Bound::add(
                                self.value(Atom::Scalar(*a), context)?,
                                self.value(Atom::Scalar(*b), context)?,
                            ));
                        }
                        (
                            NativeScalarOp::U32ToU64
                            | NativeScalarOp::U16ToU64
                            | NativeScalarOp::U8ToU64,
                            [a],
                        ) => return self.value(Atom::Scalar(*a), context),
                        _ => {}
                    }
                }
                let facts = self.flow.plan.facts();
                if let [d] = facts.definition_sites(value).ok()?
                    && let Some(site) = d.site().map(|s| s as usize)
                    && self
                        .calls
                        .get(site)
                        .and_then(Option::as_ref)
                        .is_some_and(|c| c.supports_unsigned_matrix_length())
                    && let Action::LibraryCall {
                        operands, output, ..
                    } = self.flow.plan.blocks()[site].action()
                    && output.get() == value
                {
                    return Some(Bound::Length(operands.first()?.get()));
                }
                let maximum = self.flow.plan.values()[value as usize]
                    .scalar_kind()?
                    .unsigned_maximum()?;
                (maximum < u64::MAX).then_some(Bound::Constant(maximum))
            }
            Atom::Field(value, field) => {
                let loop_ = self.flow.forest.loops.iter().find(|l| {
                    let NativeControlLoop::Tail(t) =
                        &self.flow.plan.iterations()[l.iteration as usize]
                    else {
                        unreachable!()
                    };
                    t.carrier().get() == value
                })?;
                if loop_.iteration == self.flow.forest.loops[0].iteration {
                    return Some(Bound::Root(field));
                }
                let NativeControlLoop::Tail(tail) =
                    &self.flow.plan.iterations()[loop_.iteration as usize]
                else {
                    unreachable!()
                };
                let trip = self.prior.iter().find(|p| p.iteration == loop_.iteration);
                let tag = trip.map_or(self.tag, |p| p.tag);
                if trip.is_some_and(|p| p.field == field) {
                    let candidate = self
                        .flow
                        .forest
                        .candidate_guards(self.flow.plan, self.flow.frames, self.flow.aggregates)
                        .into_iter()
                        .find(|(id, _)| *id == loop_.iteration)?
                        .1
                        .into_iter()
                        .find(|g| Some(g.site) == trip.map(|p| p.guard))?;
                    let flow = Flow {
                        plan: self.flow.plan,
                        frames: self.flow.frames,
                        aggregates: self.flow.aggregates,
                        forest: self.flow.forest,
                        loop_,
                    };
                    // Lift a nested use to its immediate initializer in this
                    // traversal before proving the selected guard dominates it.
                    let mut at = context;
                    while self.flow.plan.blocks()[at].iteration() != Some(loop_.iteration) {
                        let id = self.flow.plan.blocks()[at].iteration()?;
                        at = self
                            .flow
                            .forest
                            .loops
                            .iter()
                            .find(|l| l.iteration == id)?
                            .initialize;
                    }
                    if !flow.edge_dominates(&candidate, at, tag) {
                        return None;
                    }
                    return self.value(Atom::Scalar(candidate.other), candidate.site);
                }
                if !self.invariant_for(loop_, field, tag)? {
                    return None;
                }
                let seeds = self.resolver.alternatives(tail.seed().get(), &[field])?;
                let mut bound = None;
                for (_, words) in seeds {
                    let current = self.value(words[0], loop_.initialize)?;
                    bound = Some(bound.map_or(current.clone(), |old| Bound::maximum(old, current)));
                }
                bound
            }
        }
    }
    fn invariant_for(&self, loop_: &Loop, field: usize, tag: Option<u64>) -> Option<bool> {
        let NativeControlLoop::Tail(tail) = &self.flow.plan.iterations()[loop_.iteration as usize]
        else {
            unreachable!()
        };
        let is_tagged = tagged(self.flow.plan, tail);
        let fields = if is_tagged {
            vec![0, field]
        } else {
            vec![field]
        };
        let next = self.resolver.alternatives(tail.next().get(), &fields)?;
        Some(next.iter().all(|(_, words)| {
            let continuing = !is_tagged || words[0] == Atom::Constant(tag.unwrap());
            !continuing || words[usize::from(is_tagged)] == Atom::Field(tail.carrier().get(), field)
        }))
    }
    pub fn invariant(&self, field: usize, tag: Option<u64>) -> Option<bool> {
        self.invariant_for(self.flow.loop_, field, tag)
    }
    pub fn invariant_value(&self, atom: Atom) -> bool {
        self.stable(atom, &mut BTreeSet::new()) == Some(true)
    }
    fn stable(&self, original: Atom, active: &mut BTreeSet<Atom>) -> Option<bool> {
        let atom = self.resolver.atom(original)?;
        if !active.insert(atom) {
            return None;
        }
        let result = (|| match atom {
            Atom::Constant(_) => Some(true),
            Atom::Field(value, field) if value == self.flow.tail().carrier().get() => {
                self.invariant(field, self.tag)
            }
            Atom::Field(value, _) => {
                let loop_ = self.flow.forest.loops.iter().find(|l| {
                    let NativeControlLoop::Tail(t) =
                        &self.flow.plan.iterations()[l.iteration as usize]
                    else {
                        unreachable!()
                    };
                    t.carrier().get() == value
                })?;
                let mut at = self.flow.loop_.parent;
                while let Some(id) = at {
                    if id == loop_.iteration {
                        return Some(true);
                    }
                    at = self
                        .flow
                        .forest
                        .loops
                        .iter()
                        .find(|l| l.iteration == id)?
                        .parent;
                }
                Some(false)
            }
            Atom::Scalar(value) => {
                let facts = self.flow.plan.facts();
                let [d] = facts.definition_sites(value).ok()? else {
                    return None;
                };
                let Some(site) = d.site().map(|s| s as usize) else {
                    return Some(true);
                };
                if self.flow.plan.blocks()[site].iteration() != Some(self.flow.loop_.iteration) {
                    let definition = self.flow.plan.blocks()[site].iteration();
                    let mut parent = self.flow.loop_.parent;
                    while let Some(id) = parent {
                        if definition == Some(id) {
                            return Some(true);
                        }
                        parent = self
                            .flow
                            .forest
                            .loops
                            .iter()
                            .find(|l| l.iteration == id)?
                            .parent;
                    }
                    return Some(definition.is_none());
                }
                if let Some((_, args)) = primitive(self.flow.plan, value) {
                    return Some(
                        args.into_iter()
                            .all(|v| self.stable(Atom::Scalar(v), active) == Some(true)),
                    );
                }
                if self
                    .calls
                    .get(site)
                    .and_then(Option::as_ref)
                    .is_some_and(|c| c.supports_unsigned_matrix_length())
                    && let Action::LibraryCall { operands, .. } =
                        self.flow.plan.blocks()[site].action()
                {
                    return self.stable(Atom::Scalar(operands.first()?.get()), active);
                }
                Some(false)
            }
        })();
        active.remove(&atom);
        result
    }
}
