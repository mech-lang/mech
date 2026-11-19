//! Typed pre-SSA ownership and local domains, shared by preview and lowering.
use super::{
    flow::Flow,
    layout::Layout,
    partition_plan::{End, Graph},
};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use mech_engine::native_artifact::{
    NativeControlLoop as Loop, NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchTerminator as Branch, NativeProgramArtifactPlan as Plan,
};
use std::{collections::BTreeSet, path::Path};

pub(super) fn preview(
    plan: &Plan,
    layout: &Layout,
    windows: &str,
    path: &Path,
) -> Result<(), NativeCompileError> {
    let flow = Flow::derive(plan)?;
    let ownership = plan_partition(plan, layout, &flow, windows)?;
    let iteration_count = plan.iterations().len();
    let mut scopes = vec![Vec::new(); iteration_count + plan.collections().len()];
    for (site, &owner) in ownership.owners.iter().enumerate() {
        if owner == Some(0) {
            // Root resource references are checked against complete typed scopes
            // by plan_partition; the report only needs their reachable owners.
            match plan.blocks()[site].action() {
                Action::Initialize { iteration }
                | Action::Latch { iteration }
                | Action::PrepareU64Range { iteration }
                | Action::CollectionElement { iteration } => scopes[*iteration as usize].push(site),
                Action::CollectionBegin { collection }
                | Action::CollectionPush { collection, .. }
                | Action::CollectionFinish { collection } => {
                    scopes[iteration_count + *collection as usize].push(site)
                }
                _ => {}
            }
            if let Term::Header { iteration, .. } | Term::TailHeader { iteration, .. } =
                plan.blocks()[site].terminator()
            {
                scopes[iteration as usize].push(site);
            }
        }
    }

    let describe = |ids: &[u32], domain: &[u32]| {
        ids.iter()
            .enumerate()
            .map(|(local, &global)| {
                let ty = layout.ty(global);
                serde_json::json!({"global_value":global,"local_value":domain.binary_search(&global).expect("unit local value"),"transport_position":local,
            "global_word_base":layout.slot(global),"words":ty.words,
            "owner_lanes":ty.owners,"type":ty.summary()})
            })
            .collect::<Vec<_>>()
    };
    let slots = |ids: &[u32]| {
        ids.iter()
            .flat_map(|&v| layout.slot(v)..layout.slot(v) + layout.ty(v).words)
            .collect::<Vec<_>>()
    };
    let hidden_words = |ids: &[usize]| {
        ids.iter()
            .map(|&id| {
                if id >= iteration_count {
                    7
                }
                // six builder lanes and rows-set flag
                else if matches!(plan.iterations()[id], Loop::Collection(_)) {
                    3
                } else {
                    0
                } // tail carriers already have ordinary typed value homes
            })
            .sum::<usize>()
    };
    let region_scopes = ownership
        .regions
        .iter()
        .flat_map(|r| r.scopes.iter().copied())
        .collect::<BTreeSet<_>>();
    let root_scopes = scopes
        .iter()
        .enumerate()
        .filter_map(|(id, refs)| {
            (!region_scopes.contains(&id) && refs.iter().any(|&s| flow.reachable[s])).then_some(id)
        })
        .collect::<Vec<_>>();
    let regions = ownership.regions.iter().map(|r| {
        let origins = r.failures.iter().map(|&s|serde_json::json!({"site":s,"origin":plan.blocks()[s].origin()})).collect::<Vec<_>>();
        serde_json::json!({"entry":r.entry,"normal_exit":r.exit,"source_sites":r.sites,
            "input_values":describe(&r.inputs, &r.local_values),"output_values":describe(&r.outputs, &r.local_values),
            "constant_values":describe(&r.constants, &r.local_values),"local_values":describe(&r.local_values, &r.local_values),
            "local_word_slots":slots(&r.local_values),"local_words":r.local_words,
            "local_collection_state_words":hidden_words(&r.scopes),
            "input_words":r.inputs.iter().map(|&v|layout.ty(v).words).sum::<usize>(),
            "output_words":r.outputs.iter().map(|&v|layout.ty(v).words).sum::<usize>(),
            "owned_iterations":r.scopes.iter().filter(|&&id|id<iteration_count).collect::<Vec<_>>(),
            "owned_collections":r.scopes.iter().filter(|&&id|id>=iteration_count).map(|id|id-iteration_count).collect::<Vec<_>>(),
            "explicit_failures":origins,"implicit_failure_origin_sites":r.sites,
            "all_path_initialization_proved":true,
            "synthetic_root_call_origin":plan.blocks()[r.entry].origin()})
    }).collect::<Vec<_>>();
    let report = serde_json::json!({"status":"typed source ownership and local domains only; this preview does not enable emission",
        "emission_authorized":false,"source_sites":plan.blocks().len(),"global_values":layout.types.len(),
        "global_words":layout.words,"source_owners":ownership.owners,
        "root":{"source_sites":ownership.root_sites,"local_values":describe(&ownership.root_values, &ownership.root_values),
            "local_words":ownership.root_words,"local_word_slots":slots(&ownership.root_values),
            "local_collection_state_words":hidden_words(&root_scopes),
            "owned_iterations":root_scopes.iter().filter(|&&id|id<iteration_count).collect::<Vec<_>>(),
            "owned_collections":root_scopes.iter().filter(|&&id|id>=iteration_count).map(|id|id-iteration_count).collect::<Vec<_>>(),
            "synthetic_region_calls":ownership.regions.len()},"regions":regions,
        "unproved":["failure and partial-write transport","ownership handoff and cleanup at calls",
            "finite admission and native ABI/frame checks","lowering local root/helper CFG and word domains",
            "total construction reduction and actual-worker qualification"]});
    std::fs::write(path, serde_json::to_vec_pretty(&report).map_err(backend)?).map_err(backend)
}

