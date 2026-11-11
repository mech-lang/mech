//! A current sole-owner capability for short, non-growing write intervals.
//! The source CFG and ordinary owner settlement remain authoritative. No
//! capability survives a call, alias construction, join, or changed view.
use super::{
    flow::Flow,
    layout::{Kind, Layout},
    recipes::Recipe,
};
use crate::native_artifact::codegen::common::ssa_words::Words;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as P, NativeIterationAction as Action,
    NativeIterationTerminator as Term, NativeMatchAction as Scalar,
    NativeMatchTerminator as Branch, NativeMatchValue as V, NativeProgramArtifactPlan as Plan,
};
use std::collections::BTreeSet;

// A proof-search/code-growth bound, not an execution allowance. Larger or
// incompatible intervals retain their unchanged ordinary lowering.
const MAX_SITES: usize = 32;

pub(super) struct Chain {
    sites: Vec<usize>,
    writes: Vec<(usize, V, V)>,
    scalar_reads: Vec<usize>,
    source: V,
    exit: usize,
}

/// Construction emits the present, current sole-owner guard and selects its
/// successful block. This witness cannot be manufactured by array emitters.
pub(super) struct Admitted<'chain> {
    chain: &'chain Chain,
}
impl Admitted<'_> {
    pub(super) fn permits(&self, site: usize, source: V, output: V) -> bool {
        self.chain.writes.contains(&(site, source, output))
    }
}

impl Chain {
    pub(super) fn first(&self) -> usize {
        self.sites[0]
    }
    pub(super) fn sites(&self) -> &[usize] {
        &self.sites
    }
    pub(super) fn exit(&self) -> usize {
        self.exit
    }
    pub(super) fn summary(&self) -> serde_json::Value {
        let mut summary = serde_json::json!({"first":self.first(),"sites":self.sites,"writes":self.writes.iter().map(|&(site,source,output)|serde_json::json!({"site":site,"source":source.get(),"output":output.get()})).collect::<Vec<_>>(),"exit":self.exit,"permission":"current nonnull sole-owner guard; exact consumed non-growing view; empty prechange retirements"});
        if !self.scalar_reads.is_empty() {
            summary["scalar_read_sites"] = serde_json::json!(self.scalar_reads);
        }
        summary
    }
    pub(super) fn admit<'chain>(
        &'chain self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        layout: &Layout,
        fallback: Block,
    ) -> Admitted<'chain> {
        let owner = words.read(
            b,
            layout.slot(self.source.get()) + super::layout::ARRAY_OWNER,
        );
        let present = b.ins().icmp_imm(IntCC::NotEqual, owner, 0);
        let inspect = b.create_block();
        let private = b.create_block();
        b.ins().brif(present, inspect, &[], fallback, &[]);
        b.switch_to_block(inspect);
        let refs = b.ins().load(types::I64, MemFlags::new(), owner, 0);
        let unique = b.ins().icmp_imm(IntCC::Equal, refs, 1);
        b.ins().brif(unique, private, &[], fallback, &[]);
        b.switch_to_block(private);
        Admitted { chain: self }
    }
}

fn replacement(
    site: usize,
    layout: &Layout,
    actions: &[Recipe],
    consuming: &[bool],
    buffer_results: &[Option<(usize, usize, bool)>],
    retire_before_change: &[Vec<usize>],
) -> Option<(V, V)> {
    let Recipe::Buffer {
        operation: P::MatrixIndexedReplace,
        operands,
        output,
    } = &actions[site]
    else {
        return None;
    };
    if operands.len() != 3
        || !consuming[site]
        || !matches!(buffer_results[site], Some((_, _, true)))
        || !retire_before_change[site].is_empty()
    {
        return None;
    }
    let source = operands[0];
    // Full element checks and canonical row/column/rectangular translation are
    // shared with ordinary emission. Identity here forbids changing the view.
    if layout.ty(source.get()) != layout.ty(output.get())
        || !matches!(layout.ty(source.get()).kind, Kind::Array { .. })
    {
        return None;
    }
    Some((source, *output))
}

fn scalar_only(plan: &Plan, layout: &Layout, flow: &Flow, actions: &[Recipe], site: usize) -> bool {
    let owner_free = flow.uses[site]
        .iter()
        .chain(&flow.defs[site])
        .all(|&value| layout.ty(value).owners.is_empty());
    owner_free
        && match &actions[site] {
            Recipe::Aggregate(_) => true,
            Recipe::None => matches!(
                plan.blocks()[site].action(),
                Action::Scalar(Scalar::None | Scalar::Primitive { .. })
            ),
            _ => false,
        }
}

