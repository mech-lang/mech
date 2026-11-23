//! Every element of an exact immutable array version is zero or is a source
//! word proved below the same global bound at its original checked write.
use super::*;

fn bounded_write(plan: &Plan, graph: &Graph, site: usize, incoming: u32, bound: &Ref) -> bool {
    let Some(value) = graph.atom(&Ref::value(incoming)) else {
        return false;
    };
    if value == Atom::Word(0) {
        return true;
    } // consumer must discharge bound > 0
    let Atom::Source(value) = value else {
        return false;
    };
    let Some(owner) = graph.tails.get(&value.0) else {
        return false;
    };
    if graph.ty(&value) != Some(&Ty::Word) {
        return false;
    }
    plan.blocks().iter().enumerate().any(|(guard_site, block)| {
        if graph.charge(1).is_none() {
            return false;
        }
        let Action::Scalar(Scalar::Primitive {
            op: Op::U64Lt,
            operands,
            output,
        }) = block.action()
        else {
            return false;
        };
        if operands.len() != 2
            || graph.atom(&Ref::value(operands[0].get())) != Some(Atom::Source(value.clone()))
            || graph.atom(&Ref::value(operands[1].get())) != Some(Atom::Source(bound.clone()))
        {
            return false;
        }
        // The selected comparison and the write must belong to this exact
        // current traversal; a true guard from an earlier visit is no fact.
        true_edges(plan, graph, &Ref::value(output.get()), &BTreeSet::new())
            .into_iter()
            .any(|edge| {
                let _ = guard_site;
                edge_dominates(plan, graph, owner.header, site, edge, Some(owner.latch))
            })
    })
}

pub(super) fn prove(plan: &Plan, graph: &Graph, root: Ref, bound: &Ref) -> bool {
    struct Proof<'a> {
        plan: &'a Plan,
        graph: &'a Graph,
        bound: &'a Ref,
        active: BTreeSet<Ref>,
        proved: BTreeMap<Ref, bool>,
        constructors: usize,
        remaining: usize,
    }
    impl Proof<'_> {
        fn visit(&mut self, r: Ref) -> Option<bool> {
            self.graph.charge(1)?;
            self.remaining = self.remaining.checked_sub(1)?;
            if self.graph.ty(&r) != Some(&Ty::Array) {
                return Some(false);
            }
            if self.active.contains(&r) {
                // Only a checked initialized carrier can be an induction
                // hypothesis. An arbitrary copy/alias cycle is not a seed.
                return Some(self.graph.tails.contains_key(&r.0));
            }
            if let Some(&value) = self.proved.get(&r) {
                return Some(value);
            }
            self.active.insert(r.clone());
            let answer = (|| {
                let alternatives = self.graph.resolve(&r, &BTreeSet::new())?;
                if alternatives.is_empty() {
                    return Some(false);
                }
                if alternatives != BTreeSet::from([Atom::Source(r.clone())]) {
                    for atom in alternatives {
                        let Atom::Source(child) = atom else {
                            return Some(false);
                        };
                        if !self.visit(child)? {
                            return Some(false);
                        }
                    }
                    return Some(true);
                }
                if let Some(tail) = self.graph.tails.get(&r.0) {
                    // Both the initial content AND every subsequent version
                    // must satisfy the same invariant. No partial-init guess.
                    return Some(self.visit(r.with(tail.seed))? && self.visit(r.with(tail.next))?);
                }
                if r.1.is_empty()
                    && self.graph.charge(self.plan.collections().len()).is_some()
                    && let Some((collection, value)) = self
                        .plan
                        .collections()
                        .iter()
                        .enumerate()
                        .find(|(_, c)| c.output().get() == r.0)
                {
                    if value.kind() != mech_engine::ComprehensionKind::Matrix
                        || !matches!(value.result(), NativeCollectionResult::Materialized)
                    {
                        return Some(false);
                    }
                    let mut count = 0usize;
                    for (site, block) in self.plan.blocks().iter().enumerate() {
                        self.graph.charge(1)?;
                        if let Action::CollectionPush {
                            collection: current,
                            value,
                        } = block.action()
                            && *current as usize == collection
                        {
                            count += 1;
                            if !bounded_write(self.plan, self.graph, site, value.get(), self.bound)
                            {
                                return Some(false);
                            }
                        }
                    }
                    if count == 0 {
                        return Some(false);
                    }
                    self.constructors += 1;
                    return Some(true);
                }
                if !r.1.is_empty() {
                    return Some(false);
                }
                match self.graph.definition(r.0) {
                    Some((_, Def::Library(Primitive::SequenceWithCapacity, _))) => {
                        self.constructors += 1;
                        Some(true)
                    }
                    Some((site, Def::Library(Primitive::MatrixIndexedReplace, args)))
                        if args.len() == 3 =>
                    {
                        if !bounded_write(self.plan, self.graph, site, args[1], self.bound) {
                            return Some(false);
                        }
                        self.visit(Ref::value(args[0]))
                    }
                    _ => Some(false),
                }
            })();
            self.active.remove(&r);
            if let Some(answer) = answer {
                self.proved.insert(r, answer);
            }
            answer
        }
    }
    let mut proof = Proof {
        plan,
        graph,
        bound,
        active: BTreeSet::new(),
        proved: BTreeMap::new(),
        constructors: 0,
        remaining: LIMIT,
    };
    proof.visit(root) == Some(true) && proof.constructors != 0
}

