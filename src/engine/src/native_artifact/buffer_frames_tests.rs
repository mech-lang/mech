use super::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::Arc;

fn plan_results(
    source: &str,
) -> [Result<Plan, crate::native_artifact::NativeArtifactLoweringError>; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x42554646), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let mut catalog = mech_core::FunctionCatalogBuilder::new();
    crate::install_intrinsic_runtime(&mut catalog).unwrap();
    crate::install_intrinsic_compiler_runtime(&mut catalog).unwrap();
    crate::install_intrinsic_resident(&mut catalog).unwrap();
    crate::install_intrinsic_source(&mut catalog).unwrap();
    let catalog = Arc::new(catalog.build().unwrap());
    let original = crate::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
        .map(|artifact| super::super::lower_native_program_artifact(&artifact, &catalog))
}

fn plans(source: &str) -> [Plan; 2] {
    plan_results(source).map(Result::unwrap)
}

fn scalar_span(frames: &NativeBufferFrames, field: NativeBufferFrameField) -> bool {
    match field.lane() {
        NativeBufferFrameLane::Scalar { .. } => true,
        NativeBufferFrameLane::Composite { .. } => frames
            .children(field)
            .unwrap()
            .iter()
            .all(|field| scalar_span(frames, *field)),
        NativeBufferFrameLane::Managed { .. } | NativeBufferFrameLane::Tagged { .. } => false,
    }
}

#[test]
fn closed_scalar_children_are_spans_of_private_mixed_frames() {
    for source in [
        "frame := (input<[u8]>, (seed<u8>, 7u64))\n(buffer, state) := frame\nbyte<u64> := buffer[1u64]\nscore<u64> := state.1\nbyte + score + state.2\n",
        "frame := {buffer: input<[u8]>, state: {score: seed<u8>, rank: 7u64}}\nbuffer := frame.buffer\nstate := frame.state\nbyte<u64> := buffer[1u64]\nscore<u64> := state.score\nbyte + score + state.rank\n",
        "frame := (input<[u8]>, (seed<u8>, (7u64, true)))\n(buffer, state) := frame\nbyte<u64> := buffer[1u64]\nscore<u64> := state.1\nbyte + score + state.2.1\n",
    ] {
        for plan in plans(source) {
            let frames = plan.facts().buffer_frames();
            assert!(
                frames.slots.iter().enumerate().any(|(value, _)| {
                    let Some(fields) = frames.fields_at(value) else {
                        return false;
                    };
                    fields
                        .iter()
                        .any(|f| matches!(f.lane(), NativeBufferFrameLane::Managed { .. }))
                        && fields.iter().any(|field| {
                            matches!(field.lane(), NativeBufferFrameLane::Composite { .. })
                                && scalar_span(&frames, *field)
                        })
                }),
                "missing embedded scalar span: {source}"
            );
        }
    }
}

#[test]
fn standalone_scalar_aggregate_keeps_its_scalar_owner() {
    for plan in plans("state := (seed<u8>, 7u64)\nscore<u64> := state.1\nscore + state.2\n") {
        assert_eq!(plan.facts().buffer_frames().elements(), 0);
        assert!(plan.scalar_aggregates().value_count() > 0);
    }
}

#[test]
fn an_escaping_mixed_frame_cannot_hide_scalar_child_publication() {
    for plan in plans("frame := (input<[u8]>, (seed<u8>, 7u64))\nframe\n") {
        assert_eq!(plan.facts().buffer_frames().elements(), 0);
    }
}

const TAGGED_MAP: &str = r#"
+> map
read-word(index<{string:u64}>, count<u64>) => (out<u64>) :=
  words := {index: index, count: count}
  out := words.index{"needle"} + words.count.
#Carry(initial<{string:u64}>, limit<u64>) => <u64>
  | :Next(at<u64>, index<{string:u64}>, count<u64>)
  | :Done(index<{string:u64}>, count<u64>).
#Carry(initial, limit) -> :Next(0u64, initial, 0u64)
  :Next(at, index, count)
    | at < limit -> :Next(at + 1u64, index, count + 1u64)
    | * -> :Done(index, count)
  :Done(index, count) => read-word(index, count).
#Carry(initial<{string:u64}>, limit<u64>)
"#;

