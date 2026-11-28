use super::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
fn plans(source: &str) -> [Plan; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x444f5450), Revision(1), source).unwrap(),
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
fn ordered_map_dot_proofs_reconstruct_all_five_source_dots_and_independent_projection_parent() {
    for (name, count, batches) in [("gramschmidt", 2, 1), ("durbin", 1, 0), ("cholesky", 2, 0)] {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/polybench/{name}.mec")),
        )
        .unwrap();
        for plan in plans(&source) {
            let layout = Layout::derive(&plan).unwrap();
            let dots = layout
                .regions
                .iter()
                .filter_map(|r| Dot::derive(&plan, r, None))
                .collect::<Vec<_>>();
            assert_eq!(dots.len(), count, "{name}");
            let grouped = layout
                .regions
                .iter()
                .filter_map(|r| Batch::derive(&plan, r))
                .collect::<Vec<_>>();
            assert_eq!(grouped.len(), batches, "{name}");
            if name == "gramschmidt" {
                let batch = &grouped[0];
                let predicate: &Predicate = batch.predicate.as_ref().unwrap();
                assert_eq!(predicate.op, NativeScalarOp::U64Gt);
                assert!(batch.predicate.as_ref().unwrap().dot_when_true);
                assert!(batch.alternative.is_some());
                assert!(!batch.dot.operands[0].indices[0].dependencies[1]);
                assert!(batch.dot.operands[1].indices[1].dependencies[1]);
            }
        }
    }
}
#[test]
fn ordered_map_dot_proofs_accept_affine_scaling_and_reject_nonlinear_extra_arithmetic_and_forged_reads()
 {
    let source = "+> stats\na := left<[f64]:32,1>\nb := right<[f64]:32,1>\nstats/sum/column([a[i * scale<u64> + shift<u64>] * b[bound<u64> - i] | i <- first<u64>..last<u64>])[1u64]\n";
    for plan in plans(source) {
        let layout = Layout::derive(&plan).unwrap();
        let r = layout
            .regions
            .iter()
            .find(|r| r.reduction.is_some())
            .unwrap();
        let dot = Dot::derive(&plan, r, None).expect("affine scaled and reversed streams");
        assert!(dot.operands.iter().all(|o| o.indices[0].dependencies[0]));
        let index: &Index = &dot.operands[0].indices[0];
        assert!(matches!(
            index.expr,
            Expr::Binary {
                op: NativeScalarOp::U64Add,
                ..
            }
        ));
        let mut forged = r.clone();
        forged.matrices[0].width = 4;
        assert!(Dot::derive(&plan, &forged, None).is_none());
        let mut forged = r.clone();
        forged.reads[0].indices[0] = r.element;
        assert!(Dot::derive(&plan, &forged, None).is_none());
        let mut forged = r.clone();
        forged.result = r.reads[0].output;
        assert!(Dot::derive(&plan, &forged, None).is_none());
    }
    for term in [
        "a[i * i] * b[i]",
        "a[i / divisor<u64>] * b[i]",
        "a[i] * b[i] + 0.5",
        "((i == 1u64)? | true => a[i] * b[i] | false => 0.0)",
    ] {
        let source = format!(
            "+> stats\na := left<[f64]:32,1>\nb := right<[f64]:32,1>\nstats/sum/column([{term} | i <- 1u64..=3u64])[1u64]\n"
        );
        for plan in plans(&source) {
            let layout = Layout::derive(&plan).unwrap();
            assert!(
                layout.regions.iter().any(|r| r.reduction.is_some()),
                "retained generic reduction: {term}"
            );
            assert!(
                layout
                    .regions
                    .iter()
                    .all(|r| Dot::derive(&plan, r, None).is_none()),
                "{term}"
            );
        }
    }
}

#[test]
fn ordered_map_dot_proofs_reject_child_captures_unavailable_at_parent_entry() {
    let source = "+> stats\nvalues := data<[f64]:4,6>\nweights := coefficients<[f64]:3,1>\n[stats/sum/column([weights[i] * values[i + shift<u64>,j] | i <- 1u64..=3u64])[1u64] | j <- 1u64..=6u64]\n";
    fn capture(index: &Index) -> Option<u32> {
        match &index.expr {
            Expr::Capture(id) => Some(*id),
            Expr::Binary { left, right, .. } => capture(left).or_else(|| capture(right)),
            _ => None,
        }
    }
    for plan in plans(source) {
        let layout = Layout::derive(&plan).unwrap();
        let parent = layout
            .regions
            .iter()
            .find(|r| Batch::derive(&plan, r).is_some())
            .unwrap();
        let batch = Batch::derive(&plan, parent).unwrap();
        let id = batch
            .dot
            .operands
            .iter()
            .flat_map(|o| &o.indices)
            .find_map(capture)
            .unwrap();
        let mut changed = parent.clone();
        changed.captures.retain(|&v| v != id);
        assert!(Batch::derive(&plan, &changed).is_none());
    }
}
