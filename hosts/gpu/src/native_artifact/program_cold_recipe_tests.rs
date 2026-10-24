//! Execute exact cold reconstruction independently of the cursor/steady pass.
use super::*;
use std::collections::BTreeSet;

#[test]
fn cold_recipes_bind_actual_live_words_initial_and_completed_source_edges() {
    let artifact = tests::artifact_source(&format!(
        "{}\n#Count(\"abcabc\",2u64,6u64,empty)\n",
        structural_projection_tests::COUNT
    ));
    let catalog = mech_stdlib::source_catalog();
    take_cold_recipe_emissions();
    let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
    let emissions = take_cold_recipe_emissions();
    let metadata = object.metadata();
    let loops = metadata["compiled_steady_loops"].as_array().unwrap();
    assert_eq!(loops.len(), 1);
    let records = loops[0]["cold_snapshot_recipes"].as_array().unwrap();
    assert!(!records.is_empty());
    assert_eq!(emissions, records.len());
    let mut boundaries = BTreeSet::new();
    let mut completed = false;
    let mut initial = false;
    let mut body = false;
    for record in records {
        use crate::native_artifact::program_packed_map::Frame as PackedFrame;
        assert_eq!(record["reload_words"], serde_json::json!([
            core::mem::offset_of!(PackedFrame, source_ptr) / 8,
            core::mem::offset_of!(PackedFrame, width) / 8,
        ]));
        let node = record["node"].as_u64().unwrap();
        let header = record["header"].as_bool().unwrap();
        let incoming = record["incoming"].as_u64();
        assert!(boundaries.insert((node, header, incoming)));
        let continuation = loops[0]["continuations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry[0].as_u64() == Some(node + 1))
            .unwrap();
        let live = continuation[2].as_array().unwrap();
        for word in record["words"].as_array().unwrap() {
            assert!(live.contains(&word[0]));
            assert!(word[1].is_u64());
        }
        if header {
            assert_eq!(record["updates"], 0);
            if let Some(edge) = incoming {
                completed = true;
                assert!(
                    loops[0]["cursor_recurrence_edges"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|value| value.as_u64() == Some(edge))
                );
                assert_eq!(record["previous"], u64::MAX);
            } else {
                initial = true;
                assert!(record["previous"].is_null());
            }
        } else {
            body = true;
            assert!(incoming.is_none());
        }
    }
    assert!(initial && completed && body);
    let original =
        without_cold_recipes(|| crate::native_artifact::emit_program_object(&artifact, &catalog))
            .unwrap();
    assert_eq!(take_cold_recipe_emissions(), 0);
    assert!(
        !original.metadata()["compiled_steady_loops"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for region in original.metadata()["compiled_steady_loops"]
        .as_array()
        .unwrap()
    {
        assert!(
            region["cold_snapshot_recipes"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn cold_recipes_preserve_changed_inputs_complete_maps_and_exact_work() {
    changed_input_equivalence(Pass::ColdRecipes);
}

#[test]
fn cold_recipes_preserve_original_overflows_nonempty_seeds_and_declined_proofs() {
    arithmetic_failure_equivalence(Pass::ColdRecipes);
}
