//! Structured numeric Tail subtrees. Every edge belongs to a validated source
//! loop; an initializer/return is an internal CFG edge, never a second loan.
use super::readonly_numeric::ScalarBinding;
use mech_engine::native_artifact::{
    NativeControlLoop, NativeIterationTerminator as Term, NativeMatchTerminator as Branch,
    NativeProgramArtifactPlan as Plan, NativeProgramFacts, NativeTailLoop,
};
use std::collections::{BTreeMap, BTreeSet};

/// The counters are private transport, not source values or entry authority.
/// Traversals price each loop independently, including its terminal traversal.
pub(in crate::native_artifact) const NODE_WORDS: usize = 3;

#[derive(Clone, Debug, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Node {
    pub iteration: u32,
    pub parent: Option<usize>,
    pub tail: NativeTailLoop,
    pub compute: u64,
    pub comparison: u64,
    pub scalar_bindings: Vec<ScalarBinding>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Proof {
    /// Parent before children. The first node is the one admitted owner loan.
    pub nodes: Vec<Node>,
    pub sites: Vec<(usize, usize)>,
    /// Relative to the unified numeric shadow; fixed and independently bounded.
    pub counters: usize,
}
impl Proof {
    pub fn node(&self, site: usize) -> Option<usize> {
        self.sites
            .iter()
            .find(|(at, _)| *at == site)
            .map(|(_, n)| *n)
    }
    pub fn words(&self) -> usize {
        self.nodes.len() * NODE_WORDS + 1
    }
    /// Refreshed after cold Rust mutation; never deserialized access authority.
    pub fn bank(&self) -> usize {
        self.counters + self.nodes.len() * NODE_WORDS
    }
    pub fn traversals(&self, node: usize) -> usize {
        self.counters + node * NODE_WORDS
    }
    pub fn active(&self, node: usize) -> usize {
        self.traversals(node) + 1
    }
    /// Whether the current activation has taken its declared backedge. Tail
    /// progress itself remains exactly zero, as in ordinary source execution.
    pub fn latched(&self, node: usize) -> usize {
        self.traversals(node) + 2
    }
}

/// Discover only a closed, properly nested Tail tree. No collection, arbitrary
/// cycle, activation boundary, cross-child jump, or ambiguous return is admitted.
/// The numeric/alias proof subsequently checks every action in this inventory.
pub(in crate::native_artifact) struct Discovery<'a> {
    plan: &'a Plan,
}
impl<'a> Discovery<'a> {
    /// One validation covers every candidate borrowed from this immutable plan.
    /// Discovery still checks every candidate's own structured edge inventory.
    pub fn new(plan: &'a Plan) -> Result<Self, &'static str> {
        plan.validate_current_visits()?;
        Ok(Self { plan })
    }

    pub fn discover(&self, root: &NativeTailLoop) -> Result<Proof, &'static str> {
        discover(self.plan, root)
    }
}

