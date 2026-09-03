#![cfg(feature = "functions")]

use std::sync::Arc;

use mech_core::{
    CanonicalFunctionSpecializer, FunctionCatalogBuilder, FunctionTypeDeclaration,
    FunctionTypeOverload, InputKindScheme, KindExpr, KindScheme, MResult, SourceInputKind,
    SourceTypeAuthority, SpecializationContext, SpecializationInvocation, SpecializedFunction,
    maintained_source_type_declaration,
};

struct NeverSpecialize;

impl CanonicalFunctionSpecializer for NeverSpecialize {
    fn specialize_invocation(
        &self,
        _: &SpecializationInvocation,
        _: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        unreachable!("catalog declaration tests never execute physical specializers")
    }
}

fn nullary_scheme() -> KindScheme {
    KindScheme::new(
        Box::new([]),
        Box::new([]),
        InputKindScheme::Fixed(Box::new([])),
        vec![KindExpr::Index].into_boxed_slice(),
        Box::new([]),
    )
    .unwrap()
}

#[test]
fn named_specializers_are_scheme_authoritative() {
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_canonical_specializer(
            "math/add",
            maintained_source_type_declaration("math/add").unwrap(),
            Arc::new(NeverSpecialize),
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    let entry = catalog.all_specializers().next().unwrap();
    let SourceTypeAuthority::Schemes(declaration) = &entry.type_authority else {
        panic!("a named operation cannot be syntax-directed")
    };
    assert!(!declaration.overloads.is_empty());
}

#[test]
fn parser_intrinsics_are_explicit_and_never_named_exports() {
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_canonical_intrinsic_specializer("assign", Arc::new(NeverSpecialize))
        .unwrap();
    let catalog = builder.build().unwrap();
    let entry = catalog.intrinsic_specializer_entries().next().unwrap();
    assert_eq!(
        entry.type_authority,
        SourceTypeAuthority::SyntaxDirectedIntrinsic
    );
    assert!(catalog.all_specializers().next().is_none());
    assert!(catalog.all_exports().next().is_none());
}

#[test]
fn malformed_and_duplicate_overloads_are_rejected() {
    let malformed = FunctionTypeDeclaration {
        overloads: vec![FunctionTypeOverload {
            id: 1,
            input_layout: vec![SourceInputKind::Value].into_boxed_slice(),
            scheme: nullary_scheme(),
        }]
        .into_boxed_slice(),
    };
    let mut builder = FunctionCatalogBuilder::new();
    let error = builder
        .insert_canonical_specializer("test/malformed", malformed, Arc::new(NeverSpecialize))
        .unwrap_err();
    assert_eq!(error.kind_name(), "FunctionCatalogInvalidTypeDeclaration");

    let overload = FunctionTypeOverload {
        id: 7,
        input_layout: Box::new([]),
        scheme: nullary_scheme(),
    };
    let duplicate = FunctionTypeDeclaration {
        overloads: vec![overload.clone(), overload].into_boxed_slice(),
    };
    let error = builder
        .insert_canonical_specializer("test/duplicate", duplicate, Arc::new(NeverSpecialize))
        .unwrap_err();
    assert_eq!(error.kind_name(), "FunctionCatalogInvalidTypeDeclaration");
}

#[test]
fn maintained_declarations_are_deterministic_and_ids_are_unique() {
    const OPERATIONS: &[&str] = &[
        "math/add",
        "math/mod",
        "math/neg",
        "compare/seq",
        "compare/eq",
        "compare/gt",
        "logic/not",
        "logic/and",
        "range/inclusive",
        "range/inclusive-increment",
        "matrix/transpose",
        "matrix/matmul",
        "matrix/dot",
        "matrix/solve",
        "matrix/horzcat",
        "matrix/vertcat",
        "set/element-of",
        "set/insert",
        "set/remove",
        "set/union",
        "set/intersection",
        "set/difference",
        "set/symmetric-difference",
        "set/cartesian-product",
        "set/powerset",
        "set/subset",
        "set/size",
        "string/concat",
        "stats/sum/column",
        "stats/sum/row",
        "combinatorics/n-choose-k",
    ];
    for operation in OPERATIONS {
        let first = maintained_source_type_declaration(operation).unwrap();
        let second = maintained_source_type_declaration(operation).unwrap();
        assert_eq!(first, second, "{operation}");
        let mut ids = first
            .overloads
            .iter()
            .map(|overload| overload.id)
            .collect::<Vec<_>>();
        let original = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), original, "{operation}");
    }
}
