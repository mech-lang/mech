use super::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
fn plans(source: &str) -> [Plan; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x43454c4c), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let catalog = mech_stdlib::source_catalog();
    let original = mech_engine::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
        .map(|a| mech_engine::native_artifact::lower_native_program_artifact(&a, &catalog).unwrap())
}
#[test]
fn affine_map_cells_prove_unchanged_training_updates_and_piecewise_prefix() {
    for (name, minimum, paths) in [("gramschmidt", 4, 2), ("durbin", 1, 3)] {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/polybench/{name}.mec")),
        )
        .unwrap();
        for plan in plans(&source) {
            let layout = Layout::derive(&plan).unwrap();
            let cells = layout
                .regions
                .iter()
                .filter_map(|r| CellProgram::derive(&plan, r))
                .collect::<Vec<_>>();
            assert!(cells.len() >= minimum, "{name}: {cells:#?}");
            assert!(
                cells.iter().any(|c| c.paths.len() == paths),
                "{name}: {cells:#?}"
            );
            assert!(cells.iter().flat_map(|c| &c.paths).any(|p| {
                p.values
                    .iter()
                    .any(|v| matches!(v, CellValue::Binary(NativeScalarOp::F64Mul, _)))
            }));
        }
    }
}
#[test]
fn affine_map_cells_prove_affine_reverse_rectangles_and_all_axis_relations() {
    let affine = "a := left<[f64]>\nb := right<[f64]>\n[a[i * scale<u64> + shift<u64>] - gain<f64> * b[bound<u64> - i] | i <- first<u64>..last<u64>]\n";
    let rectangle = "a := left<[f64]:3,5>\nx := rows<[f64]:3,1>\ny := columns<[f64]:1,5>\n[[a[i,j] - x[i] * y[j] | i <- 1u64..=3u64]' | j <- 1u64..=5u64]\n";
    for source in [affine, rectangle] {
        for plan in plans(source) {
            let layout = Layout::derive(&plan).unwrap();
            let recipe = layout
                .regions
                .iter()
                .filter(|r| source != rectangle || r.rectangle.is_some())
                .find_map(|r| CellProgram::derive(&plan, r))
                .expect("closed affine cells");
            assert_eq!(recipe.paths.len(), 1);
            assert!(!recipe.paths[0].reads.is_empty());
        }
    }
    for op in ["==", "!=", "<", "<=", ">", ">="] {
        let source = format!(
            "a := data<[f64]:7,1>\n[((i {op} cut<u64>)? | true => a[i] * gain<f64> | false => a[i]) | i <- 1u64..=7u64]\n"
        );
        for plan in plans(&source) {
            let layout = Layout::derive(&plan).unwrap();
            let recipe = layout
                .regions
                .iter()
                .find_map(|r| CellProgram::derive(&plan, r))
                .unwrap();
            assert_eq!(recipe.paths.len(), 2, "{op}");
            assert_eq!(
                recipe.paths[0].predicates[0],
                recipe.paths[1].predicates[0].inverted()
            );
        }
    }
}
#[test]
fn affine_map_cells_decline_unproved_control_nonaffine_reads_and_forged_metadata() {
    for body in [
        "a[i * i]",
        "a[i / divisor<u64>]",
        "((a[i] > gain<f64>)? | true => a[i] | false => 0.0)",
    ] {
        let source = format!("a := data<[f64]:8,1>\n[{body} | i <- 1u64..=3u64]\n");
        for plan in plans(&source) {
            let layout = Layout::derive(&plan).unwrap();
            assert!(!layout.regions.is_empty());
            assert!(
                layout
                    .regions
                    .iter()
                    .all(|r| CellProgram::derive(&plan, r).is_none()),
                "{body}"
            );
        }
    }
    for plan in plans("a := data<[f64]:8,1>\n[a[i] + a[bound<u64> - i] | i <- 1u64..=3u64]\n") {
        let layout = Layout::derive(&plan).unwrap();
        let r = layout
            .regions
            .iter()
            .find(|r| CellProgram::derive(&plan, r).is_some())
            .unwrap();
        let mut forged = r.clone();
        forged.matrices[0].width = 4;
        assert!(CellProgram::derive(&plan, &forged).is_none());
        let mut forged = r.clone();
        forged.reads[1].indices[0] = r.element;
        assert!(CellProgram::derive(&plan, &forged).is_none());
        let mut forged = r.clone();
        forged.result = r.reads[0].output;
        assert!(CellProgram::derive(&plan, &forged).is_none());
    }
}
