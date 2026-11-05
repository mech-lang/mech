//! Bottom-up current-version contracts for structured numeric children.
//! A contract changes static eligibility only. Generated execution still runs
//! every original child traversal, operation, checked failure and fuel debit.
use super::*;

struct Summary {
    requires: BTreeSet<Key>,
    writes: BTreeSet<usize>,
    returns: BTreeSet<Key>,
}
struct NodeProof {
    traversal: Traversal,
    summary: Summary,
}

pub(super) fn validate(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    provenance: &mut Provenance<'_, '_>,
    actions: &[(usize, Operation)],
    arrays: &[Array],
    result: &ResultRecipe,
    tree: &super::super::super::numeric_tree::Proof,
) -> Result<Analysis, &'static str> {
    let domain = provenance
        .domain
        .as_ref()
        .ok_or("numeric induction lacks its identity domain")?;
    let roots = std::sync::Arc::clone(&domain.roots);
    let mut empty = domain.top.clone();
    empty.clear_where(|_| true);
    let mut proofs = BTreeMap::<usize, NodeProof>::new();
    for node in (0..tree.nodes.len()).rev() {
        let traversal = traversal(plan, tree, node)?;
        let mut requires = keys_for(&roots, tree.nodes[node].tail.carrier().get());
        let mut reads = BTreeSet::new();
        let mut writes = BTreeSet::new();
        for &site in &traversal.order {
            reads.extend(read_keys(plan, frames, actions, arrays, &roots, site)?);
            if let Some(child) = child_at(tree, node, site) {
                let summary = &proofs
                    .get(&child)
                    .ok_or("numeric induction child has no prior proof")?
                    .summary;
                reads.extend(&summary.requires);
                writes.extend(&summary.writes);
            }
            if let Some(Operation::Write { array, .. } | Operation::Append { array, .. }) =
                actions.iter().find(|(s, _)| *s == site).map(|(_, op)| op)
            {
                writes.insert(arrays[*array].root);
            }
        }
        let returns = keys_for(&roots, tree.nodes[node].tail.returned().get());
        reads.extend(&returns);
        for key in reads {
            if external(plan, tree, node, key.0) {
                // An external immutable identity cannot silently become the
                // newest version at the next traversal. Only readonly captures
                // can be assumed again; mutable state must use a carrier phi.
                if writes.contains(&roots[&key]) {
                    return Err("numeric induction mutates a captured array version");
                }
                requires.insert(key);
            }
        }
        let summary = Summary {
            requires,
            writes,
            returns,
        };
        let mut entry = empty.clone();
        for key in &summary.requires {
            entry.insert(*key, true);
        }
        let analysis = analyze(
            plan, frames, provenance, actions, arrays, tree, node, &traversal, &proofs, &roots,
            &entry,
        )?;
        verify(
            plan, frames, provenance, actions, arrays, tree, node, &analysis, &proofs, &summary,
        )?;
        proofs.insert(node, NodeProof { traversal, summary });
    }

    // Compose actual incoming aliases down the initializer ancestry. A child
    // header can follow any number of its own writes, so older aliases of its
    // may-written roots are forgotten there. Its proved current carrier is the
    // only replacement. This analysis supplies cold-home proofs, not execution.
    let mut entries = BTreeMap::from([(0usize, {
        let mut entry = empty.clone();
        for key in keys_for(&roots, tail.carrier().get()) {
            entry.insert(key, true);
        }
        for array in arrays {
            entry.insert(array.entry_key(tail), true);
        }
        entry
    })]);
    let mut inherited = BTreeMap::<usize, Vec<KnownPredicate>>::new();
    let mut combined = Analysis {
        inputs: BTreeMap::new(),
        outputs: BTreeMap::new(),
        known: Vec::new(),
    };
    for node in 0..tree.nodes.len() {
        let proof = &proofs[&node];
        let Some(entry) = entries.remove(&node) else {
            // A child whose unique initializer is proved unreachable cannot
            // execute, and neither can its descendants or cold boundaries.
            continue;
        };
        if !current(&entry, &proof.summary.requires) {
            return Err("numeric induction caller requires an older array version");
        }
        let mut analysis = analyze(
            plan,
            frames,
            provenance,
            actions,
            arrays,
            tree,
            node,
            &proof.traversal,
            &proofs,
            &roots,
            &entry,
        )?;
        verify(
            plan,
            frames,
            provenance,
            actions,
            arrays,
            tree,
            node,
            &analysis,
            &proofs,
            &proof.summary,
        )?;
        for predicate in inherited.remove(&node).unwrap_or_default() {
            analysis.known.push(predicate);
        }
        for (child, child_node) in tree.nodes.iter().enumerate() {
            if child_node.parent != Some(node) {
                continue;
            }
            let initialize = child_node.tail.initialize() as usize;
            let Some(mut entry) = analysis.inputs.get(&initialize).cloned() else {
                // The source initializer is independently proved unreachable.
                // An unreachable child has no ordinary cold continuation.
                continue;
            };
            transfer(
                plan, frames, provenance, initialize, actions, arrays, &mut entry,
            )?;
            let summary = &proofs[&child].summary;
            if !current(&entry, &summary.requires) {
                return Err("numeric induction child requires an older array version");
            }
            invalidate(&mut entry, &roots, &summary.writes, provenance);
            for key in keys_for(&roots, child_node.tail.carrier().get()) {
                entry.insert(key, true);
            }
            entries.insert(child, entry);
            let mut truths = Vec::new();
            for predicate in &analysis.known {
                if let Some(&truth) = predicate.truth.get(&initialize) {
                    truths.push(KnownPredicate {
                        iteration: predicate.iteration,
                        edges: predicate.edges.clone(),
                        truth: proofs[&child]
                            .traversal
                            .order
                            .iter()
                            .map(|&site| (site, truth))
                            .collect(),
                    });
                }
            }
            inherited.insert(child, truths);
        }
        combined.inputs.extend(analysis.inputs);
        combined.outputs.extend(analysis.outputs);
        combined.known.extend(analysis.known);
    }
    verify_tree(plan, tail, frames, &combined, actions, arrays, result, tree)?;
    Ok(combined)
}

