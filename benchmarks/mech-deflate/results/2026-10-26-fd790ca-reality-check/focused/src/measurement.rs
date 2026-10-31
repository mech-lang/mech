//! Measurement harness only: committed compiler/source behavior is unchanged.
use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;
use std::time::Instant;
const KERNEL: &str = concat!(include_str!("/private/tmp/mech-deflate-reality-check/hosts/gpu/tests/fixtures/native/deflate-matchfinder.mec"), "\n", include_str!("/private/tmp/mech-deflate-reality-check/hosts/gpu/tests/fixtures/native/deflate-binary-tree.mec"));
const COLLECTOR: &str = include_str!("/private/tmp/mech-deflate-reality-check/hosts/gpu/tests/fixtures/native/deflate-collector.mec");
fn matrix16(entries: &[(usize,u16)]) -> Data {
    let mut words=vec![Data::U16(0);262144];
    for &(i,v) in entries { words[i]=Data::U16(v); }
    Data::Matrix(words.into())
}
fn matrix32(entries: &[(usize,u32)]) -> Data {
    let mut words=vec![Data::U32(0);1500513];
    for &(i,v) in entries { words[i]=Data::U32(v); }
    Data::Matrix(words.into())
}
pub fn measure() {
    let selected=std::env::var("MECH_REALITY_CASE").unwrap_or_else(|_| "all".into());
    let reps:usize=std::env::var("MECH_REALITY_REPS").ok().map(|s|s.parse().unwrap()).unwrap_or(5);
    for case in ["initialization","search","skip","six-position"] {
        if selected!="all" && selected!=case { continue; }
        let source=match case {
            "initialization"=>format!("{KERNEL}\n{COLLECTOR}\noptimal-collect-prefix(input<[u8]>, 0u64)\n"),
            "six-position"=>format!("{KERNEL}\n{COLLECTOR}\noptimal-collect-prefix(input<[u8]>, 6u64)\n"),
            "search"=>format!("{KERNEL}\nbt-get-matches(tables<[u16]>, hash3<u64>, hash4<u64>, input<[u8]>, base<u64>, position<u64>, maximum<u64>, nice<u64>, depth<u64>, cache<[u32]>, pointer<u64>)\n"),
            "skip"=>format!("{KERNEL}\nbt-skip-byte(tables<[u16]>, hash3<u64>, hash4<u64>, input<[u8]>, base<u64>, position<u64>, nice<u64>, depth<u64>)\n"),
            _=>unreachable!(),
        };
        let compile_start=Instant::now();
        let ([artifact,_],_)=artifacts(&source);
        let jit=JitProgramArtifact::compile(&artifact,&mech_stdlib::source_catalog()).unwrap();
        println!("{}",serde_json::json!({"engine":"mech","case":case,"source_and_jit_compile_seconds":compile_start.elapsed().as_secs_f64()}));
        let memory=ManagedMemoryBudget::new(256*1024*1024);
        let before=if case=="search" || case=="skip" {matrix16(&[(127616,32768),(174848,32768)])} else {Data::Matrix(Vec::new().into())};
        let cache=if case=="search" {matrix32(&[])} else {Data::Matrix(Vec::new().into())};
        let args=inputs(&artifact,&[
            ("input",Data::Matrix(vec![Data::U8(97);320].into())),
            ("tables",before.clone()),("cache",cache.clone()),
            ("hash3",Data::U64(21888)),("hash4",Data::U64(62080)),
            ("base",Data::U64(0)),("position",Data::U64(1)),
            ("maximum",Data::U64(258)),("nice",Data::U64(258)),
            ("depth",Data::U64(300)),("pointer",Data::U64(3)),
        ],&memory);
        let mut policy=limits();
        policy.library.max_output_elements=Some(1500513);
        policy.library.max_compute_work=Some(10_000_000_000);
        policy.library.max_comparison_work=Some(10_000_000_000);
        policy.execution.max_work=10_000_000_000;
        let baseline=memory.used_bytes();
        // One warmup plus exactly reps samples; no prior output is retained.
        for rep in 0..=reps {
            let mut output=vec![None;artifact.outputs().len()];
            let start=Instant::now();
            let (result,allocations,bytes)=allocation_probe::measured_with_bytes(||jit.evaluate_into(&args,&mut output,policy,&memory));
            let elapsed=start.elapsed().as_nanos();
            println!("{}",serde_json::json!({"engine":"mech","case":case,"rep":rep,"warmup":rep==0,"nanoseconds":elapsed,"allocations":allocations,"requested_bytes":bytes,"result":format!("{result:?}")}));
            if case=="six-position" {
                assert_eq!(format!("{:?}",result.as_ref().unwrap_err()), "Value(Base(WorkLimit { phase: Setup, required: 10000306764, limit: 10000000000 }))");
                assert!(output.iter().all(Option::is_none));
                assert_eq!(memory.used_bytes(),baseline);
            } else {
                result.unwrap();
                let Data::Record(fields)=output.last().unwrap().as_ref().unwrap().canonical_data_draft().unwrap() else {panic!("state")};
                let field=|name:&str| &fields.iter().find(|f|f.name==name).unwrap().value;
                let expected=if case=="initialization" {matrix16(&[])} else {matrix16(&[(127616,32769),(174848,32769),(174849,32768)])};
                assert_eq!(field("tables"),&expected);
                if case=="search" {assert_eq!(field("cache"),&matrix32(&[(3,65539),(4,65794)]));assert_eq!(field("cache-ptr"),&Data::U64(5));}
                if case=="initialization" {assert_eq!(field("cache"),&matrix32(&[]));assert_eq!(field("next-position"),&Data::U64(0));}
                else {for(name,value)in[("next-hash3",21888),("next-hash4",62080)]{assert_eq!(field(name),&Data::U64(value));}}
                for(name,expected)in[("tables",&before),("cache",&cache)] {
                    if let Some(i)=artifact.inputs().iter().position(|i|mech_engine::decode_source_input_name(&i.name).unwrap()==name) {assert_eq!(&args[i].canonical_data_draft().unwrap(),expected);}
                }
            }
            drop(output);
            assert_eq!(memory.used_bytes(),baseline);
        }
        drop(args);
        assert_eq!(memory.used_bytes(),0);
    }
}
