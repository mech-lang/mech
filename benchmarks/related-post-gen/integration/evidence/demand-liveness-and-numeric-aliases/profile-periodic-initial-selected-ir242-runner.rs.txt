//! Diagnostic main-thread sampling; all timings excluded from benchmark comparisons.
//! Host adapter only: JSON, immutable input admission, native invocation and JSON output.
//! Index construction, counting and top-five selection live in related.mec.
use mech_core::{ManagedMemoryBudget, Value, ValueData, snapshot::SequenceView};
#[cfg(not(linked))]
use mech_engine::ProgramArtifact;
#[cfg(not(linked))]
use mech_engine::CanonicalSourceFrontend;
#[cfg(not(linked))]
use mech_gpu::native_artifact::JitProgramArtifact;
use mech_gpu::native_artifact::NativeProgramLimitMode;
#[cfg(not(linked))]
use mech_syntax::document::{AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot, parse_canonical_document};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufWriter, sync::Arc, time::Instant};

#[cfg(linked)]
include!("build/linked.rs");
#[cfg(linked)]
include!("build/catalog.rs");

#[derive(Clone, Deserialize, Serialize)]
struct Post {
    #[serde(rename = "_id")]
    id: String,
    title: String,
    tags: Vec<Box<str>>,
}
#[derive(Serialize)]
struct Related<'a> {
    #[serde(rename = "_id")]
    id: &'a str,
    tags: &'a [Box<str>],
    related: [&'a PostView<'a>; 5],
}
#[derive(Serialize)]
struct PostView<'a> {
    #[serde(rename = "_id")]
    id: &'a str,
    title: &'a str,
    tags: &'a [Box<str>],
}
fn post_views<'a>(descriptor: &mech_gpu::native_artifact::NativeProgramDescriptor, posts: &'a [Post], input: &'a [Value]) -> Vec<PostView<'a>> {
    let port = |name| descriptor.plan().inputs().iter().position(|p| mech_engine::decode_source_input_name(p.name()).as_deref() == Some(name)).unwrap();
    let offsets = indices(&input[port("offsets")]);
    let ValueData::Matrix(matrix) = input[port("tags")].data() else {panic!("expected tags matrix")};
    let SequenceView::String(tags) = matrix.elements() else {panic!("expected String tags")};
    assert_eq!(offsets.len(), posts.len()+1);
    posts.iter().enumerate().map(|(at,p)| PostView { id:&p.id, title:&p.title, tags:&tags[usize::try_from(offsets[at]).unwrap()..usize::try_from(offsets[at+1]).unwrap()] }).collect()
}
#[cfg(not(linked))]
fn source(text: &str) -> ProgramArtifact {
    let parsed = parse_canonical_document(TextSnapshot::new(DocumentId(71), Revision(1), text).unwrap(), ParseConfig::default());
    assert!(parsed.is_strictly_clean(), "source must parse without recovery");
    CanonicalSourceFrontend.compile_document_with_catalog(&DocumentSyntax::cast(parsed.syntax()).unwrap(), mech_stdlib::source_catalog()).unwrap().compile_artifact().unwrap()
}
fn inputs(descriptor: &mech_gpu::native_artifact::NativeProgramDescriptor, posts: &mut [Post], memory: &ManagedMemoryBudget) -> Vec<Value> {
    let schemas = Arc::new(descriptor.schemas().clone());
    let count=posts.iter().try_fold(0usize,|sum,post| sum.checked_add(post.tags.len())).unwrap();
    let mut offsets=Vec::with_capacity(posts.len()+1); offsets.push(0u64);
    let mut tags=Vec::with_capacity(count);
    for post in posts {
        // Transfer JSON-owned payloads into the admitted canonical input.
        // The emptied per-post vectors are released here, inside preparation.
        tags.extend(std::mem::take(&mut post.tags));
        offsets.push(tags.len() as u64);
    }
    let mut offsets=Some(offsets.into_boxed_slice()); let mut tags=Some(tags.into_boxed_slice());
    descriptor.plan().inputs().iter().map(|port| {
        let name=mech_engine::decode_source_input_name(port.name()).unwrap();
        let count=if name=="offsets" { offsets.as_ref().unwrap().len() } else { assert_eq!(name,"tags"); tags.as_ref().unwrap().len() };
        let shape=mech_core::shape_for_resolved_extents(schemas.get(port.ty().schema()).unwrap(),&[1,count as u64]).unwrap();
        let value=if name=="offsets" {
            Value::from_u64_matrix(port.ty().schema(),shape,offsets.take().unwrap(),Arc::clone(&schemas)).unwrap()
        } else {
            mech_core::snapshot::StringMatrixSnapshotConstructor::bind(port.ty().schema(),shape,Arc::clone(&schemas)).unwrap().construct(tags.take().unwrap()).unwrap()
        };
        // External input construction precedes normal canonical budget import;
        // both construction and admission remain inside the post-JSON timer.
        let mut admission=memory.reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap()).unwrap();
        value.into_memory_budget(&mut admission).unwrap()
    }).collect()
}