#[test]
fn private_tagged_maps_have_disjoint_variant_payload_spans() {
    for plan in plans(TAGGED_MAP) {
        let frames = plan.facts().buffer_frames();
        let mut tags = 0;
        for value in 0..plan.values().len() {
            let Some([field]) = frames.fields_at(value) else {
                continue;
            };
            let NativeBufferFrameLane::Tagged { tag, at, count } = field.lane() else {
                continue;
            };
            assert!(tag < frames.scalar_fields());
            assert_eq!(count, 2);
            let variants = frames.children(*field).unwrap();
            assert_eq!(variants.len(), count);
            assert_eq!(
                frames.tagged_variant(*field, 0).unwrap().schema(),
                variants[0].schema()
            );
            assert!(frames.tagged_variant(*field, 2).is_none());
            let ranges = variants
                .iter()
                .map(|v| {
                    let NativeBufferFrameLane::Composite { at, count } = v.lane() else {
                        panic!("payload was not private")
                    };
                    at..at + count
                })
                .collect::<Vec<_>>();
            assert!(ranges[0].end <= ranges[1].start || ranges[1].end <= ranges[0].start);
            assert!(at < frames.elements());
            tags += 1;
        }
        assert!(tags > 0, "no private tagged state");
        assert!(
            frames
                .actions
                .iter()
                .flatten()
                .any(|a| matches!(a.operation(), NativeBufferFrameOperation::EnumPack { .. }))
        );
        assert!(frames.actions.iter().flatten().any(|a| matches!(
            a.operation(),
            NativeBufferFrameOperation::Observe {
                observation: Observation::EnumOrdinal { .. },
                ..
            }
        )));
        assert!(
            frames
                .actions
                .iter()
                .flatten()
                .any(|a| matches!(a.operation(), NativeBufferFrameOperation::Project { .. }))
        );
    }
}

#[test]
fn shallow_map_records_keep_opaque_current_owner_lanes() {
    for plan in plans(
        "+> map\nframe := {index: initial<{string:u64}>, count: seed<u64>}\ncopy := frame.index\ncopy{\"needle\"} + frame.count\n",
    ) {
        let frames = plan.facts().buffer_frames();
        assert!(frames.managed_fields() > 0);
        assert!(frames.slots.iter().enumerate().any(|(v, _)| {
            matches!(plan.values()[v].definition().body(), SchemaBody::Record(_))
                && frames.fields_at(v).is_some_and(|fields| {
                    fields.iter().any(|field| {
                        tagged::opaque_map(
                            plan.values()
                                .iter()
                                .find(|ty| ty.schema() == field.schema())
                                .unwrap()
                                .definition()
                                .body(),
                        ) && matches!(field.lane(), NativeBufferFrameLane::Managed { .. })
                    })
                })
        }));
    }
}

#[test]
fn public_map_frame_keeps_ordinary_canonical_publication() {
    for source in [
        "frame := (initial<{string:u64}>, seed<u64>)\nframe\n",
        "frame := {index: initial<{string:u64}>, count: seed<u64>}\nframe\n",
    ] {
        for plan in plans(source) {
            assert_eq!(plan.facts().buffer_frames().elements(), 0);
        }
    }
}

#[test]
fn pure_scalar_tagged_state_keeps_scalar_aggregate_owner() {
    let source = r#"
#Count(limit<u64>) => <u64>
  | :Next(at<u64>, total<u64>)
  | :Done(total<u64>).
#Count(limit) -> :Next(0u64, 0u64)
  :Next(at, total)
    | at < limit -> :Next(at + 1u64, total + 2u64)
    | * -> :Done(total)
  :Done(total) => total.
#Count(limit<u64>)
"#;
    for plan in plans(source) {
        assert_eq!(plan.facts().buffer_frames().elements(), 0);
        assert!(plan.scalar_aggregates().value_count() > 0);
    }
}

#[test]
fn map_owner_recipe_declines_nested_and_dynamic_entries() {
    let map = |value| SchemaBody::Map {
        key: Box::new(SchemaBody::String),
        value: Box::new(value),
        cardinality: mech_core::ExtentSpec::Dynamic { upper_bound: None },
    };
    assert!(tagged::opaque_map(&map(SchemaBody::UnsignedInteger(
        IntegerWidth::W64
    ))));
    assert!(tagged::opaque_map(&map(SchemaBody::String)));
    assert!(!tagged::opaque_map(&map(SchemaBody::Dynamic)));
    assert!(!tagged::opaque_map(&map(map(SchemaBody::UnsignedInteger(
        IntegerWidth::W64
    )))));
    assert!(!tagged::opaque_map(&map(SchemaBody::Tuple(
        [SchemaBody::UnsignedInteger(IntegerWidth::W64)].into()
    ))));
}

#[path = "buffer_frames_admission_tests.rs"]
mod admission;

