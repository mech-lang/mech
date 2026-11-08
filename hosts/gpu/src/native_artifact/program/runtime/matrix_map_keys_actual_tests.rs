//! Explicit retained-input diagnostic, excluded from the self-contained CI set.
use super::composition_tests::matrix_map_keys::{Fault, run};
use super::*;
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

#[test]
#[ignore = "requires caller-pinned full source, input and oracle paths"]
fn matrix_map_keys_full_input_selects_28_borrowed_cells_with_exact_results() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let read = |name| std::fs::read_to_string(std::env::var(name).unwrap()).unwrap();
        let source = read("MECH_MATRIX_KEY_BENCH_SOURCE");
        let parsed = parse_canonical_document(TextSnapshot::new(DocumentId(0x4d4b4655), Revision(1), source.as_str()).unwrap(), ParseConfig::default());
        let catalog = mech_stdlib::source_catalog();
        let artifact = mech_engine::CanonicalSourceFrontend.compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(), catalog.clone()).unwrap().compile_artifact().unwrap();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let posts: serde_json::Value = serde_json::from_str(&read("MECH_MATRIX_KEY_BENCH_INPUT")).unwrap();
        let posts = posts.as_array().unwrap();
        let oracle: serde_json::Value = serde_json::from_str(&read("MECH_MATRIX_KEY_BENCH_ORACLE")).unwrap();
        assert_eq!(posts.len(), 5000);
        let mut tags = Vec::new(); let mut offsets = vec![ValueDataDraft::U64(0)];
        for post in posts {
            tags.extend(post["tags"].as_array().unwrap().iter().map(|s| ValueDataDraft::String(s.as_str().unwrap().into())));
            offsets.push(ValueDataDraft::U64(tags.len() as u64));
        }
        let memory = NativeProgramLimitMode::NoLimits.memory_budget();
        let args: Vec<Value> = artifact.inputs().iter().map(|port| {
            let name = mech_engine::decode_source_input_name(&port.name).unwrap();
            let cells = match name.as_str() { "tags" => tags.clone(), "offsets" => offsets.clone(), _ => panic!("unexpected input {name}") };
            let schema = artifact.schemas().get(port.schema).unwrap();
            let shape = mech_core::shape_for_resolved_extents(schema, &[1, cells.len() as u64]).unwrap();
            let value = ValueDraft { schema: port.schema, shape_values: shape.parameter_values().into(), data: ValueDataDraft::Matrix(cells.into()) }
                .finalize(&SnapshotValidationContext::new(artifact.schemas())).unwrap();
            let mut reservation = memory.reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap()).unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        }).collect();
        for restored in [false, true] {
            let mut output = vec![None; artifact.outputs().len()];
            let (result, counts) = run(&native, &args, &mut output, &memory, restored, Fault::None);
            result.unwrap(); assert_eq!(counts, (28,28,0,28));
            let ValueData::Matrix(matrix) = output.last().unwrap().as_ref().unwrap().data() else { panic!("result matrix"); };
            let mech_core::snapshot::SequenceView::U64(indices) = matrix.elements() else { panic!("U64 result"); };
            assert_eq!(indices.len(), 25000);
            for (row, expected) in indices.chunks_exact(5).zip(oracle.as_array().unwrap()) {
                for (index, expected) in row.iter().zip(expected["related"].as_array().unwrap()) {
                    assert_eq!(&posts[*index as usize], expected);
                }
            }
            println!("{}", serde_json::json!({"full_input_posts":posts.len(),"restored":restored,"producer_callbacks":counts.0,"borrowed_cells":counts.1,"canonical_string_values":counts.2,"retained_insertions":counts.3,"exact_related_posts":true,"timings_ineligible":true}));
        }
        drop(args); assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0,0));
    }).unwrap().join().unwrap();
}