fn indices(value: &Value) -> &[u64] {
    let ValueData::Matrix(matrix) = value.data() else { panic!("expected indices") };
    let SequenceView::U64(values) = matrix.elements() else { panic!("expected u64 indices") };
    values
}
fn run() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 4, "runner SOURCE POSTS_JSON OUTPUT_JSON SAMPLES");
    #[cfg(not(linked))]
    let artifact = source(&std::fs::read_to_string(&args[0]).unwrap());
    #[cfg(linked)]
    assert_eq!(std::fs::read_to_string(&args[0]).unwrap(), include_str!("build/source.mec"), "source differs from the linked compiler bundle");
    let catalog = linked_catalog().unwrap();
    let mode = NativeProgramLimitMode::NoLimits;
    let compiled = Instant::now();
    #[cfg(not(linked))]
    let program = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation()).unwrap();
    #[cfg(linked)]
    // SAFETY: build.py pins the compiler libraries and links the exact object and
    // descriptor emitted together from this source. Symbols live for the process.
    let program = unsafe { mech_gpu::native_artifact::LinkedProgramArtifact::from_compiled_module(&LINKED_MODULE, &catalog) }.unwrap();
    let compile_seconds = compiled.elapsed().as_secs_f64();
    let prepare = Instant::now();
    let template: Vec<Post> = serde_json::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    assert!(!template.is_empty());
    let json_seconds = prepare.elapsed().as_secs_f64();
    let samples: usize = args[3].parse().unwrap();
    let oracle: serde_json::Value = serde_json::from_slice(&std::fs::read("/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json").unwrap()).unwrap();
    let by_id = template.iter().enumerate().map(|(i,p)|(p.id.as_str(),i as u64)).collect::<std::collections::HashMap<_,_>>();
    assert_eq!(by_id.len(), template.len());
    let expected_indices = oracle.as_array().unwrap().iter().flat_map(|post|post["related"].as_array().unwrap().iter().map(|related|by_id[related["_id"].as_str().unwrap()])).collect::<Vec<_>>();
    assert_eq!(expected_indices.len(), template.len()*5);
    assert_eq!(unsafe { sam_init() }, 0, "sampler initialization must succeed");
    for sample in 0..samples {
        let mut posts = template.clone();
        unsafe { sam_enable(1) };
    let post_json_start = Instant::now();
    let admit = Instant::now();
    let memory = mode.memory_budget();
    let input = inputs(program.descriptor(), &mut posts, &memory);
    let admission_seconds = admit.elapsed().as_secs_f64();
    // Views borrow the canonical strings through JSON output. Their allocation
    // stays inside the same continuous post-JSON preparation/execution timer.
    let posts_view = post_views(program.descriptor(), &posts, &input);
        let mut output = vec![None; program.descriptor().plan().outputs().len()];
        let start = Instant::now();
        let report = program.evaluate_into(&input, &mut output, mode.execution(), &memory).unwrap();
        let values = indices(output.last().unwrap().as_ref().unwrap());
        assert_eq!(values.len(), posts.len()*5);
        let related: Vec<_> = posts_view.iter().zip(values.chunks_exact(5)).map(|(post, top)| Related { id:&post.id, tags:&post.tags, related:std::array::from_fn(|i| &posts_view[top[i] as usize]) }).collect();
        std::hint::black_box(&related);
        let seconds = std::hint::black_box(Instant::now()).duration_since(start).as_secs_f64();
        let post_json_seconds = post_json_start.elapsed().as_secs_f64();
        unsafe { sam_enable(0) };
        assert_eq!(values, expected_indices.as_slice(), "every profiled result must match Rust");
        if sample == 0 || sample + 1 == samples { assert_eq!(serde_json::to_value(&related).unwrap(), oracle); }
        println!("{}", serde_json::json!({"sample":sample,"posts":posts.len(),"seconds":seconds,"work":report.work,"parallel_regions":report.parallel_regions,"parallel_batches":report.parallel_batches,"parallel_tasks":report.parallel_tasks,"parallel_peak_workers":report.parallel_peak_workers,"parallel_numeric_regions":report.parallel_numeric_regions,"parallel_numeric_work":report.parallel_numeric_work,"compile_or_restore_seconds":compile_seconds,"json_read_and_parse_seconds":json_seconds,"input_admission_seconds":admission_seconds,"processing_plus_admission_seconds":seconds+admission_seconds,"post_json_seconds":post_json_seconds,"post_json_boundary":"one continuous interval after JSON decoding through input admission, intervening setup, output-slot allocation, native invocation, internal cleanup and RelatedPosts materialization; restoration and JSON IO remain in complete-process time","linked":cfg!(linked),"retained_bytes":memory.used_bytes(),"boundary":"native evaluate_into (including internal cleanup) + output index export + RelatedPosts materialization; excludes JSON IO, native input admission, compilation/restoration and post-evaluation output/input destruction"}));
        if sample+1==samples {
            serde_json::to_writer(BufWriter::new(File::create(&args[2]).unwrap()), &related).unwrap();
        }
        drop(related);
        drop(output);
        drop(posts_view);
        drop(input);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
    let binary = std::ffi::CString::new(format!("{}.samples.bin", args[2])).unwrap();
    let stats = std::ffi::CString::new(format!("{}.sampler.json", args[2])).unwrap();
    let symbols = std::ffi::CString::new(format!("{}.symbols.tsv", args[2])).unwrap();
    assert_eq!(unsafe { sam_finish(binary.as_ptr(),stats.as_ptr(),symbols.as_ptr()) },0);
}
fn main() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(run).unwrap().join().unwrap();
}
unsafe extern "C" {
    fn sam_init() -> i32;
    fn sam_enable(value: i32);
    fn sam_finish(binary: *const std::ffi::c_char, stats: *const std::ffi::c_char, symbols: *const std::ffi::c_char) -> i32;
}
