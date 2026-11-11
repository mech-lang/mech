//! Exact private words consumed by the existing numeric cold bridges.
//! This narrows SSA lifetimes, without changing the checked runtime recipes.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::compiler::backend;
use crate::native_artifact::program::prepare::{numeric_tree, readonly_numeric};
use std::collections::BTreeSet;

/// Immutable same-plan lexical ancestry. Runtime still validates the generated
/// active flags, all node debits, and the source boundary independently.
pub(super) fn carrier_words(
    tree: &numeric_tree::Proof,
    site: usize,
) -> Result<BTreeSet<usize>, NativeCompileError> {
    let mut current = if site == tree.nodes[0].tail.exit() as usize {
        Some(0)
    } else {
        Some(
            tree.node(site)
                .ok_or_else(|| backend("private liveness lost numeric scope"))?,
        )
    };
    let mut words = BTreeSet::new();
    while let Some(index) = current {
        let node = &tree.nodes[index];
        words.extend(node.scalar_bindings.iter().map(|binding| binding.word));
        current = node.parent;
    }
    Ok(words)
}

pub(super) fn at(
    proof: &readonly_numeric::Proof,
    tree: Option<&numeric_tree::Proof>,
    site: usize,
    exit: usize,
    projected_debits: bool,
) -> Result<BTreeSet<usize>, NativeCompileError> {
    // Both the cold root-carrier reconciliation and successful scalar-bank
    // export consume these leaves. Terminal result leaves are success-only.
    let mut words = proof
        .scalar_inputs
        .iter()
        .map(|binding| binding.word)
        .collect::<BTreeSet<_>>();
    if site == exit {
        words.extend(proof.scalar_outputs.iter().map(|binding| binding.word));
    }
    for (_, bindings) in proof
        .cold_scalars
        .iter()
        .filter(|(header, _)| *header == site)
    {
        words.extend(bindings.iter().map(|binding| binding.word));
    }
    // A loan ends before any ordinary consumer can reuse a raw view. Runtime
    // freezes private arrays from their checked owners, reading only an append
    // length. Base pointers, dimensions, COW flags, capacities and the bank
    // pointer have no cold consumer and remain ordinary hot SSA dependencies.
    words.extend(
        proof
            .arrays
            .iter()
            .enumerate()
            .filter(|(_, array)| array.appends)
            .map(|(array, _)| proof.view(array) + 1),
    );
    if let Some(tree) = tree {
        words.extend(carrier_words(tree, site)?);
        // Completed siblings still contribute debit history. Keep the root
        // count, weighted child totals and every inactive flag even when their
        // source carriers are dead; unused counter lanes are constant zero.
        words.extend(
            (tree.counters..tree.bank())
                .filter(|&word| tree.live_counter_word(word, projected_debits)),
        );
    }
    Ok(words)
}