fn keys_for(roots: &BTreeMap<Key, usize>, value: u32) -> BTreeSet<Key> {
    roots.keys().filter(|key| key.0 == value).copied().collect()
}
fn current(state: &State, requires: &BTreeSet<Key>) -> bool {
    requires.iter().all(|key| state.get(key).unwrap_or(false))
}
fn invalidate(
    state: &mut State,
    roots: &BTreeMap<Key, usize>,
    writes: &BTreeSet<usize>,
    provenance: &Provenance<'_, '_>,
) {
    if writes.iter().all(|root| {
        provenance
            .domain
            .as_ref()
            .and_then(|domain| domain.masks.get(root))
            .is_some_and(|mask| state.try_clear_mask(mask))
    }) {
        return;
    }
    state.clear_where(|key| writes.contains(&roots[key]));
}
fn child_at(
    tree: &super::super::super::numeric_tree::Proof,
    node: usize,
    site: usize,
) -> Option<usize> {
    tree.nodes.iter().enumerate().find_map(|(child, n)| {
        (n.parent == Some(node) && n.tail.initialize() as usize == site).then_some(child)
    })
}
fn external(
    plan: &Plan,
    tree: &super::super::super::numeric_tree::Proof,
    node: usize,
    value: u32,
) -> bool {
    let scope = match plan.value_scopes()[value as usize] {
        Scope::Immutable { iteration } => iteration,
        Scope::Carrier { iteration } => Some(iteration),
    };
    let Some(mut owner) = tree.nodes.iter().position(|n| Some(n.iteration) == scope) else {
        return true;
    };
    loop {
        if owner == node {
            return false;
        }
        let Some(parent) = tree.nodes[owner].parent else {
            return true;
        };
        owner = parent;
    }
}
fn traversal(
    plan: &Plan,
    tree: &super::super::super::numeric_tree::Proof,
    node: usize,
) -> Result<Traversal, &'static str> {
    let tail = &tree.nodes[node].tail;
    let mut next = BTreeMap::new();
    for &(site, owner) in &tree.sites {
        if owner != node {
            continue;
        }
        let successors = if site == tail.latch() as usize || site == tail.finish() as usize {
            Vec::new()
        } else if let Some(child) = child_at(tree, node, site) {
            vec![tree.nodes[child].tail.exit() as usize]
        } else {
            NativeProgramFacts::control_successors(plan.blocks()[site].terminator())
                .into_iter()
                .flatten()
                .map(|s| s as usize)
                .collect()
        };
        next.insert(site, successors);
    }
    Traversal::from_next(tail.header() as usize, next)
}

