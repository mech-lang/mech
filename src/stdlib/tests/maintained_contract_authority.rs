#![cfg(feature = "full_source")]

use mech_core::{FunctionValueRepresentation, OperationId, maintained_operation_contract};

#[test]
fn concrete_provider_contracts_equal_the_portable_source_authority() {
    let catalog = mech_stdlib::source_catalog();
    for (name, arity) in [
        ("math/add", 2),
        ("math/atan2", 2),
        ("math/neg", 1),
        ("math/sin", 1),
        ("compare/eq", 2),
        ("logic/not", 1),
        ("logic/and", 2),
        ("string/concat", 2),
        ("range/inclusive", 2),
        ("range/exclusive", 2),
        ("range/inclusive-increment", 3),
        ("range/exclusive-increment", 3),
        ("matrix/transpose", 1),
    ] {
        let operation = OperationId::from_name(name);
        assert!(
            catalog.specializer(operation).is_some(),
            "{name} needs an advertised source specializer"
        );
        let mut checked = 0;
        for entry in catalog.runtime_entries() {
            let Some(actual) = entry.operation_contract(operation) else {
                continue;
            };
            let matrix = matches!(
                entry.signature().output,
                FunctionValueRepresentation::Matrix { .. }
            );
            assert_eq!(
                actual,
                &maintained_operation_contract(name, arity, matrix).unwrap(),
                "{name} {:?}",
                entry.signature()
            );
            checked += 1;
        }
        assert!(
            checked > 0,
            "{name} must have an advertised provider to qualify this parity witness"
        );
    }
}

#[test]
fn every_advertised_mathematical_operation_has_maintained_type_and_contract_metadata() {
    let catalog = mech_stdlib::source_catalog();
    let names = catalog
        .all_exports()
        .map(|export| export.canonical_name.as_str())
        .filter(|name| name.starts_with("math/") && !name.contains("-assign"))
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        names.len() > 40,
        "the full mathematical provider surface must be exercised"
    );
    for name in names {
        let operation = mech_core::maintained_math_operation(name).unwrap_or_else(|| {
            panic!("advertised {name} is absent from maintained operation metadata")
        });
        assert!(
            mech_core::maintained_source_type_declaration(name).is_ok(),
            "{name}"
        );
        for matrix in [false, true] {
            assert!(
                maintained_operation_contract(name, operation.input_count(), matrix).is_some(),
                "{name}"
            );
        }
    }
}
