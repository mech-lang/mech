
use mech_syntax::document::{DocumentId, Revision, TextSnapshot, ParseConfig, parse_canonical_document, GreenNode, GreenElement};
use sha2::{Digest, Sha256};
use std::{path::PathBuf,time::Instant};
fn tree(node:&GreenNode,h:&mut Sha256) {
    h.update([0]);h.update(node.id.0.to_le_bytes());h.update((node.kind as u64).to_le_bytes());
    h.update(node.text_len.0.to_le_bytes());h.update(node.flags.0.to_le_bytes());
    h.update(node.structural_hash.to_le_bytes());h.update((node.children.len() as u64).to_le_bytes());
    for child in node.children.iter() {
        match child {
            GreenElement::Node(n)=>tree(n,h),
            GreenElement::Token(t)=>{h.update([1]);h.update(t.id.0.to_le_bytes());h.update((t.kind as u64).to_le_bytes());h.update(t.text_len.0.to_le_bytes());h.update(t.flags.0.to_le_bytes());h.update(t.text_hash.to_le_bytes());}
        }
    }
    h.update([2]);
}
fn run() {
    let args:Vec<_>=std::env::args().collect();
    let source=std::fs::read_to_string(&args[1]).unwrap();
    let repeats:usize=args.get(3).map(|x| x.parse().unwrap()).unwrap_or(1);
    for iteration in 0..repeats {
    let start=Instant::now();
    let parsed=parse_canonical_document(TextSnapshot::new(DocumentId(0x50524f47),Revision(1),source.as_str()).unwrap(),ParseConfig::default());
    assert!(parsed.is_strictly_clean(),"{:?}",parsed.diagnostics);
    let elapsed=start.elapsed().as_secs_f64();
    let mut digest=Sha256::new();tree(&parsed.root,&mut digest);
    let hash=digest.finalize().iter().map(|v| format!("{v:02x}")).collect::<String>();
    let report=serde_json::json!({"parse_seconds":elapsed,"tree_sha256":hash,"source_bytes":source.len(),"source_sha256":Sha256::digest(source.as_bytes()).iter().map(|v| format!("{v:02x}")).collect::<String>(),"parser_steps":parsed.stats.parser_steps,"events":parsed.stats.events_emitted});
    std::fs::write(PathBuf::from(&args[2]).with_extension(format!("{iteration}.json")),serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}
#[path = "/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP/src/compiler_allocator.rs"]
mod compiler_allocator;
fn main(){std::thread::Builder::new().stack_size(32*1024*1024).spawn(run).unwrap().join().unwrap();}
