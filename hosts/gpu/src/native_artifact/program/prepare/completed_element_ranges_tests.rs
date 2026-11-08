use super::*;
use crate::native_artifact::program::prepare::{PreparedNativeProgram, matrix_map_keys::tests::artifact};

fn prepared(a: &mech_engine::ProgramArtifact) -> PreparedNativeProgram {
    let catalog = mech_stdlib::source_catalog();
    let plan = mech_engine::native_artifact::lower_native_program_artifact(a, &catalog).unwrap();
    PreparedNativeProgram::prepare(a, &catalog, plan).unwrap()
}
fn source(seed: &str, index: &str, next: &str) -> String {
    format!(r#"
+> sequence
<pair> := <([u64],[u64])>
set(values<[u64]>, at<u64>, value<u64>) => (out<[u64]>) :=
  ~next := values
  next[at] = value
  out := next.
counted(output<pair>, key<u64>) => (out<pair>) :=
  keys := sequence/append(output.1, key)
  counts := set(output.2, {index}, output.2[{index}] + 1u64)
  out := (keys,counts).
builder(ids<[u64]>, at<u64>, end<u64>, output<pair>) => (out<pair>) :=
  more := at < end
  next := (more? | true => {next} | false => output)
  out := (more? | true => builder(ids,at + 1u64,end,next) | false => output).
build(ids<[u64]>, buckets<u64>) => (out<[u64]>) :=
  empty := sequence/with-capacity(0u64,sequence/length(ids))
  seed := {seed}
  counts := [0u64 | i <- 1u64..=buckets]
  built := builder(ids,0u64,sequence/length(ids),(seed,counts))
  out := built.1.
build(ids<[u64]>,buckets<u64>)
"#)
}

#[test]
fn completed_element_ranges_require_every_append_checked_and_an_empty_seed() {
    let good = source("empty", "key + 1u64", "counted(output,ids[at + 1u64])");
    for (text, eligible) in [
        (good, true),
        (source("sequence/append(empty,99u64)", "key + 1u64", "counted(output,ids[at + 1u64])"), false),
        (source("empty", "key + 2u64", "counted(output,ids[at + 1u64])"), false),
        (source("empty", "key + 1u64", "(sequence/append(output.1,ids[at + 1u64]),output.2)"), false),
    ] {
        let original = artifact(&text);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
        let mut before = None;
        for a in [&original, &decoded] {
            let p = prepared(a);
            let facts = producer_candidates(p.metadata());
            assert_eq!(!facts.is_empty(), eligible, "{text}\n{facts:?}");
            if let Some(old) = &before { assert_eq!(old, &facts); }
            before = Some(facts);
        }
    }
}

#[test]
#[ignore = "requires caller-pinned full benchmark source"]
fn completed_element_ranges_actual_source_validates_all_triple_and_quartet_appends() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let text = std::fs::read_to_string(std::env::var("MECH_COMPLETED_ELEMENTS_SOURCE").unwrap()).unwrap();
        let original = artifact(&text);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
        let mut before = None;
        for a in [&original, &decoded] {
            let p = prepared(a);
            let facts = producer_candidates(p.metadata());
            println!("{}", serde_json::json!({"successful_producer_element_ranges":facts,"applicability_only":true,"native_check_removal":false,"timing_eligible":false}));
            for (header, appends) in [(272,15), (1037,7)] {
                let f = facts.iter().find(|f| f.producer == header && f.array == 1 && f.bound == 2).expect("actual producer range missing");
                assert_eq!(f.checked_appends.len(), appends);
            }
            if let Some(old) = &before { assert_eq!(old, &facts); }
            before = Some(facts);
        }
    }).unwrap().join().unwrap();
}