fn read_keys(
    plan: &Plan,
    frames: &NativeBufferFrames,
    actions: &[(usize, Operation)],
    arrays: &[Array],
    roots: &BTreeMap<Key, usize>,
    site: usize,
) -> Result<BTreeSet<Key>, &'static str> {
    let mut keys = BTreeSet::new();
    let mut add = |key| {
        if roots.contains_key(&key) {
            keys.insert(key);
        }
    };
    match actions.iter().find(|(s, _)| *s == site).map(|(_, op)| op) {
        Some(Operation::Frame) => {
            for (_, source) in numeric_fields::transfers(plan, frames, site)? {
                add(source);
            }
        }
        Some(Operation::ArrayCopy) => {
            let Action::Scalar(Scalar::Copy { source, .. }) = plan.blocks()[site].action() else {
                return Err("numeric induction copy lost its source");
            };
            add((source.get(), None));
        }
        Some(
            operation @ (Operation::Read { array, .. }
            | Operation::Write { array, .. }
            | Operation::Append { array, .. }
            | Operation::Length { array, .. }),
        ) => {
            let Action::LibraryCall { operands, .. } = plan.blocks()[site].action() else {
                return Err("numeric induction array use lost its source");
            };
            if !matches!(operation, Operation::Length { .. }) || arrays[*array].appends {
                add((operands[0].get(), None));
            }
        }
        _ => {}
    }
    Ok(keys)
}

#[allow(clippy::too_many_arguments)]
fn analyze(
    plan: &Plan,
    frames: &NativeBufferFrames,
    provenance: &mut Provenance<'_, '_>,
    actions: &[(usize, Operation)],
    arrays: &[Array],
    tree: &super::super::super::numeric_tree::Proof,
    node: usize,
    traversal: &Traversal,
    proofs: &BTreeMap<usize, NodeProof>,
    roots: &BTreeMap<Key, usize>,
    entry: &State,
) -> Result<Analysis, &'static str> {
    let mut analysis = pass(
        plan, frames, provenance, actions, arrays, tree, node, traversal, proofs, roots, entry,
        None,
    )?;
    for edges in traversal.predicates(provenance.facts) {
        let refined = pass(
            plan,
            frames,
            provenance,
            actions,
            arrays,
            tree,
            node,
            traversal,
            proofs,
            roots,
            entry,
            Some(&edges),
        )?;
        analysis.strengthen(refined);
    }
    Ok(analysis)
}

