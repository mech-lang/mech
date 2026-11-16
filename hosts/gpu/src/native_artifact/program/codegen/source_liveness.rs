//! Ordinary source-home liveness at a native region's cold boundaries.
//! Private reconciliation words remain independently retained by its proof.
use super::super::prepare::PreparedNativeProgram;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeMatchAction as Scalar, NativeProgramFacts,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) struct Liveness {
    before: Vec<BTreeSet<usize>>,
}
impl Liveness {
    pub(super) fn derive(
        program: &PreparedNativeProgram,
        candidates: &BTreeSet<usize>,
    ) -> Result<Self, &'static str> {
        let plan = program.plan();
        let facts = plan.facts();
        let mut values = BTreeMap::new();
        for index in 0..plan.transport_input_count() {
            let value = plan.transport_input_value(index);
            values.insert(value.get(), value);
        }
        for constant in plan.constants() {
            values.insert(constant.value().get(), constant.value());
        }
        // An unknown producer keeps its old conservative homes. This inventory
        // only identifies existing constructor-private words; it grants no new
        // interpretation, current-value or execution authority.
        for block in plan.blocks() {
            let output = match block.action() {
                Action::Scalar(Scalar::Primitive { output, .. } | Scalar::Copy { output, .. })
                | Action::LibraryCall { output, .. }
                | Action::Structural { output, .. } => Some(*output),
                _ => None,
            };
            if let Some(value) = output {
                values.insert(value.get(), value);
            }
        }
        let count = plan.blocks().len();
        let mut action = vec![BTreeSet::new(); count];
        let mut terminal = action.clone();
        let mut definitions = action.clone();
        let mut known = BTreeSet::new();
        let mut pinned = BTreeSet::new();
        for (&index, &value) in &values {
            let mut homes = BTreeSet::new();
            if plan.values()[index as usize].scalar_kind().is_some() {
                homes.insert(index as usize);
            }
            if let Some((base, count)) = program.metadata().scalar_aggregates.slot(value) {
                homes.extend(base..base + count);
            }
            homes.retain(|home| candidates.contains(home));
            if homes.is_empty() {
                continue;
            }
            known.extend(&homes);
            for role in facts.read_roles(value)? {
                match NativeProgramFacts::control_read_site(plan, *role) {
                    Some((site, false)) => action[site as usize].extend(&homes),
                    Some((site, true)) => terminal[site as usize].extend(&homes),
                    // Only source-home reads with an exact original visit join
                    // this dataflow. Owner/effect/publication frontiers and
                    // unknown producers keep their conservative pinned homes.
                    None => pinned.extend(&homes),
                }
            }
            for definition in facts.definition_sites(index)? {
                if let Some(site) = definition.site() {
                    definitions[site as usize].extend(&homes);
                }
            }
        }
        pinned.extend(candidates.difference(&known).copied());
        let successors = (0..count)
            .map(|site| facts.successors(site as u32))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            before: solve(&action, &terminal, &definitions, &successors, &pinned),
        })
    }
    pub(super) fn at(&self, site: usize) -> &BTreeSet<usize> {
        &self.before[site]
    }
}

fn solve(
    action: &[BTreeSet<usize>],
    terminal: &[BTreeSet<usize>],
    definitions: &[BTreeSet<usize>],
    successors: &[[Option<u32>; 2]],
    pinned: &BTreeSet<usize>,
) -> Vec<BTreeSet<usize>> {
    let mut predecessors = vec![Vec::new(); successors.len()];
    for (site, next) in successors.iter().enumerate() {
        for &next in next.iter().flatten() {
            predecessors[next as usize].push(site);
        }
    }
    let mut before = vec![BTreeSet::new(); successors.len()];
    let mut queued = vec![true; successors.len()];
    let mut pending = (0..successors.len()).rev().collect::<VecDeque<_>>();
    while let Some(site) = pending.pop_front() {
        queued[site] = false;
        let mut after = terminal[site].clone();
        for &next in successors[site].iter().flatten() {
            after.extend(&before[next as usize]);
        }
        let live = action[site]
            .union(pinned)
            .copied()
            .chain(after.difference(&definitions[site]).copied())
            .collect();
        if live != before[site] {
            before[site] = live;
            for &prior in &predecessors[site] {
                if !core::mem::replace(&mut queued[prior], true) {
                    pending.push_back(prior);
                }
            }
        }
    }
    before
}

#[cfg(test)]
mod tests {
    use super::*;
    fn set(values: &[usize]) -> BTreeSet<usize> {
        values.iter().copied().collect()
    }

