//! Whole nested Tail inventory for one native numeric phase.
//! This uses canonical loop ancestry and the actual CFG, without source names.
//! Every declared backedge is cut for per-traversal validation; any remaining
//! cycle, collection, activation, or external edge declines the phase.
use mech_engine::native_artifact::NativeScalarOp;
use mech_engine::native_artifact::{
    NativeBufferFrames, NativeControlLoop, NativeIterationAction as Action,
    NativeIterationTerminator as Term, NativeMatchAction as Scalar,
    NativeMatchTerminator as Branch, NativePreparedLibraryCall, NativeProgramArtifactPlan as Plan,
    NativeProgramFacts, NativeScalarAggregates, NativeScalarKind, NativeTailLoop,
};
use std::collections::{BTreeMap, BTreeSet};
mod atoms;
mod bounds;
mod cold;
mod flow;
mod homes;
mod intervals;
mod schedule;
mod words;
pub(in crate::native_artifact) use bounds::Bound;
pub(in crate::native_artifact) use cold::Child as ColdChild;
pub(in crate::native_artifact) use homes::boundary_work;
pub(in crate::native_artifact) use schedule::Schedule;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(in crate::native_artifact) enum Coverage {
    Complete,
    ScalarCold,
    Intervals,
}

/// Candidate source comparison. A bounded-trip proof must additionally prove
/// recurrence, entry seed, the selected edge and an invariant opposite operand.
/// Merely identifying this coordinate never removes a source check.
#[derive(Debug)]
pub(in crate::native_artifact) struct Guard {
    pub site: usize,
    pub predicate: u32,
    pub field: usize,
    /// The compared value is the current field plus this checked constant.
    /// Wrapping affine comparisons do not establish an ordered upper bound.
    pub offset: u64,
    pub other: u32,
    pub op: NativeScalarOp,
    pub yes: u32,
    pub no: u32,
}

#[derive(Debug)]
pub(in crate::native_artifact) struct Loop {
    pub iteration: u32,
    pub header: usize,
    pub latch: usize,
    pub initialize: usize,
    pub finish: usize,
    pub exit: usize,
    pub parent: Option<u32>,
    pub sites: Vec<usize>,
}

#[derive(Debug)]
pub(in crate::native_artifact) struct Forest {
    pub loops: Vec<Loop>,
    pub cold: Vec<ColdChild>,
    pub sites: BTreeSet<usize>,
    /// A current traversal order, cutting each declared latch backedge.
    pub order: Vec<usize>,
    /// Actual native exits; unlike a summary, these have no native successor.
    pub stops: BTreeSet<usize>,
    /// The complete root traversal paid before a partial interval starts.
    pub prepaid_sites: BTreeSet<usize>,
}

