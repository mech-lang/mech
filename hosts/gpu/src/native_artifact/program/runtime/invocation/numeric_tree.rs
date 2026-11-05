//! One checked loan for a complete generated numeric Tail subtree. Child
//! initialization, backedges and returns are native CFG edges, not callbacks.
//! Only a real escape or first changed store reconciles this shared account.
use super::tail::State;
use super::{Error, Invocation, invalid};
use crate::native_artifact::iteration_abi::Progress;
use crate::native_artifact::program::prepare::numeric_tree::Proof as Tree;
use crate::native_artifact::program::prepare::readonly_numeric::Operation;
use mech_engine::native_artifact::NativeIterationAction as Action;

/// Checked independently of raw fuel. Overflow never becomes renewed credit.
fn traversal_debits(terms: impl IntoIterator<Item = (u64, u64, u64)>) -> Option<(u64, u64)> {
    terms.into_iter().try_fold((0u64, 0u64), |total, term| {
        Some((
            total.0.checked_add(term.0.checked_mul(term.1)?)?,
            total.1.checked_add(term.0.checked_mul(term.2)?)?,
        ))
    })
}

/// The selected nodes are parent-before-child. A cold active set must be one
/// exact ancestry path, never two siblings or an orphaned descendant.
fn on_path(
    mut current: usize,
    candidate: usize,
    count: usize,
    parent: impl Fn(usize) -> Option<usize>,
) -> bool {
    for _ in 0..count {
        if current == candidate {
            return true;
        }
        let Some(next) = parent(current) else {
            return false;
        };
        if next >= current {
            return false;
        }
        current = next;
    }
    false
}