#[test]
fn map_owner_and_independent_tagged_component_share_a_canonical_seed() {
    let owned = r#"
+> map
update-owner(h<{string:u64}>) => (out<{string:u64}>) :=
  one := map/insert(h, "one", 11u64)
  out := map/insert(one, "two", 22u64).
#Owner(initial<{string:u64}>) => <{string:u64}>
  | :Run(h<{string:u64}>)
  | :Done(h<{string:u64}>).
#Owner(initial) -> :Run(initial)
  :Run(h)
    | * -> :Done(update-owner(h))
  :Done(h) => h.
#Owner(initial<{string:u64}>)
"#;
    let source = format!("{owned}\n{}", TAGGED_MAP.replace("+> map", ""));
    for plan in plans(&source) {
        let facts = plan.facts();
        let (frames, owners) = facts.buffer_frame_owner_layouts().unwrap();
        assert!(!owners.regions().is_empty());
        let claimed = owners
            .regions()
            .iter()
            .flat_map(|r| r.values().iter().copied())
            .map(Value::get)
            .collect::<std::collections::BTreeSet<_>>();
        let added_slots = (0..plan.values().len())
            .filter_map(|i| {
                let value = Value(i as u32);
                (owners.aggregates().slot(value).is_some()
                    && facts.scalar_aggregates().slot(value).is_none())
                .then_some(value.get())
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            claimed, added_slots,
            "every additional Map-owner slot has one exact exclusion seed"
        );
        assert!(
            frames
                .actions
                .iter()
                .flatten()
                .any(|a| matches!(a.operation(), NativeBufferFrameOperation::EnumPack { .. }))
        );
        for value in owners.regions().iter().flat_map(|r| r.values()) {
            assert!(
                frames.slot(*value).is_none(),
                "one value cannot have two owners"
            );
        }
        let defaults = facts.buffer_frames();
        for value in 0..plan.values().len() {
            assert_eq!(frames.slot_at(value), defaults.slot_at(value));
        }
        let canonical_maps = (0..plan.transport_input_count())
            .map(|i| plan.transport_input_value(i))
            .filter(|v| {
                matches!(
                    plan.values()[v.get() as usize].definition().body(),
                    SchemaBody::Map { .. }
                )
            })
            .collect::<Vec<_>>();
        assert!(!canonical_maps.is_empty());
        for value in canonical_maps {
            assert!(
                owners
                    .regions()
                    .iter()
                    .all(|r| !r.values().contains(&value)),
                "external seed stays canonical"
            );
        }
    }
}

#[test]
fn retained_map_owner_publishes_flat_record_or_tuple_results() {
    let source = r#"
+> map
<dictionary> := <{index<{string:u64}> count<u64>}>
#Collect(initial<{string:u64}>, limit<u64>) => <dictionary>
  | :Next(index<{string:u64}>, count<u64>)
  | :Done(index<{string:u64}>, count<u64>).
#Collect(initial, limit) -> :Next(initial, 0u64)
  :Next(index, count)
    | count < limit -> :Next(map/insert(index, "needle", count), count + 1u64)
    | * -> :Done(index, count)
  :Done(index, count) => {index: index, count: count}.
words := #Collect(initial<{string:u64}>, limit<u64>)
words
"#;
    let tuple = source
        .replace(
            "<dictionary> := <{index<{string:u64}> count<u64>}>",
            "<dictionary> := <({string:u64},u64)>",
        )
        .replace("{index: index, count: count}.", "(index, count).");
    for source in [source, &tuple] {
        for plan in plans(source) {
            let (frames, owners) = plan.facts().buffer_frame_owner_layouts().unwrap();
            assert_eq!(owners.regions().len(), 1);
            let region = &owners.regions()[0];
            assert!(owners.aggregates().slot(region.tail().returned()).is_some());
            assert!(
                owners.aggregates().slot(region.tail().result()).is_none(),
                "canonical result is not a private native token"
            );
            for value in region.values() {
                assert!(frames.slot(*value).is_none(), "owner selection is disjoint");
                assert!(
                    matches!(
                        plan.values()[value.get() as usize].definition().body(),
                        SchemaBody::Map { .. }
                            | SchemaBody::Record(_)
                            | SchemaBody::Tuple(_)
                            | SchemaBody::Enum { .. }
                    ),
                    "scalar never becomes a Map owner word"
                );
            }
        }
    }
}

#[test]
fn an_old_map_record_alias_blocks_private_mutation() {
    let source = r#"
+> map
<dictionary> := <{index<{string:u64}> count<u64>}>
#Alias(initial<{string:u64}>, limit<u64>) => <dictionary>
  | :Next(index<{string:u64}>, count<u64>)
  | :Done(index<{string:u64}>, count<u64>).
#Alias(initial, limit) -> :Next(initial, 0u64)
  :Next(index, count)
    | count < limit -> {
        old := {index: index, count: count}
        next := map/insert(index, "needle", count)
        observed := old.index{"needle"}
      } -> :Done(next, observed)
    | * -> :Done(index, count)
  :Done(index, count) => {index: index, count: count}.
#Alias(initial<{string:u64}>, limit<u64>)
"#;
    let early = source
        .replace(
            "next := map/insert",
            "prior-index := old.index\n        next := map/insert",
        )
        .replace("observed := old.index{", "observed := prior-index{");
    let copied = source
        .replace(
            "next := map/insert",
            "saved := old\n        next := map/insert",
        )
        .replace("observed := old.index{", "observed := saved.index{");
    for source in [
        source.to_owned(),
        early,
        copied,
        source
            .replace(
                "old := {index: index, count: count}",
                "old := (index, count)",
            )
            .replace("old.index{", "old.1{"),
    ] {
        for plan in plans(&source) {
            assert!(plan.map_owner_words().unwrap().regions().is_empty());
            let writes = plan.map_write_uses().unwrap();
            assert!(!writes.is_empty());
            assert!(
                writes.iter().any(|write| !write.late_reads().is_empty()),
                "Record/Tuple projection must keep the old Map version live"
            );
        }
    }
}

#[test]
fn recursive_map_reads_respect_one_exact_current_visit_predicate() {
    let source = r#"
+> map
walk(at<u64>, end<u64>, other-end<u64>, index<{string:u64}>) => (out<{string:u64}>) :=
  more := at < end
  next := (more? | true => map/insert(index, "needle", at) | false => index)
  again := more
  out := (again? | true => walk(at + 1u64, end, other-end, next) | false => index).
walk(0u64, end<u64>, other-end<u64>, initial<{string:u64}>)
"#;
    for plan in plans(source) {
        let writes = plan.map_write_uses().unwrap();
        assert_eq!(writes.len(), 1);
        assert!(
            writes[0].late_reads().is_empty(),
            "contradictory repeat test: {writes:?}"
        );
        assert_eq!(plan.map_owner_words().unwrap().regions().len(), 1);
    }
    for different in [
        "at < other-end",
        "((at < other-end)? | true => more | false => false)",
        "at + 1u64 < end",
    ] {
        for plan in plans(&source.replace("again := more", &format!("again := {different}"))) {
            let writes = plan.map_write_uses().unwrap();
            assert_eq!(writes.len(), 1);
            assert!(
                !writes[0].late_reads().is_empty(),
                "distinct/phi predicate: {different}"
            );
            assert!(plan.map_owner_words().unwrap().regions().is_empty());
        }
    }
}

#[test]
fn record_payload_binding_keeps_its_required_structural_draft_witness() {
    let source = r#"
+> map
<dictionary> := <{index<{string:u64}> count<u64>}>
#Carry(initial<{string:u64}>, limit<u64>) => <dictionary>
  | :Next(words<dictionary>)
  | :Done(words<dictionary>).
#Carry(initial, limit) -> :Next({index: initial, count: 0u64})
  :Next(words)
    | words.count < limit -> {
        next := map/insert(words.index, "needle", words.count)
      } -> :Next({index: next, count: words.count + 1u64})
    | * -> :Done(words)
  :Done(words) => words.
#Carry(initial<{string:u64}>, limit<u64>)
"#;
    for result in plan_results(source) {
        assert!(
            matches!(
                result,
                Err(
                    crate::native_artifact::NativeArtifactLoweringError::UnsupportedNode {
                        feature: "structural child requires a native draft expansion witness",
                        ..
                    }
                )
            ),
            "a Record payload binding still needs its canonical label/draft witness"
        );
    }
}

#[test]
fn bounded_map_alias_analysis_declines_without_rejecting_unrelated_deep_records() {
    let mut source = String::from(
        r#"
+> map
#Owner(initial<{string:u64}>) => <{string:u64}>
  | :Run(index<{string:u64}>)
  | :Done(index<{string:u64}>).
#Owner(initial) -> :Run(initial)
  :Run(index) | * -> :Done(map/insert(index, "needle", 7u64))
  :Done(index) => index.
#Owner(initial<{string:u64}>)
r0 := {leaf: seed<u64>}
"#,
    );
    for i in 1..=34 {
        source.push_str(&format!("r{i} := {{next: r{}}}\n", i - 1));
    }
    source.push_str("r34\n");
    for plan in plans(&source) {
        assert!(matches!(
            plan.map_write_uses(),
            Err("native map alias schema depth")
        ));
        assert!(plan.map_owner_words().unwrap().regions().is_empty());
        assert!(plan.facts().buffer_frame_owner_layouts().is_ok());
    }
}