// A checked scalar read borrows this exact current view and publishes no
// owner. It neither changes backing/geometry nor creates another live alias.
// The shared action emitter still performs the read at its original position,
// including original bounds and canonical matrix translation. No permission
// is inferred for a different view or an owner-producing element.
fn scalar_read(layout: &Layout, actions: &[Recipe], site: usize, current: V) -> bool {
    let Recipe::Read { source, output, .. } = &actions[site] else {
        return false;
    };
    *source == current
        && layout.ty(output.get()).owners.is_empty()
        && matches!(layout.ty(output.get()).kind, Kind::Scalar { .. })
}

pub(super) fn derive(
    plan: &Plan,
    layout: &Layout,
    flow: &Flow,
    actions: &[Recipe],
    consuming: &[bool],
    buffer_results: &[Option<(usize, usize, bool)>],
    retire_before_change: &[Vec<usize>],
    boundaries: &BTreeSet<usize>,
) -> Vec<Chain> {
    let permit_reads = std::env::var_os("MECH_PRIVATE_READ_INTERVALS").is_some();
    let mut predecessors = vec![BTreeSet::new(); actions.len()];
    for (site, edges) in flow.successors.iter().enumerate() {
        if flow.reachable[site] {
            for &next in edges {
                predecessors[next].insert(site);
            }
        }
    }
    let mut occupied = BTreeSet::new();
    let mut chains = Vec::new();
    for first in 0..actions.len() {
        if !flow.reachable[first] || occupied.contains(&first) || boundaries.contains(&first) {
            continue;
        }
        let Some((source, mut current)) = replacement(
            first,
            layout,
            actions,
            consuming,
            buffer_results,
            retire_before_change,
        ) else {
            continue;
        };
        let mut sites = vec![first];
        let mut writes = vec![(first, source, current)];
        let mut end = 1;
        let mut at = first;
        while sites.len() < MAX_SITES {
            let Term::Scalar(Branch::Jump(next)) = plan.blocks()[at].terminator() else {
                break;
            };
            let next = next as usize;
            if predecessors[next].len() != 1
                || !predecessors[next].contains(&at)
                || occupied.contains(&next)
                || boundaries.contains(&next)
                || sites.contains(&next)
            {
                break;
            }
            if let Some((from, output)) = replacement(
                next,
                layout,
                actions,
                consuming,
                buffer_results,
                retire_before_change,
            ) {
                if from != current {
                    break;
                }
                current = output;
                writes.push((next, from, output));
                sites.push(next);
                end = sites.len();
            } else if scalar_only(plan, layout, flow, actions, next)
                || (permit_reads && scalar_read(layout, actions, next, current))
            {
                sites.push(next);
            } else {
                break;
            }
            at = next;
        }
        sites.truncate(end);
        if writes.len() < 2 {
            continue;
        }
        let Term::Scalar(Branch::Jump(exit)) = plan.blocks()[*sites.last().unwrap()].terminator()
        else {
            continue;
        };
        let exit = exit as usize;
        if sites.contains(&exit) {
            continue;
        }
        occupied.extend(sites.iter().copied());
        let scalar_reads = sites
            .iter()
            .copied()
            .filter(|&site| matches!(actions[site], Recipe::Read { .. }))
            .collect();
        chains.push(Chain {
            sites,
            writes,
            scalar_reads,
            source,
            exit,
        });
    }
    chains
}