impl Invocation<'_, '_, '_> {
    /// Innermost maintained parent/child marker for this exact lexical source
    /// scope. The immutable composition facts remain the sole edge inventory.
    fn numeric_tree_pending(&self, mut iteration: Option<u32>) -> Result<Option<usize>, Error> {
        let metadata = self.descriptor.metadata();
        for _ in 0..=metadata.plan.iterations().len() {
            let Some(current) = iteration else {
                return Ok(None);
            };
            let control = metadata
                .plan
                .iterations()
                .get(current as usize)
                .ok_or_else(|| invalid("numeric tree current scope is out of range"))?
                .as_ref();
            if let Some((boundary, true)) = metadata
                .compositions
                .boundary(control.initialize() as usize)
            {
                let (parent, child) = metadata
                    .compositions
                    .child_boundary(boundary)
                    .ok_or_else(|| invalid("numeric tree parent marker lost its proof"))?;
                if child.child != current || control.parent() != Some(parent.iteration) {
                    return Err(invalid("numeric tree parent marker has foreign ancestry"));
                }
                return Ok(Some(current as usize));
            }
            iteration = control.parent();
        }
        Err(invalid("numeric tree source ancestry is cyclic"))
    }

    fn numeric_tree_node_active(&self, tree: &Tree, node: usize) -> Result<bool, Error> {
        match self.read_borrowed_word(2 + tree.active(node))? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(invalid("numeric tree active word is not Boolean")),
        }
    }

    pub(super) fn end_numeric_tree(
        &mut self,
        id: usize,
        mut state: State,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let descriptor = self.descriptor;
        let metadata = descriptor.metadata();
        let region = &metadata.closed_tails.regions[id];
        let tree = region
            .numeric_tree
            .as_ref()
            .ok_or_else(|| invalid("numeric tree exit lost its proof"))?;
        let proof = region
            .numeric
            .as_ref()
            .ok_or_else(|| invalid("numeric tree exit lost its array proof"))?;
        if tree
            .nodes
            .first()
            .is_none_or(|node| node.iteration != region.iteration || node.parent.is_some())
        {
            return Err(invalid("numeric tree root differs from its owner region"));
        }
        let entry = metadata
            .visits
            .get(state.entry as usize)
            .ok_or_else(|| invalid("numeric tree entry is out of range"))?;
        let entry_iteration = match entry.progress {
            Progress::Initialize(iteration) => Some(iteration),
            _ => entry.iteration,
        };
        if self.state.control.previous != Some(state.entry)
            || self.state.control.current != entry_iteration
            || !state.same_budget(&self.state.library)
        {
            return Err(invalid(
                "numeric tree account or entry changed during execution",
            ));
        }
        let old_pending = self.numeric_tree_pending(entry_iteration)?;
        if self.parent_continuation.current() != old_pending {
            return Err(invalid("numeric tree suspended parent edge changed"));
        }
        let entry_node = tree
            .nodes
            .iter()
            .position(|node| Some(node.iteration) == entry_iteration)
            .ok_or_else(|| invalid("numeric tree loan entry has a foreign source scope"))?;
        let node = match args[2] {
            0 if site == region.tail.exit() as usize => 0,
            1 | 2 | 3 | 5 => tree
                .node(site)
                .ok_or_else(|| invalid("numeric tree cold site is outside its proof"))?,
            _ => {
                return Err(invalid(
                    "numeric tree exit mode differs from its source boundary",
                ));
            }
        };
        let current = tree
            .nodes
            .get(node)
            .ok_or_else(|| invalid("numeric tree cold node is out of range"))?;
        let previous = match args[2] {
            0 => region.tail.finish(),
            1 if site == current.tail.header() as usize => {
                match self.read_borrowed_word(2 + tree.latched(node))? {
                    0 => current.tail.initialize(),
                    1 => current.tail.latch(),
                    _ => return Err(invalid("numeric tree backedge word is not Boolean")),
                }
            }
            2 | 3 => site as u32,
            5 if proof.action(site).is_some_and(|operation| {
                matches!(
                    operation,
                    Operation::Write { .. } | Operation::Append { .. }
                )
            }) =>
            {
                site as u32
            }
            _ => {
                return Err(invalid(
                    "numeric tree cold mode has no matching source action",
                ));
            }
        };
        // All validation precedes progress/publication changes. Each body debit
        // is counted at its own header, including its terminal traversal.
        let mut compute = 0u64;
        let mut comparison = 0u64;
        for (index, selected) in tree.nodes.iter().enumerate() {
            let entry_active = on_path(entry_node, index, tree.nodes.len(), |at| {
                tree.nodes[at].parent
            });
            if self.progress[selected.iteration as usize * 2..][..2] != [0, u64::from(entry_active)]
            {
                return Err(invalid(
                    "numeric tree ordinary progress changed during its loan",
                ));
            }
            let expected = on_path(node, index, tree.nodes.len(), |at| tree.nodes[at].parent);
            let active = self.numeric_tree_node_active(tree, index)?;
            let latched = self.read_borrowed_word(2 + tree.latched(index))?;
            if active != expected || latched > 1 || (!active && latched != 0) {
                return Err(invalid(
                    "numeric tree active nodes differ from its source ancestry",
                ));
            }
            let traversals = self.read_borrowed_word(2 + tree.traversals(index))?;
            let debit = traversal_debits([(traversals, selected.compute, selected.comparison)])
                .ok_or_else(|| invalid("numeric tree traversal debit overflows"))?;
            compute = compute
                .checked_add(debit.0)
                .ok_or_else(|| invalid("numeric tree compute debit overflows"))?;
            comparison = comparison
                .checked_add(debit.1)
                .ok_or_else(|| invalid("numeric tree comparison debit overflows"))?;
        }
        if state.remaining.0.checked_sub(args[0]) != Some(compute)
            || state.remaining.1.checked_sub(args[1]) != Some(comparison)
            || (args[2] == 0
                && self.read_borrowed_word(2 + tree.traversals(0))? == 0
                && !state.partial)
        {
            return Err(invalid(
                "numeric tree fuel differs from its exact traversal debits",
            ));
        }
        let new_pending = self.numeric_tree_pending(Some(current.iteration))?;
        // Mode5 deliberately keeps the sole private owner in the root bank.
        // Internal managed aliases must not be cloned before consuming it.
        let worker_funded = state.native_fuel.is_some();
        if let Some(fuel) = state.native_fuel.take() {
            fuel.finish(compute, comparison)
                .map_err(|error| Error::Collection {
                    origin: metadata.plan.blocks()[site].origin(),
                    error,
                })?;
            self.record_worker_numeric(compute)?;
        }
        self.synchronize_numeric_tree_carriers(id, node, site, args[2])?;
        if !worker_funded {
            self.state
                .library
                .charge(compute, comparison)
                .map_err(|error| Error::Collection {
                    origin: metadata.plan.blocks()[site].origin(),
                    error,
                })?;
        }
        self.parent_continuation
            .reconcile_tree(old_pending, new_pending)?;
        for (index, selected) in tree.nodes.iter().enumerate() {
            let active = on_path(node, index, tree.nodes.len(), |at| tree.nodes[at].parent);
            self.progress[selected.iteration as usize * 2] = 0;
            self.progress[selected.iteration as usize * 2 + 1] = u64::from(active);
        }
        self.state.control.current = Some(current.iteration);
        self.state.control.previous = Some(previous);
        self.state.scalar_path = None;
        self.state.consumed = !matches!(args[2], 2 | 5);
        if args[2] == 0 && (!proof.scalar_inputs.is_empty() || !proof.scalar_outputs.is_empty()) {
            let bank = self
                .state
                .buffer_frames
                .scalar_bank_pointer(&metadata.buffer_frames)?;
            self.write_borrowed_word(2, bank)?;
        }
        if args[2] == 3 {
            self.borrowed_strings = None;
        }
        Ok(None)
    }

    fn synchronize_numeric_tree_carriers(
        &mut self,
        id: usize,
        current: usize,
        site: usize,
        mode: u64,
    ) -> Result<(), Error> {
        let metadata = self.descriptor.metadata();
        let region = &metadata.closed_tails.regions[id];
        let tree = region.numeric_tree.as_ref().unwrap();
        let proof = region.numeric.as_ref().unwrap();
        if mode == 0 {
            return self.synchronize_numeric_carrier(id, site, 0);
        }
        // Only a real ordinary header continuation needs these ancestor
        // frames. Their exact prepared physical leaves were empty at entry;
        // validate every exported value before changing any destination.
        if mode == 1 {
            for (_, bindings) in proof
                .cold_scalars
                .iter()
                .filter(|(header, _)| *header == site)
            {
                for binding in bindings {
                    if !binding
                        .kind
                        .accepts_bits(self.read_borrowed_word(2 + binding.word)?)
                    {
                        return Err(invalid("numeric cold scalar frame has invalid bits"));
                    }
                }
            }
        }
        // A loan ends at most once at an ordinary continuation. Keep prepared
        // capabilities (not Values) until this final cold header so no first
        // changed store acquires an accidental alias of its consuming owner.
        let mut publications = if mode == 1
            && proof
                .cold_aliases
                .iter()
                .any(|(_, aliases)| !aliases.is_empty())
        {
            self.state.buffer_frames.take_private_aliases(id)?
        } else {
            None
        };
        // The existing root reconciliation owns freezing/publication and its
        // prepared root scalar fields. A bounds retry is materialized below
        // after all active child carriers have regained their current owners.
        self.synchronize_numeric_carrier(
            id,
            site,
            if mode == 5 {
                5
            } else if mode == 3 {
                3
            } else {
                1
            },
        )?;
        if mode == 1 {
            for (_, bindings) in proof
                .cold_scalars
                .iter()
                .filter(|(header, _)| *header == site)
            {
                for binding in bindings {
                    let bits = self.read_borrowed_word(2 + binding.word)?;
                    self.state.buffer_frames.synchronize_scalar_binding(
                        binding.physical,
                        binding.kind,
                        bits,
                    )?;
                }
            }
            for (_, bindings) in proof
                .cold_frames
                .iter()
                .filter(|(header, _)| *header == site)
            {
                for &(physical, array) in bindings {
                    let root = &proof.arrays[array];
                    let owner = self
                        .numeric_array_owner(root)?
                        .ok_or_else(|| invalid("numeric cold frame lost its current root owner"))?
                        .clone();
                    self.state.buffer_frames.synchronize_numeric_owner(
                        physical,
                        root.schema,
                        owner,
                    )?;
                }
            }
            let mut base = 0usize;
            for (header, aliases) in &proof.cold_aliases {
                if *header == site {
                    for (index, alias) in aliases.iter().enumerate() {
                        let ticket = publications
                            .as_mut()
                            .ok_or_else(|| {
                                invalid("numeric cold continuation lost its alias capabilities")
                            })?
                            .take(base + index, alias.value)?;
                        let owner = self
                            .numeric_array_owner(&proof.arrays[alias.array])?
                            .ok_or_else(|| {
                                invalid("numeric cold continuation lost its frozen current owner")
                            })?
                            .clone();
                        self.state.arena.put_prepared(ticket, owner)?;
                    }
                }
                base = base
                    .checked_add(aliases.len())
                    .ok_or_else(|| invalid("numeric cold alias inventory overflows"))?;
            }
        }
        for (index, node) in tree.nodes.iter().enumerate().skip(1) {
            if !on_path(current, index, tree.nodes.len(), |at| tree.nodes[at].parent) {
                continue;
            }
            if mode != 3 {
                for binding in &node.scalar_bindings {
                    let bits = self.read_borrowed_word(2 + binding.word)?;
                    self.state.buffer_frames.synchronize_scalar_binding(
                        binding.physical,
                        binding.kind,
                        bits,
                    )?;
                }
            }
            if mode != 5 {
                let Some(frame) = proof.frame(node.tail.carrier().get()) else {
                    // A scalar-only child uses the already emitted ordinary
                    // scalar/aggregate homes. It has no managed owner lanes to
                    // pin or reconstruct at this cold boundary.
                    if metadata.plan.values()[node.tail.carrier().get() as usize]
                        .scalar_kind()
                        .is_some()
                        || metadata
                            .scalar_aggregates
                            .slot(node.tail.carrier())
                            .is_some()
                    {
                        continue;
                    }
                    return Err(invalid("numeric tree child carrier lost its exact frame"));
                };
                for &(physical, array) in &frame.array_bindings {
                    {
                        let owner = self
                            .numeric_array_owner(&proof.arrays[array])?
                            .ok_or_else(|| {
                                invalid("numeric tree child lost its current root owner")
                            })?
                            .clone();
                        self.state.buffer_frames.synchronize_numeric_owner(
                            physical,
                            proof.arrays[array].schema,
                            owner,
                        )?;
                    }
                }
            }
        }
        if mode == 2 {
            let Some(Operation::Read { array, .. } | Operation::Write { array, .. }) =
                proof.action(site)
            else {
                return Err(invalid(
                    "numeric tree bounds exit has no original array operation",
                ));
            };
            let Action::LibraryCall { operands, .. } = metadata.plan.blocks()[site].action() else {
                return Err(invalid(
                    "numeric tree bounds exit lost its original selected call",
                ));
            };
            let owner = self.project_numeric_array_owner(&proof.arrays[*array])?;
            self.state
                .arena
                .put(operands[0].get(), &owner, &mut self.state.control.work)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{on_path, traversal_debits};

    #[test]
    fn independent_node_debits_preserve_both_finite_accounts_and_reject_overflow() {
        assert_eq!(
            traversal_debits([(3, 7, 2), (5, 11, 0), (0, u64::MAX, u64::MAX)]),
            Some((76, 6))
        );
        assert_eq!(traversal_debits([(2, u64::MAX, 0)]), None);
        assert_eq!(traversal_debits([(1, u64::MAX, 0), (1, 1, 0)]), None);
        assert_eq!(traversal_debits([(2, 0, u64::MAX)]), None);
    }

    #[test]
    fn cold_path_cannot_activate_a_sibling_or_skip_ancestry() {
        let parents = [None, Some(0), Some(0), Some(1)];
        assert!(
            (0..4)
                .filter(|&at| on_path(3, at, 4, |i| parents[i]))
                .eq([0, 1, 3])
        );
        assert!(!on_path(3, 2, 4, |i| parents[i]));
        assert!(!on_path(1, 0, 2, |_| Some(1)));
    }
}