impl Forest {
    pub(in crate::native_artifact) fn candidate_guards(
        &self,
        plan: &Plan,
        frames: &NativeBufferFrames,
        aggregates: &NativeScalarAggregates,
    ) -> Vec<(u32, Vec<Guard>)> {
        let facts = plan.facts();
        let resolver = atoms::Resolver {
            plan,
            frames,
            aggregates,
            forest: self,
        };
        self.loops
            .iter()
            .map(|loop_| {
                let NativeControlLoop::Tail(tail) = &plan.iterations()[loop_.iteration as usize]
                else {
                    unreachable!("Tail forest")
                };
                let words = words::Words {
                    plan,
                    facts: &facts,
                    frames,
                    aggregates,
                    tail,
                    iteration: loop_.iteration,
                };
                let guards = loop_
                    .sites
                    .iter()
                    .flat_map(|&site| {
                        let operands = match plan.blocks()[site].terminator() {
                            Term::Scalar(Branch::Branch { condition, .. }) => vec![condition],
                            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                                vec![left, right]
                            }
                            _ => Vec::new(),
                        };
                        operands
                            .into_iter()
                            .filter_map(|condition| {
                                let (no, yes) =
                                    facts.boolean_branch_edges(site as u32, condition)?;
                                let mut predicate = condition;
                                while let Some(source) = facts.unique_identity_copy(predicate) {
                                    predicate = source;
                                }
                                let [definition] = facts.definition_sites(predicate.get()).ok()?
                                else {
                                    return None;
                                };
                                let at = definition.site()? as usize;
                                if plan.blocks()[at].iteration() != Some(loop_.iteration) {
                                    return None;
                                }
                                let Action::Scalar(Scalar::Primitive {
                                    op,
                                    operands,
                                    output,
                                }) = plan.blocks()[at].action()
                                else {
                                    return None;
                                };
                                if *output != predicate
                                    || operands.len() != 2
                                    || !matches!(
                                        op,
                                        NativeScalarOp::U64Eq
                                            | NativeScalarOp::U64Ne
                                            | NativeScalarOp::U64Lt
                                            | NativeScalarOp::U64Le
                                            | NativeScalarOp::U64Gt
                                            | NativeScalarOp::U64Ge
                                    )
                                    || plan.values()[operands[0].get() as usize].scalar_kind()
                                        != Some(NativeScalarKind::U64)
                                {
                                    return None;
                                }
                                let (field, offset) =
                                    if let Some(field) = words.carrier(operands[0].get()) {
                                        (field, 0)
                                    } else {
                                        let [definition] =
                                            facts.definition_sites(operands[0].get()).ok()?
                                        else {
                                            return None;
                                        };
                                        let at = definition.site()? as usize;
                                        if plan.blocks()[at].iteration() != Some(loop_.iteration) {
                                            return None;
                                        }
                                        let Action::Scalar(Scalar::Primitive {
                                            op: NativeScalarOp::U64Add,
                                            operands: add,
                                            output,
                                        }) = plan.blocks()[at].action()
                                        else {
                                            return None;
                                        };
                                        if *output != operands[0] || add.len() != 2 {
                                            return None;
                                        }
                                        let atoms::Atom::Constant(offset) =
                                            resolver.atom(atoms::Atom::Scalar(add[1].get()))?
                                        else {
                                            return None;
                                        };
                                        (words.carrier(add[0].get())?, offset)
                                    };
                                Some(Guard {
                                    site,
                                    predicate: predicate.get(),
                                    field,
                                    offset,
                                    other: operands[1].get(),
                                    op: *op,
                                    yes,
                                    no,
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect();
                (loop_.iteration, guards)
            })
            .collect()
    }

    /// This is an effect inventory, not permission to borrow a view. The
    /// enclosing numeric proof must still establish every array's provenance
    /// and current version. Descendants may read; mutations remain in the root.
    pub(in crate::native_artifact) fn validate_child_actions(
        &self,
        plan: &Plan,
        frames: &NativeBufferFrames,
        aggregates: &NativeScalarAggregates,
        calls: &[Option<NativePreparedLibraryCall>],
    ) -> Result<usize, &'static str> {
        let mut reads = 0;
        for child in self.loops.iter().skip(1) {
            for &site in &child.sites {
                if frames.action(site).is_some() || aggregates.action(site).is_some() {
                    continue;
                }
                match plan.blocks()[site].action() {
                    Action::Scalar(Scalar::None | Scalar::Primitive { .. }) => {}
                    Action::Scalar(Scalar::Copy { source, output })
                        if plan.values()[output.get() as usize].scalar_kind().is_some()
                            || plan.values()[source.get() as usize].definition()
                                == plan.values()[output.get() as usize].definition() => {}
                    Action::LibraryCall { template, .. } => {
                        let call = calls
                            .get(site)
                            .and_then(Option::as_ref)
                            .filter(|call| call.template() == template)
                            .ok_or("numeric child lacks its exact selected factory")?;
                        if call.supports_unsigned_matrix_element() {
                            reads += 1;
                        } else if !call.supports_unsigned_matrix_length() {
                            return Err("numeric child has a mutation or unsupported effect");
                        }
                    }
                    _ => return Err("numeric child has an unsupported state action"),
                }
            }
        }
        Ok(reads)
    }

    pub(in crate::native_artifact) fn derive(
        plan: &Plan,
        root: &NativeTailLoop,
    ) -> Result<Self, &'static str> {
        Self::derive_inventory(plan, root, Coverage::Complete)
    }

    pub(in crate::native_artifact) fn with_cold_children(
        plan: &Plan,
        root: &NativeTailLoop,
    ) -> Result<Self, &'static str> {
        Self::derive_inventory(plan, root, Coverage::ScalarCold)
    }

    pub(in crate::native_artifact) fn intervals(
        plan: &Plan,
        root: &NativeTailLoop,
    ) -> Result<Self, &'static str> {
        Self::derive_inventory(plan, root, Coverage::Intervals)
    }

    pub fn successors(&self, plan: &Plan, site: usize) -> Vec<usize> {
        if self.stops.contains(&site) {
            Vec::new()
        } else {
            cold::successors(&self.cold, plan, site)
        }
    }

    fn derive_inventory(
        plan: &Plan,
        root: &NativeTailLoop,
        mode: Coverage,
    ) -> Result<Self, &'static str> {
        let root_id = plan
            .iterations()
            .iter()
            .position(|loop_| {
                matches!(loop_, NativeControlLoop::Tail(tail) if tail.header() == root.header())
            })
            .ok_or("numeric phase root is absent")? as u32;
        let mut members = BTreeSet::from([root_id]);
        let cold = if mode == Coverage::ScalarCold {
            cold::children(plan, root_id)?
        } else {
            Vec::new()
        };
        let (mut stops, prepaid_sites) = if mode == Coverage::Intervals {
            intervals::boundaries(plan, root_id)?
        } else {
            (BTreeSet::new(), BTreeSet::new())
        };
        let successors = |site| {
            if stops.contains(&site) {
                Vec::new()
            } else {
                cold::successors(&cold, plan, site)
            }
        };
        while mode == Coverage::Complete {
            let before = members.len();
            for (id, loop_) in plan.iterations().iter().enumerate() {
                if loop_
                    .as_ref()
                    .parent()
                    .is_some_and(|p| members.contains(&p))
                {
                    if !matches!(loop_, NativeControlLoop::Tail(_)) {
                        return Err("numeric phase contains a non-Tail child");
                    }
                    members.insert(id as u32);
                }
            }
            if members.len() == before {
                break;
            }
        }
        let mut sites = BTreeSet::new();
        let mut pending = vec![root.header()];
        while let Some(site) = pending.pop() {
            if site == root.exit() || !sites.insert(site as usize) {
                continue;
            }
            let block = &plan.blocks()[site as usize];
            if !block.iteration().is_some_and(|id| members.contains(&id))
                || plan.activation_boundary() == Some(site)
            {
                return Err("numeric phase crosses an external iteration or activation");
            }
            if !matches!(
                block.terminator(),
                Term::TailHeader { .. }
                    | Term::Scalar(
                        Branch::Jump(_)
                            | Branch::Branch { .. }
                            | Branch::LiteralBranch { .. }
                            | Branch::Reject
                            | Branch::IntegrityFailure
                    )
            ) {
                return Err("numeric phase has an unsupported terminator");
            }
            pending.extend(successors(site as usize).into_iter().map(|s| s as u32));
        }
        let mut loops = Vec::new();
        for iteration in members {
            let NativeControlLoop::Tail(tail) = &plan.iterations()[iteration as usize] else {
                unreachable!("only Tail descendants were admitted");
            };
            if [tail.header(), tail.latch(), tail.finish()]
                .iter()
                .any(|s| {
                    !sites.contains(&(*s as usize))
                        && (mode != Coverage::Intervals || *s == tail.header())
                })
                || (iteration != root_id
                    && [tail.initialize(), tail.exit()]
                        .iter()
                        .any(|s| !sites.contains(&(*s as usize))))
            {
                return Err("numeric phase has an incomplete nested invocation");
            }
            loops.push(Loop {
                iteration,
                header: tail.header() as usize,
                latch: tail.latch() as usize,
                initialize: tail.initialize() as usize,
                finish: tail.finish() as usize,
                exit: tail.exit() as usize,
                parent: tail.parent(),
                sites: if mode == Coverage::Intervals {
                    &prepaid_sites
                } else {
                    &sites
                }
                .iter()
                .copied()
                .filter(|&s| plan.blocks()[s].iteration() == Some(iteration))
                .collect(),
            });
        }
        let backedges = loops
            .iter()
            .map(|loop_| (loop_.latch, loop_.header))
            .collect::<BTreeSet<_>>();
        let mut incoming = sites
            .iter()
            .map(|&s| (s, 0usize))
            .collect::<BTreeMap<_, _>>();
        for &site in &sites {
            for next in successors(site) {
                if next != root.exit() as usize && !backedges.contains(&(site, next)) {
                    *incoming
                        .get_mut(&next)
                        .ok_or("numeric phase has an unaccounted outgoing edge")? += 1;
                }
            }
        }
        let mut ready = incoming
            .iter()
            .filter_map(|(&s, &n)| (n == 0).then_some(s))
            .collect::<Vec<_>>();
        let mut order = Vec::new();
        while let Some(site) = ready.pop() {
            order.push(site);
            for next in successors(site) {
                if next == root.exit() as usize || backedges.contains(&(site, next)) {
                    continue;
                }
                let count = incoming.get_mut(&next).unwrap();
                *count -= 1;
                if *count == 0 {
                    ready.push(next);
                }
            }
        }
        if order.len() != sites.len() {
            return Err("numeric phase has an undeclared cycle");
        }
        stops.retain(|site| sites.contains(site));
        if mode == Coverage::Intervals && stops.is_empty() {
            return Err("numeric interval has no reachable cold boundary");
        }
        Ok(Self {
            loops,
            cold,
            sites,
            order,
            stops,
            prepaid_sites,
        })
    }
}

