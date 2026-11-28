//! Pure, independently rederived source/replay funds for complete generators.
//! Bounds grant neither a pointer nor permission to execute a source operation.
use super::{banks, *};
use crate::native_artifact::iteration_abi::NativeIterationVisit;
use mech_engine::native_artifact::NativePreparedLibraryCall;

#[derive(Clone, Debug, serde::Serialize)]
pub(in crate::native_artifact) struct Budget {
    pub setup: u64,
    pub native: u64,
    pub native_per_visit: u64,
    pub compute: u64,
    pub comparison: u64,
    pub replay: u64,
    pub temporary: u64,
    pub output: u64,
    pub nodes: u64,
    pub elements: u64,
    pub cloned: u64,
    pub maximum: [u64; banks::MAX_LOOPS],
}
#[derive(Clone, Debug, serde::Serialize)]
pub(in crate::native_artifact) struct Selected {
    pub region: Region,
    pub banks: banks::Layout,
    pub budget: Budget,
}
#[derive(Debug, Default, serde::Serialize)]
pub(in crate::native_artifact) struct Layout {
    pub regions: Vec<Selected>,
    pub declined: Vec<(u32, &'static str)>,
}
impl Layout {
    pub fn at_header(&self, site: usize) -> Option<&Selected> {
        self.regions
            .iter()
            .find(|s| s.region.header as usize == site)
    }
    pub fn derive(
        inventory: super::Layout,
        plan: &Plan,
        tails: &tail::Layout,
        calls: &[Option<NativePreparedLibraryCall>],
        visits: &[NativeIterationVisit],
        scalar_bytes: u64,
        schema_bytes: u64,
    ) -> Self {
        let mut out = Self {
            regions: Vec::new(),
            declined: inventory.declined,
        };
        for region in inventory.regions {
            let result = banks::Layout::derive(plan, &region, tails).and_then(|banks| {
                budget(
                    plan,
                    &region,
                    &banks,
                    tails,
                    calls,
                    visits,
                    scalar_bytes,
                    schema_bytes,
                )
                .map(|budget| Selected {
                    region: region.clone(),
                    banks,
                    budget,
                })
                .ok_or("generator source/replay envelope is not representable")
            });
            match result {
                Ok(selected) => out.regions.push(selected),
                Err(reason) => out.declined.push((region.collection, reason)),
            }
        }
        out
    }
}

/// Maximum primitive leaves and aggregate shells for the exact source schema.
/// Enum variants are alternatives; tuple fields coexist. Only fixed numeric
/// types are admitted, so no String/Map payload can hide behind one leaf.
fn leaves(body: &SchemaBody, depth: usize) -> Option<u64> {
    if depth == 16 {
        return None;
    }
    if NativeScalarKind::from_schema_body(body).is_some() || body == &SchemaBody::Index {
        return Some(1);
    }
    match body {
        SchemaBody::Matrix { .. } => {
            let [r, c] = matrix(body)?;
            r.checked_mul(c)?.checked_add(1)
        }
        SchemaBody::Tuple(fields) => fields
            .iter()
            .try_fold(1u64, |n, b| n.checked_add(leaves(b, depth + 1)?)),
        SchemaBody::Enum { variants, .. } => variants.iter().try_fold(1u64, |n, v| {
            Some(
                n.max(
                    v.payload
                        .as_ref()
                        .map_or(Some(0), |b| leaves(b, depth + 1))?
                        .checked_add(1)?,
                ),
            )
        }),
        _ => None,
    }
}
fn maximum(region: &Region, iteration: u32) -> Option<u64> {
    if iteration == region.iteration {
        return region.count.checked_add(1);
    }
    if let Some(map) = region.maps.iter().find(|m| m.region.iteration == iteration) {
        return region.count.checked_mul(map.count.checked_add(1)?);
    }
    region.count.checked_mul(
        region
            .children
            .iter()
            .find(|c| c.trips.iteration == iteration)?
            .trips
            .maximum,
    )
}
fn budget(
    plan: &Plan,
    region: &Region,
    banks: &banks::Layout,
    tails: &tail::Layout,
    calls: &[Option<NativePreparedLibraryCall>],
    visits: &[NativeIterationVisit],
    scalar_bytes: u64,
    schema_bytes: u64,
) -> Option<Budget> {
    let inventory = (plan.values().len() as u64)
        .checked_add(plan.blocks().len() as u64)?
        .checked_add(plan.iterations().len() as u64)?
        .checked_add(plan.collections().len() as u64)?;
    let probe = inventory
        .checked_mul(8192)?
        .checked_add(schema_bytes.checked_mul(128)?)?
        .checked_add(scalar_bytes.checked_mul(128)?)?
        .checked_add(65536)?;
    let mut out = Budget {
        setup: probe,
        native: 0,
        native_per_visit: 0,
        compute: 0,
        comparison: 0,
        replay: probe,
        temporary: 0,
        output: 0,
        nodes: 8,
        elements: 0,
        cloned: 0,
        maximum: [0; banks::MAX_LOOPS],
    };
    for (at, &iteration) in banks.loops.iter().enumerate() {
        out.maximum[at] = maximum(region, iteration)?;
    }
    let mut source_owners = 0u64;
    let local_leaves = region.local_values.iter().try_fold(0u64, |total, &v| {
        total.checked_add(leaves(plan.values()[v as usize].definition().body(), 0)?)
    })?;
    // Every original site is priced using its own lexical loop. Parent setup,
    // child initialization/exit and map bridges retain their parent frequency.
    // All branch arms are funded; a successful attempt may conservatively pay
    // more than the path it took, never less than the original source.
    for &site in &region.sites {
        let block = &plan.blocks()[site as usize];
        let n = maximum(region, block.iteration()?)?;
        let ordinary = visits
            .get(site as usize)?
            .work
            .checked_add(tail::OVERHEAD)?;
        let mut compute = ordinary.checked_add(4096)?;
        let mut replay = ordinary.checked_mul(128)?.checked_add(probe)?;
        if let Action::LibraryCall {
            template,
            operands,
            output,
        } = block.action()
        {
            let call = calls.get(site as usize)?.as_ref()?;
            if call.template() != template {
                return None;
            }
            if let Some(read) = call.scalar_matrix_element_native_work() {
                // The maintained selected scalar read constructs no matrix.
                // Reuse its complete native work and the established ordinary
                // read replay inventory, rather than inventing a source copy.
                compute = compute
                    .checked_add(read)?
                    .checked_add(scalar_bytes.checked_mul(16)?)?;
                replay = replay.checked_add(read)?.checked_add(probe)?;
                out.temporary = out.temporary.max(scalar_bytes);
                out.output = out.output.max(scalar_bytes);
                out.nodes = out.nodes.checked_add(4)?;
                out.elements = out.elements.max(1);
            } else {
                let input = operands.iter().try_fold(0u64, |n, v| {
                    n.checked_add(leaves(
                        plan.values()[v.get() as usize].definition().body(),
                        0,
                    )?)
                })?;
                let output = leaves(plan.values()[output.get() as usize].definition().body(), 0)?;
                let (source, source_replay) = call.fixed_numeric_source_bound(input, output)?;
                compute = compute.checked_add(source.work.compute)?;
                replay = replay.checked_add(source_replay)?;
                out.comparison = out
                    .comparison
                    .checked_add(n.checked_mul(source.work.comparison)?)?;
                out.cloned = out
                    .cloned
                    .checked_add(n.checked_mul(source.cloned_bytes)?)?;
                out.temporary = out.temporary.max(source.turn_peak_bytes);
                out.output = out.output.max(source.persistent_bytes);
                source_owners = source_owners.checked_add(source.persistent_bytes)?;
                out.nodes = out.nodes.checked_add(source.retained_nodes)?;
                out.elements = out.elements.max(source.output_elements);
            }
        } else {
            // Current words/owners, source retirement and aggregate copies are
            // real work even when the native body keeps them in private SSA.
            let fees = local_leaves
                .checked_mul(256)?
                .checked_add(scalar_bytes.checked_mul(16)?)?;
            compute = compute.checked_add(fees)?;
            replay = replay.checked_add(fees.checked_mul(128)?)?;
        }
        out.compute = out.compute.checked_add(n.checked_mul(compute)?)?;
        out.replay = out.replay.checked_add(n.checked_mul(replay)?)?;
    }
    // Reuse full existing original range/builder/transpose source envelopes.
    for map in &region.maps {
        let b = map.budget(plan, calls, visits)?;
        out.compute = out
            .compute
            .checked_add(region.count.checked_mul(b.compute)?)?;
        out.replay = out
            .replay
            .checked_add(region.count.checked_mul(b.replay)?)?;
        out.comparison = out
            .comparison
            .checked_add(region.count.checked_mul(b.comparison)?)?;
        out.cloned = out
            .cloned
            .checked_add(region.count.checked_mul(b.cloned)?)?;
        out.temporary = out.temporary.max(b.temporary);
        out.output = out.output.max(b.output);
        out.nodes = out.nodes.checked_add(b.nodes)?;
        out.elements = out.elements.max(b.elements);
        source_owners = source_owners.checked_add(b.output)?;
    }
    // Selected FSM prices include checked scalar instructions, source
    // boundaries and common frame emission; source constructor/copy envelopes
    // above additionally retain their first-change and publication costs.
    for child in &region.children {
        let r = &tails.regions[child.region];
        let n = maximum(region, child.trips.iteration)?;
        out.compute = out.compute.checked_add(n.checked_mul(r.compute)?)?;
        out.comparison = out.comparison.checked_add(n.checked_mul(r.comparison)?)?;
        out.replay = out
            .replay
            .checked_add(n.checked_mul(r.compute.checked_mul(128)?)?)?;
    }
    let cells = region.extents[0].checked_mul(region.extents[1])?;
    let bytes = cells.checked_mul(8)?;
    let drafts = cells.checked_add(8)?.checked_mul(
        (core::mem::size_of::<mech_core::ValueDataDraft>() as u64)
            .checked_mul(16)?
            .checked_add(1024)?,
    )?;
    // Original packed-block transport, geometric growth, row-order conversion,
    // final canonical result and all simultaneously live source owners.
    out.temporary = out
        .temporary
        .checked_add(source_owners)?
        .checked_add(drafts)?
        .checked_add(scalar_bytes)?;
    out.output = out.output.max(drafts.checked_add(scalar_bytes)?);
    out.elements = out.elements.max(cells);
    out.cloned = out
        .cloned
        .checked_add(drafts)?
        .checked_add(bytes.checked_mul(32)?)?;
    out.nodes = out
        .nodes
        .checked_add(cells.checked_mul(8)?)?
        .checked_add(256)?;
    out.compute = out
        .compute
        .checked_add(drafts.checked_mul(16)?)?
        .checked_add(bytes.checked_mul(1024)?)?;
    out.replay = out
        .replay
        .checked_add(drafts.checked_mul(2048)?)?
        .checked_add(probe)?;
    // Conservative physical instruction/copy/guard bound, charged in full
    // before entering generated code, including a failed private attempt.
    // Audit the whole static emitted graph against this per-source-visit
    // envelope. Its product with every possible source visit covers native
    // control/SSA even for wide aggregate frames. Fixed private copy loops
    // additionally retain their exact bank cardinality below.
    out.native_per_visit = inventory
        .checked_mul(256)?
        .checked_add(local_leaves.checked_mul(256)?)?
        .checked_add((region.sites.len() as u64).checked_mul(2048)?)?
        .checked_add(65536)?;
    out.native = region
        .maximum_visits
        .checked_mul(out.native_per_visit)?
        .checked_add(banks.elements.checked_mul(region.count)?.checked_mul(128)?)?
        .checked_add(bytes.checked_mul(128)?)?
        .checked_add(65536)?;
    out.setup = out.setup.checked_add(out.temporary.checked_mul(16)?)?;
    out.setup
        .checked_add(out.native)?
        .checked_add(out.compute)?
        .checked_add(out.replay)?;
    out.comparison.checked_mul(2)?;
    Some(out)
}