    #[test]
    fn actual_loop_next_homes_are_live_at_the_latch_but_not_before_their_definition() {
        use mech_engine::CanonicalSourceFrontend;
        use mech_engine::native_artifact::{
            NativeControlLoop, lower_native_program_artifact_with_definition_table_limit,
        };
        use mech_syntax::document::{
            AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
            parse_canonical_document,
        };
        let text = "advance(remaining<u64>, value<u64>) => (out<u64>) :=\n  more := remaining > 0u64\n  out := (more? | true => advance(remaining - 1u64, value + 3u64) | false => value).\nadvance(limit<u64>, seed<u64>)\n";
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x4c4f4f504c495645), Revision(1), text).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let catalog = mech_stdlib::source_catalog();
        let artifact = CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                catalog.clone(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        for artifact in [artifact, decoded] {
            let plan = lower_native_program_artifact_with_definition_table_limit(
                &artifact,
                &catalog,
                crate::native_artifact::NativeCompileLimits::default().max_definition_table_bytes,
            )
            .unwrap();
            let prepared = PreparedNativeProgram::prepare(&artifact, &catalog, plan).unwrap();
            let plan = prepared.plan();
            let facts = plan.facts();
            let mut witnesses = 0;
            for control in plan.iterations() {
                let NativeControlLoop::Tail(tail) = control else {
                    continue;
                };
                let Some((base, count)) = prepared.metadata().scalar_aggregates.slot(tail.next())
                else {
                    continue;
                };
                let homes = (base..base + count).collect::<BTreeSet<_>>();
                if homes.is_empty() {
                    continue;
                }
                let live = Liveness::derive(&prepared, &homes).unwrap();
                assert!(live.at(tail.latch() as usize).is_superset(&homes));
                for definition in facts.definition_sites(tail.next().get()).unwrap() {
                    if let Some(site) = definition.site() {
                        assert!(
                            live.at(site as usize).is_disjoint(&homes),
                            "new next-carrier words cannot require the previous visit's contents before their producer"
                        );
                        witnesses += 1;
                    }
                }
            }
            assert!(
                witnesses > 0,
                "the actual original/decoded fixture must exercise loop-next scalar homes"
            );
        }
    }
    #[test]
    fn source_liveness_keeps_loop_carried_reads_and_old_values_before_overwrite() {
        // 0: seed x; 1: branch on x; 2: x = x + 1, then backedge;
        // 3: publish x. The unrelated earlier temporary y dies before 1.
        let action = [set(&[]), set(&[]), set(&[0]), set(&[0])];
        let terminal = [set(&[]), set(&[0]), set(&[]), set(&[])];
        let definitions = [set(&[0, 1]), set(&[]), set(&[0]), set(&[])];
        let successors = [
            [Some(1), None],
            [Some(2), Some(3)],
            [Some(1), None],
            [None, None],
        ];
        let result = solve(&action, &terminal, &definitions, &successors, &set(&[9]));
        assert_eq!(
            result,
            [set(&[9]), set(&[0, 9]), set(&[0, 9]), set(&[0, 9])]
        );
    }
    #[test]
    fn source_liveness_does_not_require_old_same_site_terminator_result() {
        let empty = set(&[]);
        let result = solve(
            &[empty.clone(), set(&[1]), set(&[2])],
            &[set(&[0]), empty.clone(), empty.clone()],
            &[set(&[0]), empty.clone(), empty],
            &[[Some(1), Some(2)], [None, None], [None, None]],
            &set(&[]),
        );
        assert_eq!(result, [set(&[1, 2]), set(&[1]), set(&[2])]);
    }
    #[test]
    fn guarded_dictionary_prediction_leaves_are_current_source_homes_at_original_exit() {
        use mech_engine::CanonicalSourceFrontend;
        use mech_syntax::document::{
            AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
            parse_canonical_document,
        };
        let source = r#"+> map
+> sequence
renumber(keys<[string]>,at<u64>,end<u64>,index<{string:u64}>) => (out<{string:u64}>) :=
  more := at < end
  next := (more? | true => map/insert(index,keys[at + 1u64],at) | false => index)
  out := (more? | true => renumber(keys,at + 1u64,end,next) | false => index).
tags := tags<[string]>
histogram := map/histogram(tags)
keys := map/keys(histogram)
end := sequence/length(keys)
index := renumber(keys,0u64,end,histogram)
allowed := (end >= 3u64) && (end <= 64u64)
ids := (allowed? | true => [index{tag} | tag <- tags] | false => [0u64 | tag <- tags])
ids
"#;
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x4755415244484f4d), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let catalog = mech_stdlib::source_catalog();
        let original = CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                catalog.clone(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        for artifact in [original, decoded] {
            let plan =
                mech_engine::native_artifact::lower_native_program_artifact(&artifact, &catalog)
                    .unwrap();
            let prepared = PreparedNativeProgram::prepare(&artifact, &catalog, plan).unwrap();
            let [region] = prepared.metadata().generic_dictionary.regions.as_slice() else {
                panic!("one guarded owner")
            };
            let guard = region.guard.as_ref().unwrap();
            let homes = guard
                .leaves
                .iter()
                .map(|v| v.get() as usize)
                .collect::<BTreeSet<_>>();
            assert!(!homes.is_empty());
            let live = Liveness::derive(&prepared, &homes).unwrap();
            assert!(
                live.at(region.exit as usize).is_superset(&homes),
                "anticipation must not read an unspilled future home"
            );
            for site in &guard.sites {
                assert_eq!(
                    prepared
                        .metadata()
                        .generic_dictionary
                        .extra_work(*site as usize),
                    0,
                    "source guard fees remain at their original visits"
                );
            }
        }
    }
}