/// The first-change bridge may spend account credit. Initially, mutations are
/// supported only after all children in a root traversal have returned. Thus no
/// not-yet-debited child can depend on pre-bridge funding after that cold call.
pub(in crate::native_artifact) fn validate_mutation_order(
    plan: &Plan,
    forest: &Forest,
    actions: &[(usize, super::numeric_plan::Operation)],
) -> Result<(), &'static str> {
    use super::numeric_plan::Operation;
    let latches = forest
        .loops
        .iter()
        .map(|l| l.latch)
        .collect::<BTreeSet<_>>();
    let child_entries = forest
        .loops
        .iter()
        .skip(1)
        .map(|l| l.initialize)
        .collect::<BTreeSet<_>>();
    for &(site, ref action) in actions {
        if !matches!(action, Operation::Write { .. } | Operation::Append { .. }) {
            continue;
        }
        if plan.blocks()[site].iteration() != Some(forest.loops[0].iteration) {
            return Err("phase child mutates a buffer");
        }
        let mut seen = BTreeSet::new();
        let mut pending = vec![site];
        while let Some(at) = pending.pop() {
            if child_entries.contains(&at) {
                return Err("phase child follows a fallible mutation bridge");
            }
            if !seen.insert(at) || latches.contains(&at) {
                continue;
            }
            pending.extend(
                NativeProgramFacts::control_successors(plan.blocks()[at].terminator())
                    .into_iter()
                    .flatten()
                    .map(|s| s as usize)
                    .filter(|s| forest.sites.contains(s)),
            );
        }
    }
    Ok(())
}
