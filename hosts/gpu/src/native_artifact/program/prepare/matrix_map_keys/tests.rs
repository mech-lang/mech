use super::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

pub(in crate::native_artifact::program::prepare) fn source(key: &str, value: &str, next: &str, seed: &str) -> String {
    format!(
        r#"
+> map
+> sequence
renumber(keys<[string]>, at<u64>, end<u64>, index<{{string:u64}}>) => (out<{{string:u64}}>) :=
  more := at < end
  next := (more? | true => map/insert(index, keys[{key}], {value}) | false => index)
  out := (more? | true => renumber(keys, {next}, end, next) | false => index).
build(tags<[string]>, first<u64>, last<u64>) => (out<{{string:u64}}>) :=
  other<{{string:u64}}> := map/new()
  histogram := map/histogram(tags)
  keys := map/keys(histogram)
  out := renumber(keys, first, last, {seed}).
build(tags<[string]>, first<u64>, last<u64>)
"#
    )
}

pub(in crate::native_artifact::program::prepare) fn artifact(source: &str) -> mech_engine::ProgramArtifact {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4f52444d), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    mech_engine::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap()
}
fn derive(artifact: &mech_engine::ProgramArtifact) -> Layout {
    let plan = mech_engine::native_artifact::lower_native_program_artifact(
        artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    Layout::derive(
        &plan,
        Some(&plan.facts().buffer_frame_owner_layouts().unwrap().1),
    )
    .unwrap()
}

#[test]
fn matrix_map_keys_prove_generic_adjacent_insertions_and_preserve_decoded_proofs() {
    for (key, value, next, seed) in [
        ("at + 1u64", "at", "at + 1u64", "histogram"),
        ("at + 2u64", "at", "at + 3u64", "other"),
    ] {
        let original = artifact(&source(key, value, next, seed));
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        let a = derive(&original);
        assert_eq!(a.regions.len(), 1, "{a:?}");
        assert_eq!(a, derive(&decoded));
        assert_eq!(a.regions[0].consumer, a.regions[0].producer + 1);
    }
}

#[test]
fn matrix_map_keys_refuse_an_escaping_key_with_two_insertion_consumers() {
    assert!(
        derive(&artifact(&source(
            "at + 1u64",
            "at + 19u64",
            "at + 1u64",
            "histogram"
        )))
        .regions
        .is_empty()
    );
    let source = r#"
+> map
update(keys<[string]>, at<u64>, end<u64>, index<{string:u64}>) => (out<{string:u64}>) :=
  key := keys[at + 1u64]
  first := map/insert(index, key, at)
  second := map/insert(first, key, at + 1u64)
  more := at < end
  out := (more? | true => update(keys, at + 1u64, end, second) | false => second).
update(keys<[string]>, first<u64>, last<u64>, index<{string:u64}>)
"#;
    assert!(derive(&artifact(source)).regions.is_empty());
}

#[test]
#[ignore = "requires retained benchmark source explicitly supplied by caller"]
fn matrix_map_keys_selects_actual_current_related_post_source() {
    let source =
        std::fs::read_to_string(std::env::var("MECH_MATRIX_KEY_BENCH_SOURCE").unwrap()).unwrap();
    let layout = derive(&artifact(&source));
    assert_eq!(layout.regions.len(), 2, "{layout:?}");
    let r = layout.regions.iter().find(|r| r.producer == 61).unwrap();
    assert_eq!((r.producer, r.consumer, r.source.get()), (61, 62, 66));
}
