use super::*;
use crate::native_artifact::program::prepare::{
    PreparedNativeProgram, matrix_map_keys::tests::artifact,
};

fn prepared(a: &mech_engine::ProgramArtifact) -> PreparedNativeProgram {
    let catalog = mech_stdlib::source_catalog();
    let plan = mech_engine::native_artifact::lower_native_program_artifact(a, &catalog).unwrap();
    PreparedNativeProgram::prepare(a, &catalog, plan).unwrap()
}
fn source(seed: &str, index: &str, next: &str) -> String {
    format!(
        r#"
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
"#
    )
}

#[test]
fn completed_element_ranges_require_every_append_checked_and_an_empty_seed() {
    let good = source("empty", "key + 1u64", "counted(output,ids[at + 1u64])");
    for (text, eligible) in [
        (good, true),
        (
            source(
                "sequence/append(empty,99u64)",
                "key + 1u64",
                "counted(output,ids[at + 1u64])",
            ),
            false,
        ),
        (
            source("empty", "key + 2u64", "counted(output,ids[at + 1u64])"),
            false,
        ),
        (
            source(
                "empty",
                "key + 1u64",
                "(sequence/append(output.1,ids[at + 1u64]),output.2)",
            ),
            false,
        ),
    ] {
        let original = artifact(&text);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        let mut before = None;
        for a in [&original, &decoded] {
            let p = prepared(a);
            let facts = producer_candidates(p.metadata());
            assert_eq!(!facts.is_empty(), eligible, "{text}\n{facts:?}");
            if let Some(old) = &before {
                assert_eq!(old, &facts);
            }
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
            let additions = &p.completed_key_additions;
            println!("{}", serde_json::json!({"completed_key_additions": additions, "compiler_only_cache": true, "bounds_elided": false}));
            for header in [796,1530] {
                assert!(additions.additions.iter().any(|a|p.metadata().closed_tails.regions[a.region].tail.header() as usize == header), "missing actual arithmetic consumer {header}: {additions:?}");
            }
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

pub(in crate::native_artifact) fn consumer_source(
    seed: &str,
    index: &str,
    next: &str,
    values: &str,
) -> String {
    let scan = r#"completed-sum(values<[u64]>, at<u64>, end<u64>, sum<u64>) => (out<u64>) :=
  more := at < end
  next := (more? | true => sum + (values[at + 1u64] + 1u64) | false => sum)
  out := (more? | true => completed-sum(values,at + 1u64,end,next) | false => sum).
"#;
    let base = source(seed, index, next);
    base.replacen(
        "build(ids<[u64]>, buckets<u64>) => (out<[u64]>) :=",
        &format!("{scan}build(ids<[u64]>, buckets<u64>) => (out<u64>) :="),
        1,
    )
    .replacen(
        "out := built.1.",
        &format!("out := completed-sum({values},0u64,sequence/length({values}),0u64)."),
        1,
    )
}

#[test]
fn completed_key_additions_bind_only_checked_completed_immutable_inputs_and_exact_operands() {
    for (text, eligible) in [
        (consumer_source("empty","key + 1u64","counted(output,ids[at + 1u64])","built.1"),true),
        (consumer_source("sequence/append(empty,18446744073709551615u64)","key + 1u64","counted(output,ids[at + 1u64])","built.1"),false),
        (consumer_source("empty","key + 1u64","(sequence/append(output.1,ids[at + 1u64]),output.2)","built.1"),false),
        (consumer_source("empty","0u64 + 1u64","counted(output,ids[at + 1u64])","built.1"),false),
        (consumer_source("empty","key + 1u64","counted(output,ids[at + 1u64])","ids"),false),
        (consumer_source("empty","key + 1u64","((ids[at + 1u64] == 0u64)? | true => counted(output,ids[at + 1u64]) | false => (sequence/append(output.1,ids[at + 1u64]),output.2))","built.1"),false),
        (consumer_source("empty","key + 1u64","counted(output,ids[at + 1u64])","changed").replacen("out := completed-sum(changed", "changed := set(built.1,1u64,18446744073709551615u64)\n  out := completed-sum(changed",1),false),
    ] {
        let original = artifact(&text);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
        let mut previous = None;
        for artifact in [&original,&decoded] {
            let p=prepared(artifact);let layout=&p.completed_key_additions;
            assert_eq!(!layout.additions.is_empty(),eligible,"{text}\n{layout:?}");
            for addition in &layout.additions {
                let Action::Scalar(Scalar::Primitive{op,operands,..})=p.plan().blocks()[addition.site].action()else{panic!("addition source")};
                assert!(layout.arithmetic(addition.region,addition.site,*op,operands));
                assert!(!layout.arithmetic(addition.region+1,addition.site,*op,operands));
                assert!(!layout.arithmetic(addition.region,addition.site+1,*op,operands));
                assert!(!layout.arithmetic(addition.region,addition.site,Op::U64WrappingAdd,operands));
                let reversed=operands.iter().rev().copied().collect::<Vec<_>>();
                assert!(!layout.arithmetic(addition.region,addition.site,*op,&reversed));
            }
            if let Some(previous)=&previous{assert_eq!(previous,layout)}
            previous=Some(layout.clone());
        }
    }
}


#[test]
#[ignore = "requires caller-pinned full benchmark source"]
fn completed_key_additions_actual_source_emits_only_arithmetic_elision() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let text=std::fs::read_to_string(std::env::var("MECH_COMPLETED_ELEMENTS_SOURCE").unwrap()).unwrap();
        let original=artifact(&text);
        let decoded=mech_engine::decode_program_artifact_bytecode_v1(&mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
        let catalog=mech_stdlib::source_catalog();let mut previous=None;
        for artifact in [&original,&decoded] {
            let default=crate::native_artifact::emit_program_object(artifact,&catalog).unwrap();
            let control=crate::native_artifact::program::compiler::emit_program_object_for_test(artifact,&catalog,crate::native_artifact::codegen::options::OptimizationPass::CompletedKeyAdditions).unwrap();
            let a=default.metadata();let b=control.metadata();let mut additions=Vec::new();
            for key in["compiled_numeric_write_loops","compiled_readonly_numeric_loops"] {
                let left=a[key].as_array().unwrap();let right=b[key].as_array().unwrap();assert_eq!(left.len(),right.len());
                for (x,y)in left.iter().zip(right) {
                    assert_eq!(x["header"],y["header"]);
                    for unchanged in["typed_loads","typed_stores","proven_scalar_read_guards","owned_handoff_boundaries","proven_checked_increment_writes"] {assert_eq!(x[unchanged],y[unchanged],"{key} {unchanged}")}
                    assert_eq!(y["proven_completed_key_additions"].as_array().unwrap().len(),0);
                    let sites=x["proven_completed_key_additions"].as_array().unwrap();
                    if !sites.is_empty(){additions.push((x["header"].as_u64().unwrap(),sites.clone()))}
                }
            }
            for header in[796,1530]{assert!(additions.iter().any(|(h,_)|*h==header),"{additions:?}")}
            if let Some(old)=&previous{assert_eq!(old,&additions)}previous=Some(additions.clone());
            println!("{}",serde_json::json!({"emitted_completed_key_additions":additions,"array_guard_metadata_unchanged":true,"producer_ownership_handoff_unchanged":true,"default_code_bytes":a["code_bytes"],"control_code_bytes":b["code_bytes"],"timing_eligible":false}));
        }
    }).unwrap().join().unwrap()
}
