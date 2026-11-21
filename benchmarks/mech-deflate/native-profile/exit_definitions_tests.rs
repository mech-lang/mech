//! Compare the exact production selection with the former whole-value loop.
#[path = "../../../hosts/gpu/src/native_artifact/program/prepare/numeric_exit_definitions.rs"]
mod definitions;

use mech_engine::native_artifact::{NativeProgramArtifactPlan as Plan, NativeProgramFacts};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::collections::BTreeSet;

fn plans(source: &str) -> ([Plan; 2], mech_core::SchemaTable) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x45584954), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let catalog = mech_stdlib::source_catalog();
    let original = mech_engine::CanonicalSourceFrontend
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
    let schemas = original.schemas().clone();
    (
        [original, decoded].map(|artifact| {
            mech_engine::native_artifact::lower_native_program_artifact(&artifact, &catalog)
                .unwrap()
        }),
        schemas,
    )
}

fn reference(
    facts: &NativeProgramFacts<'_>,
    ordinals: &[usize],
    reads: &[BTreeSet<u32>],
    wanted: Option<&BTreeSet<u32>>,
) -> Result<Vec<Vec<u32>>, &'static str> {
    let mut read_values = vec![false; facts.plan().values().len()];
    for &value in reads.iter().flatten() {
        if let Some(read) = read_values.get_mut(value as usize) {
            *read = true;
        }
    }
    let mut definitions = vec![Vec::<u32>::new(); reads.len()];
    for value in 0..facts.plan().values().len() {
        if wanted.is_some_and(|wanted| !wanted.contains(&(value as u32))) {
            continue;
        }
        let sites = facts.definition_sites(value as u32)?;
        if !read_values[value] {
            continue;
        }
        for definition in sites {
            if let Some(site) = definition.site()
                && let Some(&index) = ordinals.get(site as usize)
                && index != usize::MAX
            {
                let killed = &mut definitions[index];
                if killed.last() != Some(&(value as u32)) {
                    killed.push(value as u32);
                }
            }
        }
    }
    Ok(definitions)
}

fn same(
    plan: &Plan,
    ordinals: &[usize],
    reads: &[BTreeSet<u32>],
    wanted: Option<&BTreeSet<u32>>,
) -> Result<Vec<Vec<u32>>, &'static str> {
    // Independent facts owners ensure one query cannot initialize the other's table.
    let actual = definitions::selected(&plan.facts(), ordinals, reads, wanted);
    assert_eq!(actual, reference(&plan.facts(), ordinals, reads, wanted));
    actual
}

fn restore(image: &serde_json::Value, schemas: &mech_core::SchemaTable) -> Plan {
    // SAFETY: Test metadata stays confined to descriptive queries. It never
    // enters execution, emission, linking or admission as a validated program.
    unsafe {
        mech_engine::native_artifact::decode_build_owned_native_plan_body(
            &serde_json::to_vec(image).unwrap(),
            schemas,
            &mech_stdlib::source_catalog(),
        )
        .unwrap()
    }
}

#[test]
fn selected_definitions_match_original_and_decoded_graph_subsets() {
    let sources = [
        "0u64\n",
        "left<u64> + right<u64>\n",
        "[i | i <- 0u64..count<u64>]\n",
        include_str!("../../../hosts/gpu/tests/fixtures/native/numeric-nested-phase.mec"),
        include_str!("../../../hosts/gpu/tests/fixtures/native/numeric-private-fixed-nested.mec"),
    ];
    let mut state = 0x584954444546u64;
    let mut random = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for source in sources {
        for plan in plans(source).0 {
            assert!(!plan.values().is_empty());
            for _ in 0..512 {
                let mut selected = (0..plan.blocks().len())
                    .filter(|_| random() % 3 != 0)
                    .collect::<Vec<_>>();
                selected.reverse();
                let mut ordinals = vec![usize::MAX; plan.blocks().len()];
                for (index, &site) in selected.iter().enumerate() {
                    ordinals[site] = index;
                }
                let mut reads = vec![BTreeSet::new(); selected.len()];
                for values in &mut reads {
                    for _ in 0..random() % 8 {
                        values.insert((random() % (plan.values().len() as u64 + 3)) as u32);
                    }
                    if random() & 1 == 1 {
                        values.insert(u32::MAX);
                    }
                }
                let wanted = (0..plan.values().len() as u32 + 3)
                    .filter(|_| random() % 4 == 0)
                    .chain([u32::MAX])
                    .collect::<BTreeSet<_>>();
                same(&plan, &ordinals, &reads, None).unwrap();
                same(&plan, &ordinals, &reads, Some(&wanted)).unwrap();
                same(&plan, &ordinals, &reads, Some(&BTreeSet::new())).unwrap();
            }
        }
    }
}

