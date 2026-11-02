#[cfg(any(feature = "math_mul_assign", feature = "math_div_assign"))]
use mech_engine::CanonicalSourceFrontend;
#[cfg(any(feature = "math_mul_assign", feature = "math_div_assign"))]
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

#[cfg(any(feature = "math_mul_assign", feature = "math_div_assign"))]
fn operations(source: &str) -> Vec<String> {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0xA551), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "{source:?}: {:?}",
        parsed.diagnostics
    );
    let document = DocumentSyntax::cast(parsed.syntax()).expect("canonical document");
    CanonicalSourceFrontend
        .compile_document(&document)
        .unwrap()
        .program()
        .nodes
        .iter()
        .filter_map(|node| node.operation())
        .map(|operation| {
            operation
                .module_path
                .iter()
                .chain(std::iter::once(&operation.operation_name))
                .cloned()
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect()
}

#[cfg(feature = "math_mul_assign")]
#[test]
fn whole_mul_assignment_selects_mul_operation_only() {
    let operations = operations("~x := 6.0\ny := 3.0\nx *= y\nx");
    assert!(
        operations.iter().any(|name| name == "math/mul"),
        "{operations:?}"
    );
    assert!(
        !operations.iter().any(|name| name == "math/div"),
        "{operations:?}"
    );
}

#[cfg(feature = "math_div_assign")]
#[test]
fn whole_div_assignment_selects_div_operation_only() {
    let operations = operations("~x := 6.0\ny := 3.0\nx /= y\nx");
    assert!(
        operations.iter().any(|name| name == "math/div"),
        "{operations:?}"
    );
    assert!(
        !operations.iter().any(|name| name == "math/mul"),
        "{operations:?}"
    );
}