fn discover(plan: &Plan, root: &NativeTailLoop) -> Result<Proof, &'static str> {
    let root_iteration = plan
        .iterations()
        .iter()
        .position(
            |control| matches!(control, NativeControlLoop::Tail(t) if t.header() == root.header()),
        )
        .ok_or("numeric tree root is missing")? as u32;
    let mut nodes = vec![Node {
        iteration: root_iteration,
        parent: None,
        tail: root.clone(),
        compute: 0,
        comparison: 0,
        scalar_bindings: Vec::new(),
    }];
    let mut selected = BTreeMap::from([(root_iteration, 0usize)]);
    for iteration in 0..plan.iterations().len() {
        let control = &plan.iterations()[iteration];
        let mut ancestor = control.as_ref().parent();
        while let Some(parent) = ancestor {
            if parent == root_iteration {
                let NativeControlLoop::Tail(tail) = control else {
                    return Err("numeric tree contains a non-Tail child");
                };
                let parent = selected
                    .get(&tail.parent().ok_or("numeric tree child lacks parent")?)
                    .copied()
                    .ok_or("numeric tree ancestry is not ordered")?;
                selected.insert(iteration as u32, nodes.len());
                nodes.push(Node {
                    iteration: iteration as u32,
                    parent: Some(parent),
                    tail: tail.clone(),
                    compute: 0,
                    comparison: 0,
                    scalar_bindings: Vec::new(),
                });
                break;
            }
            ancestor = plan
                .iterations()
                .get(parent as usize)
                .ok_or("numeric tree ancestor is missing")?
                .as_ref()
                .parent();
        }
    }
    if nodes.len() < 2 {
        return Err("numeric tree has no child");
    }
    let mut sites = BTreeMap::new();
    let mut pending = vec![root.header()];
    while let Some(site) = pending.pop() {
        if site == root.exit() || sites.contains_key(&(site as usize)) {
            continue;
        }
        let block = plan
            .blocks()
            .get(site as usize)
            .ok_or("numeric tree edge is out of range")?;
        let node = selected
            .get(
                &block
                    .iteration()
                    .ok_or("numeric tree leaves its root scope")?,
            )
            .copied()
            .ok_or("numeric tree enters a foreign loop")?;
        if plan.activation_boundary() == Some(site) {
            return Err("numeric tree crosses an activation boundary");
        }
        match block.terminator() {
            Term::TailHeader { iteration, .. } if iteration == nodes[node].iteration => {}
            Term::Scalar(
                Branch::Jump(_)
                | Branch::Branch { .. }
                | Branch::LiteralBranch { .. }
                | Branch::Reject
                | Branch::IntegrityFailure,
            ) => {}
            _ => return Err("numeric tree has an unsupported terminator"),
        }
        sites.insert(site as usize, node);
        for next in NativeProgramFacts::control_successors(block.terminator())
            .into_iter()
            .flatten()
        {
            if next == root.exit() {
                if site != root.finish() {
                    return Err("numeric tree leaves outside its root return");
                }
            } else {
                let target = plan
                    .blocks()
                    .get(next as usize)
                    .and_then(|block| block.iteration())
                    .and_then(|iteration| selected.get(&iteration))
                    .copied()
                    .ok_or("numeric tree edge enters a foreign scope")?;
                if target != node
                    && !((nodes[target].parent == Some(node)
                        && site == nodes[target].tail.initialize()
                        && next == nodes[target].tail.header())
                        || (nodes[node].parent == Some(target)
                            && site == nodes[node].tail.finish()
                            && next == nodes[node].tail.exit()))
                {
                    return Err("numeric tree scope edge is not an exact child call or return");
                }
            }
            pending.push(next);
        }
    }
    for (node, entry) in nodes.iter().enumerate() {
        if ![entry.tail.header(), entry.tail.latch(), entry.tail.finish()]
            .iter()
            .all(|s| sites.get(&(*s as usize)) == Some(&node))
        {
            return Err("numeric tree has incomplete loop boundaries");
        }
        if node > 0
            && ![entry.tail.initialize(), entry.tail.exit()]
                .iter()
                .all(|s| sites.get(&(*s as usize)) == entry.parent.as_ref())
        {
            return Err("numeric tree child does not return to its exact parent");
        }
    }
    // Removing exactly the source-declared latches must leave a DAG. That
    // bounds each exclusive loop traversal and rejects unstructured cycles.
    let latches = nodes
        .iter()
        .map(|n| (n.tail.latch() as usize, n.tail.header() as usize))
        .collect::<BTreeSet<_>>();
    let mut incoming = sites
        .keys()
        .map(|s| (*s, 0usize))
        .collect::<BTreeMap<_, _>>();
    for &site in sites.keys() {
        for next in NativeProgramFacts::control_successors(plan.blocks()[site].terminator())
            .into_iter()
            .flatten()
        {
            if let Some(count) = incoming.get_mut(&(next as usize)) {
                if !latches.contains(&(site, next as usize)) {
                    *count += 1;
                }
            }
        }
    }
    let mut ready = incoming
        .iter()
        .filter_map(|(s, n)| (*n == 0).then_some(*s))
        .collect::<Vec<_>>();
    let mut count = 0;
    while let Some(site) = ready.pop() {
        count += 1;
        for next in NativeProgramFacts::control_successors(plan.blocks()[site].terminator())
            .into_iter()
            .flatten()
        {
            if !latches.contains(&(site, next as usize)) {
                if let Some(n) = incoming.get_mut(&(next as usize)) {
                    *n -= 1;
                    if *n == 0 {
                        ready.push(next as usize);
                    }
                }
            }
        }
    }
    if count != sites.len() {
        return Err("numeric tree has an undeclared cycle");
    }
    Ok(Proof {
        nodes,
        sites: sites.into_iter().collect(),
        counters: 0,
    })
}