fn reaches_without(
    plan: &Plan,
    graph: &Graph,
    start: usize,
    target: usize,
    stops: &[usize],
) -> bool {
    let mut seen = BTreeSet::new();
    let mut pending = vec![start];
    while let Some(at) = pending.pop() {
        if graph.charge(1).is_none() {
            return false;
        }
        if at == target {
            return true;
        }
        if stops.contains(&at) || !seen.insert(at) {
            continue;
        }
        pending.extend(successors(plan, at));
    }
    false
}

fn indirect_index(graph: &Graph, value: u32) -> bool {
    let r = match graph.atom(&Ref::value(value)) {
        Some(Atom::Word(_)) => return false,
        Some(Atom::Source(r)) => r,
        _ => return true,
    };
    if !r.1.is_empty() {
        return false;
    }
    let Some((_, Def::Scalar(Op::U64Add, args))) = graph.definition(r.0) else {
        return false;
    };
    args.iter().any(|&v| {
        let Some(r) = graph.source(v) else {
            return false;
        };
        r.1.is_empty()
            && matches!(
                graph.definition(r.0),
                Some((_, Def::Library(Primitive::MatrixScalarAccess, _)))
            )
    })
}

pub(super) fn positive_target(
    plan: &Plan,
    graph: &Graph,
    site: usize,
    target: &Ref,
    stops: &BTreeSet<Ref>,
) -> bool {
    let mut iteration = plan.blocks()[site].iteration();
    while let Some(current) = iteration {
        let Some(owner) = graph.tail(current) else {
            return false;
        };
        for (prior, block) in plan.blocks().iter().enumerate() {
            if graph.charge(1).is_none() {
                return false;
            }
            if prior == site {
                continue;
            }
            let Action::LibraryCall {
                template, operands, ..
            } = block.action()
            else {
                continue;
            };
            if template.primitive() != Primitive::MatrixScalarAccess
                || operands.len() != 2
                || graph.one(&Ref::value(operands[0].get()), stops)
                    != Some(Atom::Source(target.clone()))
            {
                continue;
            }
            // A read whose own check might be removed by THIS mechanism cannot
            // establish positivity. Keep an independent original source guard.
            if indirect_index(graph, operands[1].get()) {
                continue;
            }
            if reaches_without(plan, graph, owner.header, site, &[owner.latch])
                && !reaches_without(plan, graph, owner.header, site, &[owner.latch, prior])
            {
                return true;
            }
        }
        iteration = owner.parent;
    }
    false
}
