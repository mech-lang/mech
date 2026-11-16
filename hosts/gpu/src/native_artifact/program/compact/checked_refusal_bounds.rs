//! Retry near-identical packed refusals with their repeated bound in one lane.
use super::{Prepared, RegionKind, matching_relocations, prepare_bodies, region_literals};
use crate::native_artifact::NativeCompileError;
use cranelift_codegen::{dominator_tree::DominatorTree, ir::Function};
use cranelift_module::Module;

fn eligible(body: &Prepared) -> bool {
    body.traversals.len() > 1
        && body
            .traversals
            .iter()
            .all(|t| t.packed_inputs && t.literals.is_empty())
}

fn worth_retrying(first: &Prepared, other: &Prepared) -> bool {
    // This census only limits extra backend work. Exact full bytes and
    // relocations still govern the resulting sharing decision.
    if first.signature != other.signature
        || first.bytes.len() != other.bytes.len()
        || !matching_relocations(first.function, &first.relocs, other.function, &other.relocs)
    {
        return false;
    }
    let changed = first
        .bytes
        .chunks_exact(4)
        .zip(other.bytes.chunks_exact(4))
        .filter(|(left, right)| left != right)
        .take(33)
        .count();
    (1..=32).contains(&changed)
}

pub(super) fn retry<M: Module>(
    module: &mut M,
    function: &Function,
    groups: &mut Vec<Prepared>,
    dominance: Option<&DominatorTree>,
) -> Result<(), NativeCompileError> {
    if module.isa().name() != "aarch64" {
        return Ok(());
    }
    let mut bodies = std::mem::take(groups)
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>();
    for index in 0..bodies.len() {
        let Some(first) = bodies[index].as_ref().filter(|body| eligible(body)) else {
            continue;
        };
        let siblings = bodies
            .iter()
            .enumerate()
            .skip(index + 1)
            .filter_map(|(at, body)| {
                let other = body.as_ref()?;
                (eligible(other) && worth_retrying(first, other)).then_some(at)
            })
            .collect::<Vec<_>>();
        if siblings.is_empty() {
            continue;
        }
        let members = std::iter::once(index)
            .chain(siblings.iter().copied())
            .collect::<Vec<_>>();
        let before: usize = members
            .iter()
            .map(|&at| bodies[at].as_ref().unwrap().bytes.len())
            .sum();
        let mut trials = members
            .iter()
            .flat_map(|&at| &bodies[at].as_ref().unwrap().traversals)
            .cloned()
            .collect::<Vec<_>>();
        if !trials
            .iter_mut()
            .all(|t| region_literals::bind_repeated_bound(function, t))
        {
            continue;
        }
        let size = trials.len();
        let mut replacement = prepare_bodies(
            module,
            function,
            trials,
            dominance,
            RegionKind::Refusal,
            false,
            false,
        )?;
        // Reserve room for materializing the bound and storing the packet word
        // at each caller. The complete Object is measured after replacement.
        if replacement.len() != 1 || replacement[0].bytes.len() + 20 * size >= before {
            continue;
        }
        let matched = replacement.pop().unwrap();
        eprintln!(
            "compact checked refusal bound merged {} native bodies at {} callers",
            members.len(),
            size
        );
        bodies[index] = Some(matched);
        for at in siblings {
            bodies[at] = None;
        }
    }
    groups.extend(bodies.into_iter().flatten());
    Ok(())
}
