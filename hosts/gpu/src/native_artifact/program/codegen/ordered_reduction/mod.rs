//! A source-derived ordered U32 minimum over paired immutable affine reads.
//! The complete-child witness owns safety and accounting; this recipe describes
//! only equivalent values. Unsupported expressions keep shared scalar emission.
use super::traversal_facts::{Candidate, Completed, Graph, Node, Window};
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::program::prepare::readonly_numeric::Proof;
use mech_engine::native_artifact::{NativeScalarOp as Op, NativeTailLoop};
mod cost;
mod emit;
mod escape;
mod normalize;
use normalize::{Cost, Item, Normalize};

#[derive(Clone, Debug, serde::Serialize)]
pub(super) struct Reduction {
    pub candidate: Candidate,
    pub best: usize,
    pub item: usize,
    cost: Cost,
    selected: Item,
    pub reads: Vec<Window>,
}

pub(super) fn derive(
    code: CodePlan<'_>,
    proof: &Proof,
    tail: &NativeTailLoop,
    completion: &Completed,
    graph: &Graph,
    windows: &[Window],
) -> Option<Reduction> {
    let candidate = completion.candidate();
    // The maintained native artifact target is the current host. Wider packed
    // loads below use its little-endian unsigned storage; other hosts decline.
    if !cfg!(target_endian = "little") || !escape::private_temporaries(code, tail) {
        return None;
    }
    let CodePlan::Program(program) = code else {
        return None;
    };
    let (tag, width) = program.metadata().scalar_aggregates.slot(tail.carrier())?;
    let (next, next_width) = program.metadata().scalar_aggregates.slot(tail.next())?;
    // One induction, one minimum and one associated choice, in either order.
    if width != 4
        || next_width != width
        || graph.advances.is_empty()
        || !terminal_local(graph, tag, width)
        || !exhaustion_preserves_payload(graph, tag, next, width)
    {
        return None;
    }
    let mut answer = None;
    for advance in &graph.advances {
        let fields = (1..width)
            .filter(|i| tag + i != candidate.induction)
            .collect::<Vec<_>>();
        let mut found = None;
        for &offset in &fields {
            let best = tag + offset;
            let Node::Select { condition, yes, no } =
                graph.nodes[*advance.get(&(next + offset))?]
            else {
                continue;
            };
            if graph.nodes[no] != Node::Initial(best) {
                continue;
            }
            let Node::Scalar(Op::U32Lt, args) = &graph.nodes[condition] else {
                continue;
            };
            if args.as_slice() != [yes, no] {
                continue;
            }
            let item_offset = *fields.iter().find(|&&i| i != offset)?;
            let Node::Select {
                condition: item_condition,
                yes: selected,
                no: old_item,
            } = graph.nodes[*advance.get(&(next + item_offset))?]
            else {
                continue;
            };
            if graph.nodes[old_item] != Node::Initial(tag + item_offset)
                || !equivalent(graph, condition, item_condition, &mut 128)
            {
                continue;
            }
            let mut normalize = Normalize {
                graph,
                candidate,
                windows,
                fuel: 256,
            };
            let cost = normalize.cost(yes)?;
            let selected = normalize.item(selected)?;
            let mut reads = Vec::new();
            cost.reads(&mut reads);
            reads.sort();
            reads.dedup();
            if reads.len() != 2
                || !reads.iter().any(|w| !w.reverse)
                || !reads.iter().any(|w| w.reverse)
                || reads
                    .iter()
                    .any(|w| !matches!(proof.arrays[w.array].width, 4 | 8))
            {
                continue;
            }
            found = Some(Reduction {
                candidate: candidate.clone(),
                best,
                item: tag + item_offset,
                cost,
                selected,
                reads,
            });
        }
        let found = found?;
        if answer.as_ref().is_some_and(|old: &Reduction| {
            old.best != found.best
                || old.item != found.item
                || old.cost != found.cost
                || old.selected != found.selected
        }) {
            return None;
        }
        answer = Some(found);
    }
    answer
}

/// Completion proves the loop finishes, not that its exhausted transition is
/// an identity. Only bypass that source edge when every current payload is
/// transported unchanged on every recorded exit. Other transitions retain the
/// existing native source traversal and its complete-child funding proof.
fn exhaustion_preserves_payload(graph: &Graph, tag: usize, next: usize, width: usize) -> bool {
    !graph.exhausted.is_empty()
        && graph.exhausted.iter().all(|edge| {
            (1..width).all(|offset| {
                edge.get(&(next + offset))
                    .and_then(|&id| graph.nodes.get(id))
                    == Some(&Node::Initial(tag + offset))
            })
        })
}

fn terminal_local(graph: &Graph, tag: usize, width: usize) -> bool {
    fn local(graph: &Graph, id: usize, tag: usize, width: usize, fuel: &mut usize) -> bool {
        let Some(left) = fuel.checked_sub(1) else {
            return false;
        };
        *fuel = left;
        match &graph.nodes[id] {
            Node::Initial(word) => {
                (tag..tag + width).contains(word) || !graph.written.contains(word)
            }
            Node::Constant(_) | Node::Length(_) => true,
            Node::Scalar(_, args) => args.iter().all(|&id| local(graph, id, tag, width, fuel)),
            Node::Read { index, .. } => local(graph, *index, tag, width, fuel),
            Node::Select { condition, yes, no } => [*condition, *yes, *no]
                .into_iter()
                .all(|id| local(graph, id, tag, width, fuel)),
            Node::Unknown => false,
        }
    }
    graph.terminal_inputs.as_ref().is_some_and(|inputs| {
        inputs
            .iter()
            .all(|&id| local(graph, id, tag, width, &mut 256))
    })
}

fn equivalent(graph: &Graph, a: usize, b: usize, fuel: &mut usize) -> bool {
    let Some(left) = fuel.checked_sub(1) else {
        return false;
    };
    *fuel = left;
    if a == b {
        return true;
    }
    match (&graph.nodes[a], &graph.nodes[b]) {
        (Node::Scalar(op, args), Node::Scalar(other, inputs)) => {
            op == other
                && args.len() == inputs.len()
                && args
                    .iter()
                    .zip(inputs)
                    .all(|(&a, &b)| equivalent(graph, a, b, fuel))
        }
        (Node::Constant(a), Node::Constant(b)) => a == b,
        (Node::Initial(a), Node::Initial(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests;