pub(super) fn plan_partition(
    plan: &Plan,
    layout: &Layout,
    flow: &Flow,
    windows: &str,
) -> Result<super::partition_plan::Plan, NativeCompileError> {
    let windows = windows
        .split(';')
        .map(|window| {
            let (entry, exit) = window
                .split_once(':')
                .ok_or_else(|| backend("expected source entry:exit"))?;
            Ok((
                entry.parse::<usize>().map_err(backend)?,
                exit.parse::<usize>().map_err(backend)?,
            ))
        })
        .collect::<Result<Vec<_>, NativeCompileError>>()?;
    if windows.is_empty() {
        return Err(backend("empty early source partition request"));
    }
    let ends = plan
        .blocks()
        .iter()
        .map(|b| match b.terminator() {
            Term::Scalar(Branch::Complete) => End::Complete,
            Term::Scalar(Branch::Reject | Branch::IntegrityFailure) => End::Failure,
            _ => End::Continue,
        })
        .collect::<Vec<_>>();
    let words = layout.types.iter().map(|t| t.words).collect::<Vec<_>>();
    let written = flow.defs.iter().flatten().copied().collect::<BTreeSet<_>>();
    let constants = plan
        .constants()
        .iter()
        .map(|c| c.value().get())
        .filter(|v| !written.contains(v))
        .collect::<BTreeSet<_>>();
    let iteration_count = plan.iterations().len();
    let mut scopes = vec![Vec::new(); iteration_count + plan.collections().len()];
    for (site, block) in plan.blocks().iter().enumerate() {
        match block.action() {
            Action::Initialize { iteration }
            | Action::Latch { iteration }
            | Action::PrepareU64Range { iteration }
            | Action::CollectionElement { iteration } => scopes[*iteration as usize].push(site),
            Action::CollectionBegin { collection }
            | Action::CollectionPush { collection, .. }
            | Action::CollectionFinish { collection } => {
                scopes[iteration_count + *collection as usize].push(site)
            }
            _ => {}
        }
        match block.terminator() {
            Term::Header { iteration, .. } | Term::TailHeader { iteration, .. } => {
                scopes[iteration as usize].push(site)
            }
            _ => {}
        }
    }
    for sites in &mut scopes {
        sites.sort_unstable();
        sites.dedup();
    }
    let incoming_uses = (0..plan.blocks().len())
        .map(|site| flow.incoming_uses(site).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let graph = Graph {
        reachable: &flow.reachable,
        successors: &flow.successors,
        uses: &incoming_uses,
        defs: &flow.defs,
        ends: &ends,
        words: &words,
        invariant_constants: &constants,
        scopes: &scopes,
        inputs: &flow.inputs,
        outputs: &flow.outputs,
    };
    let ownership = graph.partition(&windows).map_err(backend)?;
    for region in &ownership.regions {
        // Reuse the existing all-path source initializer proof for this exact
        // proposed input/output boundary. This is not native ABI admission.
        let bounded = Flow::derive_entry(
            plan,
            region.entry,
            region.inputs.clone(),
            region.outputs.clone(),
            Some(region.exit),
            None,
        )?;
        if bounded.reachable.iter().enumerate().any(|(site, &yes)| {
            yes && site != region.exit && region.sites.binary_search(&site).is_err()
        }) {
            return Err(backend("typed source region escaped its local domain"));
        }
    }

    Ok(ownership)
}
