use mech_core::{ValueData, ValueDataDraft as Data};
use mech_core::snapshot::{SnapshotValidationContext, ValueDraft, SequenceView};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use mech_syntax::document::{AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot, parse_canonical_document};
#[path = "/private/tmp/mech-deflate-linear-stack/src/core/tests/support/allocation_probe.rs"]
mod allocation_probe;
#[global_allocator]
static ALLOCATOR: allocation_probe::ProbeAllocator = allocation_probe::ProbeAllocator;

fn artifact(kind: &str, stores: usize) -> ProgramArtifact {
    let mut source = format!("touch(buffer<[{kind}]>) => (out<[{kind}]>) :=\n  ~next := buffer\n");
    for at in 1..=stores { source.push_str(&format!("  next[{at}u64] = 7{kind}\n")); }
    source.push_str(&format!("  out := next.\ntouch(input<[{kind}]>)\n"));
    let parsed = parse_canonical_document(TextSnapshot::new(DocumentId(0x50524f46), Revision(1), source.as_str()).unwrap(), ParseConfig::default());
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    CanonicalSourceFrontend.compile_document_with_catalog(&syntax, mech_stdlib::source_catalog()).unwrap().compile_artifact().unwrap()
}
fn run() {
    let (_, requests) = allocation_probe::measured(|| ());
    assert_eq!(requests, 0);
    let mode=NativeProgramLimitMode::NoLimits;
    for kind in ["u16", "u32"] {
        for stores in [1, 8] {
            let artifact=artifact(kind, stores);
            let jit=JitProgramArtifact::compile_with_limits(&artifact,&mech_stdlib::source_catalog(),mode.compilation()).unwrap();
            let lengths: &[usize]=if kind=="u16" { &[288, 262144] } else { &[288, 1500513] };
            for &len in lengths {
                let memory=mode.memory_budget();
                let schema=artifact.inputs()[0].schema;
                let shape=mech_core::shape_for_resolved_extents(artifact.schemas().get(schema).unwrap(), &[1,len as u64]).unwrap();
                let element=if kind=="u16" {Data::U16(0)} else {Data::U32(0)};
                let value=ValueDraft{schema,shape_values:shape.parameter_values().to_vec().into_boxed_slice(),data:Data::Matrix(vec![element;len].into())}.finalize(&SnapshotValidationContext::new(artifact.schemas())).unwrap();
                let mut reservation=memory.reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap()).unwrap();
                let value=value.into_memory_budget(&mut reservation).unwrap();drop(reservation);
                let input_alias=value.clone();
                let args=vec![value];
                let mut out=vec![None;artifact.outputs().len()];
                for rep in 0..3 {
                    for slot in &mut out {*slot=None;}
                    let started=std::time::Instant::now();
                    let (result, requests, bytes)=allocation_probe::measured_with_bytes(||jit.evaluate_into(&args,&mut out,mode.execution(),&memory));
                    let seconds=started.elapsed().as_secs_f64();let report=result.unwrap();
                    let ValueData::Matrix(m)=out.last().unwrap().as_ref().unwrap().data() else {panic!()};
                    match m.elements() {
                        SequenceView::U16(words)=>{assert_eq!(words.len(),len);assert!(words[..stores].iter().all(|&n|n==7));assert!(words[stores..].iter().all(|&n|n==0));},
                        SequenceView::U32(words)=>{assert_eq!(words.len(),len);assert!(words[..stores].iter().all(|&n|n==7));assert!(words[stores..].iter().all(|&n|n==0));},
                        _=>panic!(),
                    }
                    println!("{{\"kind\":\"{kind}\",\"words\":{len},\"stores\":{stores},\"rep\":{rep},\"seconds\":{seconds},\"allocations\":{requests},\"requested_bytes\":{bytes},\"visits\":{},\"work\":{},\"input_alias_kept\":true}}",report.visits,report.work);
                }
                assert!(args[0].shares_frozen_storage(&input_alias));
                let ValueData::Matrix(m)=input_alias.data() else {panic!()};
                match m.elements(){SequenceView::U16(v)=>assert!(v.iter().all(|&n|n==0)),SequenceView::U32(v)=>assert!(v.iter().all(|&n|n==0)),_=>panic!()};
                drop((args,input_alias,out));assert_eq!(memory.used_bytes(),0);assert_eq!(memory.retained_snapshot_nodes(),0);
            }
        }
    }
}
fn main(){std::thread::Builder::new().stack_size(32*1024*1024).spawn(run).unwrap().join().unwrap();}