#[cfg(test)]
mod tests {
    use super::super::{guards::Guards, owner_flow::OwnerFlow, recipes};
    use super::*;
    use mech_engine::CanonicalSourceFrontend;
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };

    const SOURCE: &str = "+> sequence\nupdate(values<[u16]>, index<u64>, second<u64>, a<u16>, b<u16>, keep<bool>, allowed<bool>) => (out<([u16],[u16])>) :=\n  entry! := allowed\n  seed := sequence/append(values, 0u16)\n  separate := sequence/append(values, 1u16)\n  alias := (keep? | true => seed | false => separate)\n  ~first := seed\n  first[index] = a\n  ~next := first\n  next[second + 1u64] = b\n  out := (next, alias).\nupdate(values<[u16]>, index<u64>, second<u64>, a<u16>, b<u16>, keep<bool>, allowed<bool>)\n";

    struct Fixture {
        plan: Plan,
        layout: Layout,
        flow: Flow,
        actions: Vec<Recipe>,
        consuming: Vec<bool>,
        results: Vec<Option<(usize, usize, bool)>>,
        retirements: Vec<Vec<usize>>,
        boundaries: BTreeSet<usize>,
    }
    impl Fixture {
        fn new() -> Self {
            let parsed = parse_canonical_document(
                TextSnapshot::new(DocumentId(240), Revision(1), SOURCE).unwrap(),
                ParseConfig::default(),
            );
            assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
            let artifact = CanonicalSourceFrontend
                .compile_document_with_catalog(
                    &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                    mech_stdlib::source_catalog(),
                )
                .unwrap()
                .compile_artifact()
                .unwrap();
            let plan = mech_engine::native_artifact::lower_native_program_artifact_with_definition_table_limit(
                &artifact, &mech_stdlib::source_catalog(), u64::MAX,
            ).unwrap();
            let layout = Layout::derive(&plan).unwrap();
            let flow = Flow::derive(&plan).unwrap();
            let actions = (0..plan.blocks().len())
                .map(|site| recipes::derive(&plan, &layout, site).unwrap())
                .collect::<Vec<_>>();
            let owners = OwnerFlow::derive(&plan, &layout, &flow, &actions).unwrap();
            let guards = Guards::derive(&plan, &flow);
            let mut consuming = vec![false; actions.len()];
            let mut results = vec![None; actions.len()];
            let mut retirements = vec![Vec::new(); actions.len()];
            for (site, action) in actions.iter().enumerate() {
                let Recipe::Buffer {
                    operands,
                    output,
                    operation,
                } = action
                else {
                    continue;
                };
                let Some(source) = operands.first() else {
                    continue;
                };
                if !matches!(layout.ty(source.get()).kind, Kind::Array { .. }) {
                    continue;
                }
                let from = layout.slot(source.get()) + super::super::layout::ARRAY_OWNER;
                let to = layout.slot(output.get()) + super::super::layout::ARRAY_OWNER;
                consuming[site] = !owners.live_after(site, from)
                    || owners.last_use_with_guards(&flow, &guards, site, from);
                results[site] = Some((
                    from,
                    to,
                    consuming[site] && owners.can_finish_source(&flow, &guards, site, from),
                ));
                if *operation == P::MatrixIndexedReplace {
                    retirements[site] =
                        owners.retire_before_indexed_change(&flow, &guards, site, from);
                }
            }
            Self {
                plan,
                layout,
                flow,
                actions,
                consuming,
                results,
                retirements,
                boundaries: BTreeSet::new(),
            }
        }
        fn chains(&self) -> Vec<Chain> {
            derive(
                &self.plan,
                &self.layout,
                &self.flow,
                &self.actions,
                &self.consuming,
                &self.results,
                &self.retirements,
                &self.boundaries,
            )
        }
        fn first_chain(&self) -> Chain {
            let mut chains = self.chains();
            assert_eq!(chains.len(), 1);
            chains.remove(0)
        }
    }

    #[test]
    fn consumed_same_view_writes_include_checked_scalar_between_them() {
        let f = Fixture::new();
        let c = f.first_chain();
        assert_eq!(c.writes.len(), 2);
        assert_eq!(c.sites.len(), 3);
        assert_eq!(c.writes[0].2, c.writes[1].1);
        assert!(!c.sites.contains(&c.exit));
    }
    #[test]
    fn every_write_requires_consumption_and_current_finish_proof() {
        let mut f = Fixture::new();
        let c = f.first_chain();
        for &(site, _, _) in &c.writes {
            f.consuming[site] = false;
            assert!(f.chains().is_empty());
            f.consuming[site] = true;
            let before = f.results[site];
            f.results[site].as_mut().unwrap().2 = false;
            assert!(f.chains().is_empty());
            f.results[site] = before;
        }
    }
    #[test]
    fn equality_dependent_retirement_forbids_private_interval() {
        let mut f = Fixture::new();
        let c = f.first_chain();
        for &(site, _, _) in &c.writes {
            f.retirements[site].push(0);
            assert!(f.chains().is_empty());
            f.retirements[site].clear();
        }
    }
    #[test]
    fn extra_incoming_edge_forbids_carrying_current_permission() {
        let mut f = Fixture::new();
        let c = f.first_chain();
        let entry = f.plan.entry() as usize;
        assert_ne!(entry, c.first());
        f.flow.successors[entry].push(c.sites[1]);
        assert!(f.chains().is_empty());
    }
    #[test]
    fn phase_or_packed_boundary_forbids_crossing_interval() {
        let mut f = Fixture::new();
        let c = f.first_chain();
        for &site in &c.sites {
            f.boundaries.insert(site);
            assert!(f.chains().is_empty());
            f.boundaries.clear();
        }
    }
    #[test]
    fn alias_construction_or_owned_use_forbids_carrying_permission() {
        let mut f = Fixture::new();
        let c = f.first_chain();
        f.flow.uses[c.sites[1]].push(c.source.get());
        assert!(f.chains().is_empty());
    }
    #[test]
    fn different_current_array_cannot_share_capability() {
        let mut f = Fixture::new();
        let c = f.first_chain();
        let Recipe::Buffer { operands, .. } = &mut f.actions[c.writes[1].0] else {
            panic!()
        };
        operands[0] = c.source;
        assert!(f.chains().is_empty());
    }
    #[test]
    fn unknown_or_memory_action_forbids_private_interval() {
        let mut f = Fixture::new();
        let c = f.first_chain();
        f.actions[c.sites[1]] = Recipe::Length {
            source: c.source,
            output: c.source,
        };
        assert!(f.chains().is_empty());
    }
}