#[allow(clippy::too_many_arguments)]
fn pass(
    plan: &Plan,
    frames: &NativeBufferFrames,
    provenance: &mut Provenance<'_, '_>,
    actions: &[(usize, Operation)],
    arrays: &[Array],
    tree: &super::super::super::numeric_tree::Proof,
    node: usize,
    traversal: &Traversal,
    proofs: &BTreeMap<usize, NodeProof>,
    roots: &BTreeMap<Key, usize>,
    entry: &State,
    partition: Option<&Vec<(usize, (usize, usize))>>,
) -> Result<Analysis, &'static str> {
    let mut inputs = BTreeMap::from([((traversal.header, 0u8), entry.clone())]);
    let mut outputs = BTreeMap::new();
    for &site in &traversal.order {
        for truth in 0..if partition.is_some() { 3 } else { 1 } {
            let Some(mut state) = inputs.get(&(site, truth)).cloned() else {
                continue;
            };
            transfer(plan, frames, provenance, site, actions, arrays, &mut state)?;
            if let Some(child) = child_at(tree, node, site) {
                let summary = &proofs[&child].summary;
                let valid = current(&state, &summary.requires);
                invalidate(&mut state, roots, &summary.writes, provenance);
                for key in &summary.returns {
                    state.insert(*key, valid);
                }
            }
            for &next in &traversal.next[&site] {
                let mut next_truth = truth;
                if let Some((_, (no, yes))) =
                    partition.and_then(|edges| edges.iter().find(|(branch, _)| *branch == site))
                {
                    if no != yes {
                        let edge = if next == *no {
                            1
                        } else if next == *yes {
                            2
                        } else {
                            return Err("numeric induction predicate leaves its source branch");
                        };
                        if truth != 0 && truth != edge {
                            continue;
                        }
                        next_truth = edge;
                    }
                }
                if let Some(merged) = inputs.get_mut(&(next, next_truth)) {
                    merged.meet(&state);
                } else {
                    inputs.insert((next, next_truth), state.clone());
                }
            }
            outputs.insert((site, truth), state);
        }
    }
    let known = partition
        .map(|edges| {
            let mut truths = BTreeMap::<usize, BTreeSet<u8>>::new();
            for &(site, truth) in inputs.keys() {
                truths.entry(site).or_default().insert(truth);
            }
            KnownPredicate {
                iteration: tree.nodes[node].iteration,
                edges: edges.clone(),
                truth: truths
                    .into_iter()
                    .filter_map(|(site, truths)| {
                        (truths.len() == 1 && !truths.contains(&0))
                            .then(|| (site, truths.contains(&2)))
                    })
                    .collect(),
            }
        })
        .into_iter()
        .collect();
    let collapse = |bank: BTreeMap<(usize, u8), State>| {
        let mut result = BTreeMap::<usize, State>::new();
        for ((site, _), state) in bank {
            if let Some(current) = result.get_mut(&site) {
                current.meet(&state);
            } else {
                result.insert(site, state);
            }
        }
        result
    };
    Ok(Analysis {
        inputs: collapse(inputs),
        outputs: collapse(outputs),
        known,
    })
}

#[allow(clippy::too_many_arguments)]
fn verify(
    plan: &Plan,
    frames: &NativeBufferFrames,
    provenance: &mut Provenance<'_, '_>,
    actions: &[(usize, Operation)],
    arrays: &[Array],
    tree: &super::super::super::numeric_tree::Proof,
    node: usize,
    analysis: &Analysis,
    proofs: &BTreeMap<usize, NodeProof>,
    summary: &Summary,
) -> Result<(), &'static str> {
    let tail = &tree.nodes[node].tail;
    verify_tree(
        plan,
        tail,
        frames,
        analysis,
        actions,
        arrays,
        &ResultRecipe::Scalar,
        tree,
    )?;
    if let Some(state) = analysis.outputs.get(&(tail.latch() as usize)) {
        if !current(state, &summary.requires) {
            return Err("numeric induction recursive edge loses a required current capture");
        }
    }
    if let Some(state) = analysis.outputs.get(&(tail.finish() as usize)) {
        if !current(state, &summary.returns) {
            return Err("numeric induction child returns an older array version");
        }
    }
    for (&site, input) in &analysis.inputs {
        if let Some(child) = child_at(tree, node, site) {
            let mut state = input.clone();
            transfer(plan, frames, provenance, site, actions, arrays, &mut state)?;
            if !current(&state, &proofs[&child].summary.requires) {
                return Err("numeric induction child call requires an older array version");
            }
        }
    }
    Ok(())
}
