//! Complete collection-rooted source inventories reuse selected child proofs.
use super::*;
use crate::native_artifact::program::prepare::numeric_generators;

fn layout(jit: &JitProgramArtifact) -> numeric_generators::Layout {
    let m = jit.descriptor().metadata();
    numeric_generators::Layout::derive(
        &m.plan,
        &m.buffer_frames,
        &m.scalar_aggregates,
        &m.closed_tails,
    )
    .unwrap()
}
pub(super) fn source(end: &str) -> String {
    format!(
        r#"#Scan(source<[f64]:3,1>, a<f64>, end<u64>) => <[f64]:3,1>
  | :Walk(values<[f64]:3,1>, at<u64>, prior<f64>)
  | :Done(values<[f64]:3,1>).
#Scan(source,a,end) -> :Walk([0.0 | j <- 1u64..=3u64]',1u64,0.0)
  :Walk(values,at,prior)
    | at < end -> {{
        value := a * source[at] + prior
        ~next := values
        next[at] = value
      }} -> :Walk(next,at + 1u64,value)
    | * -> :Done(values)
  :Done(values) => values.
image := input<[f64]:2,3>
answer := [(#Scan([image[row,column] | column <- 1u64..=3u64]',a<f64>,{end})) | row <- 1u64..=2u64]'
(answer,image)
"#
    )
}

#[test]
fn numeric_generators_compose_bounded_array_children_and_maps_with_closed_source_ownership() {
    for artifact in artifacts(&source("4u64")) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let inventory = layout(&jit);
        assert!(inventory.declined.is_empty(), "{inventory:?}");
        assert_eq!(inventory.regions.len(), 1);
        let object =
            crate::native_artifact::emit_program_object(&artifact, &mech_stdlib::source_catalog())
                .unwrap();
        let report = object.metadata();
        let candidates = &report["numeric_generator_candidates"];
        assert_eq!(
            candidates["selected"], true,
            "ordinary selection retains independent current finite admission"
        );
        assert_eq!(
            candidates["inventory"]["regions"].as_array().unwrap().len(),
            1
        );
        let r = &inventory.regions[0];
        assert_eq!(r.extents, [3, 2]);
        assert_eq!(r.block_extents, [3, 1]);
        assert_eq!(r.count, 2);
        assert_eq!(r.children.len(), 1);
        assert_eq!(r.maps.len(), 2);
        assert_eq!(r.children[0].trips.maximum, 6);
        assert!(r.maximum_visits > r.sites.len() as u64);
        assert!(r.captures.len() >= 2);
        let m = jit.descriptor().metadata();
        let child = &m.closed_tails.regions[r.children[0].region];
        assert!(
            child
                .numeric
                .as_ref()
                .unwrap()
                .sites
                .iter()
                .all(|s| r.sites.contains(&(*s as u32)))
        );
        assert!(
            r.maps
                .iter()
                .flat_map(|map| map.source_sites())
                .all(|s| r.sites.contains(&s))
        );
        assert!(r.order.windows(2).all(|p| p[0] != p[1]));
    }
}

#[test]
fn numeric_generators_decline_unbounded_children_without_changing_the_ordinary_program() {
    for artifact in artifacts(&source("end<u64>")) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let inventory = layout(&jit);
        assert!(inventory.regions.is_empty());
        assert_eq!(inventory.declined.len(), 1);
        assert!(
            inventory.declined[0].1.contains("fixed trip envelope"),
            "{inventory:?}"
        );
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|r| r
                    .numeric
                    .as_ref()
                    .is_some_and(|p| p.exits.is_empty() && p.result.direct_array().is_some()))
        );
    }
}

#[test]
fn numeric_generators_inventory_both_optimization_fixture_roots_and_every_child() {
    let source = include_str!("../../../../../tests/fixtures/polybench/deriche.mec");
    for artifact in artifacts(source) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let inventory = layout(&jit);
        assert!(inventory.declined.is_empty(), "{inventory:?}");
        assert_eq!(inventory.regions.len(), 2);
        for r in &inventory.regions {
            assert_eq!(r.extents, [64, 64]);
            assert_eq!(r.block_extents, [64, 1]);
            assert_eq!(r.count, 64);
            assert_eq!(
                r.local_values.len(),
                87,
                "both roots retain every scalar, including terminator-only definitions"
            );
            let m = jit.descriptor().metadata();
            for &site in &r.sites {
                let output = match m.plan.blocks()[site as usize].action() {
                    mech_engine::native_artifact::NativeIterationAction::Scalar(
                        mech_engine::native_artifact::NativeMatchAction::Copy { output, .. }
                        | mech_engine::native_artifact::NativeMatchAction::Primitive {
                            output, ..
                        },
                    )
                    | mech_engine::native_artifact::NativeIterationAction::Structural {
                        output,
                        ..
                    }
                    | mech_engine::native_artifact::NativeIterationAction::LibraryCall {
                        output,
                        ..
                    } => Some(*output),
                    _ => None,
                };
                if let Some(output) = output {
                    assert!(
                        r.local_values.contains(&output.get()),
                        "native definition lost at {site}: {output:?}"
                    );
                }
            }
            assert_eq!(r.children.len(), 2);
            assert_eq!(r.maps.len(), 4);
            assert_eq!(
                r.children
                    .iter()
                    .map(|c| c.trips.maximum)
                    .collect::<Vec<_>>(),
                [67, 66]
            );
        }
    }
}

#[test]
fn numeric_generators_keep_external_boolean_values_read_only_by_terminators() {
    let text = conditional_source();
    for artifact in artifacts(&text) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let m = jit.descriptor().metadata();
        let gate = m
            .plan
            .inputs()
            .iter()
            .find(|i| mech_engine::decode_source_input_name(i.name()).as_deref() == Some("gate"))
            .unwrap()
            .value()
            .get();
        let inventory = layout(&jit);
        assert!(inventory.declined.is_empty(), "{inventory:?}");
        assert_eq!(inventory.regions.len(), 1);
        assert!(inventory.regions[0].captures.contains(&gate));
    }
}

pub(super) fn conditional_source() -> String {
    source("4u64").replace("answer := [(#Scan", "answer := [((gate<bool>? | true => (#Scan")
        .replace(")) | row", ")) | false => (#Scan([image[row,column] | column <- 1u64..=3u64]',a<f64> * 2.0,4u64)))) | row")
}