#[test]
fn unread_invalid_producers_keep_the_first_definition_error() {
    let (variants, schemas) = plans("left<u64> + right<u64>\n");
    for plan in variants {
        let mut image = serde_json::to_value(&plan).unwrap();
        let last = image["blocks"].as_array().unwrap().len() - 1;
        image["blocks"][last]["action"] =
            serde_json::json!({"Scalar":{"Copy":{"source":0,"output":u32::MAX}}});
        let malformed = restore(&image, &schemas);
        assert!(malformed.facts().definition_sites(0).is_err());
        let ordinals = vec![usize::MAX; malformed.blocks().len()];
        let reads = vec![BTreeSet::new()];
        assert!(same(&malformed, &ordinals, &reads, None).is_err());
        assert!(same(&malformed, &ordinals, &reads, Some(&BTreeSet::from([1]))).is_err());
        same(&malformed, &ordinals, &reads, Some(&BTreeSet::new())).unwrap();
        same(
            &malformed,
            &ordinals,
            &reads,
            Some(&BTreeSet::from([u32::MAX])),
        )
        .unwrap();
        // Out-of-range reads were ignored by the old bitmap, even if wanted.
        let invalid_reads = vec![BTreeSet::from([u32::MAX])];
        same(
            &malformed,
            &ordinals,
            &invalid_reads,
            Some(&BTreeSet::from([u32::MAX])),
        )
        .unwrap();
        assert!(same(&malformed, &ordinals, &invalid_reads, None).is_err());
    }
}

#[test]
fn zero_values_preserve_the_absence_of_definition_demand() {
    let ([plan, _], schemas) = plans("0u64\n");
    let mut image = serde_json::to_value(&plan).unwrap();
    image["values"] = serde_json::json!([]);
    image["value_scopes"] = serde_json::json!([]);
    let malformed = restore(&image, &schemas);
    assert!(malformed.facts().definition_sites(0).is_err());
    let ordinals = vec![usize::MAX; malformed.blocks().len()];
    let reads = vec![BTreeSet::from([0, u32::MAX])];
    same(&malformed, &ordinals, &reads, None).unwrap();
    same(
        &malformed,
        &ordinals,
        &reads,
        Some(&BTreeSet::from([0, u32::MAX])),
    )
    .unwrap();
}

#[test]
fn duplicate_records_and_nonlocal_definitions_keep_order_and_set_semantics() {
    let ([plan, _], schemas) = plans("left<u64> + right<u64>\n");
    let mut image = serde_json::to_value(&plan).unwrap();
    let value = plan.inputs()[0].value().get();
    // Give the input two identical records at one selected block, another
    // record at an unselected block, and retain its site-less input record.
    let mut block = image["blocks"][0].clone();
    block["action"] = serde_json::json!({"Scalar":{"Copy":{"source":value,"output":value}}});
    let blocks = image["blocks"].as_array_mut().unwrap();
    blocks.push(block.clone());
    blocks.push(block);
    let changed = restore(&image, &schemas);
    let mut ordinals = vec![usize::MAX; changed.blocks().len()];
    ordinals[changed.blocks().len() - 2] = 0;
    ordinals[changed.blocks().len() - 1] = 0;
    let reads = vec![BTreeSet::from([value, u32::MAX])];
    assert_eq!(
        same(&changed, &ordinals, &reads, None).unwrap(),
        vec![vec![value]]
    );
    let truncated = &ordinals[..ordinals.len() - 1];
    assert_eq!(
        same(&changed, truncated, &reads, None).unwrap(),
        vec![vec![value]]
    );
}

#[test]
fn direct_output_inventory_is_sorted_unique_and_shared_within_one_owner() {
    for source in [
        "left<u64> + right<u64>\n",
        include_str!("../../../hosts/gpu/tests/fixtures/native/numeric-nested-phase.mec"),
        include_str!("../../../hosts/gpu/tests/fixtures/native/numeric-private-fixed-nested.mec"),
    ] {
        let [original, decoded] = plans(source).0;
        let original_facts = original.facts();
        let decoded_facts = decoded.facts();
        let outputs = original_facts.direct_action_outputs();
        assert!(!outputs.is_empty());
        assert_eq!(outputs, decoded_facts.direct_action_outputs());
        assert!(std::ptr::eq(
            outputs,
            original_facts.direct_action_outputs()
        ));
        assert!(outputs.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(outputs.iter().all(|(id, value)| *id == value.get()));
    }
}

#[test]
fn direct_output_inventory_keeps_duplicate_and_sparse_descriptive_outputs() {
    let ([plan, _], schemas) = plans("left<u64> + right<u64>\n");
    let mut image = serde_json::to_value(&plan).unwrap();
    let mut block = image["blocks"][0].clone();
    let blocks = image["blocks"].as_array_mut().unwrap();
    blocks.clear();
    for output in [u32::MAX, 0, 17, 17, u32::MAX, 3] {
        block["action"] = serde_json::json!({"Scalar":{"Copy":{"source":0,"output":output}}});
        blocks.push(block.clone());
    }
    let malformed = restore(&image, &schemas);
    let facts = malformed.facts();
    assert_eq!(
        facts
            .direct_action_outputs()
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>(),
        vec![0, 3, 17, u32::MAX]
    );
    // Enumeration must not hide the original whole-inventory validation error.
    assert!(facts.definition_sites(0).is_err());
    assert!(malformed.facts().definition_sites(0).is_err());
    assert_eq!(facts.direct_action_outputs().len(), 4);
}

#[test]
fn direct_output_inventory_does_not_cross_plan_owners() {
    let ([plan, _], schemas) = plans("left<u64> + right<u64>\n");
    let facts = plan.facts();
    let original = facts.direct_action_outputs().to_vec();
    let mut image = serde_json::to_value(&plan).unwrap();
    for block in image["blocks"].as_array_mut().unwrap() {
        block["action"] = serde_json::json!({"Scalar":"None"});
    }
    let other = restore(&image, &schemas);
    assert!(other.facts().direct_action_outputs().is_empty());
    assert_eq!(facts.direct_action_outputs(), original);
}
