use std::collections::BTreeSet;

use mech_core::{
    FunctionCatalog, MResult, NativeFunctionLinkage, ParsedProgram, RuntimeFunctionEntry,
    RuntimeFunctionId, hash_str,
};

use crate::{
    error::{NativeBuildErrorKind, native_build_error},
    plan::PlannedRuntimeFunction,
};

/// Resolves the exact native linkage for every runtime instruction in a
/// validated bytecode program.
///
/// IDs are sorted and deduplicated before catalog lookup, so repeated calls to
/// one runtime factory result in one planned installer.
pub(crate) fn analyze_runtime_functions(
    program: &ParsedProgram,
    catalog: &FunctionCatalog,
) -> MResult<Vec<PlannedRuntimeFunction>> {
    analyze_runtime_function_ids(
        program
            .instructions
            .iter()
            .filter_map(|instruction| instruction.runtime_function()),
        catalog,
    )
}

pub(crate) fn analyze_runtime_function_ids(
    ids: impl IntoIterator<Item = u64>,
    catalog: &FunctionCatalog,
) -> MResult<Vec<PlannedRuntimeFunction>> {
    ids.into_iter()
        .map(RuntimeFunctionId::from_raw)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|id| analyze_runtime_function(id, catalog))
        .collect()
}

fn analyze_runtime_function(
    id: RuntimeFunctionId,
    catalog: &FunctionCatalog,
) -> MResult<PlannedRuntimeFunction> {
    let entry = catalog.runtime_entry(id).ok_or_else(|| {
        native_build_error(
            NativeBuildErrorKind::NativeRuntimeFunctionUnknown { id: id.raw() },
            None,
        )
    })?;
    analyze_runtime_entry(id, entry)
}

fn analyze_runtime_entry(
    required_id: RuntimeFunctionId,
    entry: &RuntimeFunctionEntry,
) -> MResult<PlannedRuntimeFunction> {
    let raw_name_id = hash_str(&entry.name);
    let name_id = RuntimeFunctionId::from_raw(raw_name_id);
    if name_id != required_id || entry.id != required_id {
        return Err(native_build_error(
            NativeBuildErrorKind::NativeRuntimeFunctionNameMismatch {
                id: required_id.raw(),
                name: entry.name.clone(),
            },
            None,
        ));
    }

    let linkage = entry.native_linkage.as_ref().ok_or_else(|| {
        native_build_error(
            NativeBuildErrorKind::NativeRuntimeFunctionLinkageMissing {
                id: required_id.raw(),
                name: entry.name.clone(),
            },
            None,
        )
    })?;
    validate_linkage(required_id, &entry.name, linkage)?;

    Ok(PlannedRuntimeFunction {
        runtime_id: required_id.raw(),
        runtime_name: entry.name.clone(),
        package: linkage.package.to_owned(),
        crate_name: linkage.crate_name.to_owned(),
        installer_path: linkage.installer_path.to_owned(),
        cargo_features: linkage
            .cargo_features
            .iter()
            .map(|feature| (*feature).to_owned())
            .collect(),
    })
}

fn validate_linkage(
    id: RuntimeFunctionId,
    name: &str,
    linkage: &NativeFunctionLinkage,
) -> MResult<()> {
    let invalid = |reason: String| {
        native_build_error(
            NativeBuildErrorKind::NativeRuntimeFunctionLinkageInvalid {
                id: id.raw(),
                name: name.to_owned(),
                reason,
            },
            None,
        )
    };

    if !is_cargo_package_name(linkage.package) {
        return Err(invalid(format!(
            "package {:?} must match [a-z][a-z0-9-]*",
            linkage.package
        )));
    }
    if !is_rust_identifier(linkage.crate_name) {
        return Err(invalid(format!(
            "crate name {:?} must match [A-Za-z_][A-Za-z0-9_]*",
            linkage.crate_name
        )));
    }
    if !is_installer_path(linkage.installer_path) {
        return Err(invalid(format!(
            "installer path {:?} must contain at least two `::`-separated Rust identifiers",
            linkage.installer_path
        )));
    }
    if linkage.cargo_features.is_empty() {
        return Err(invalid("Cargo features must not be empty".to_owned()));
    }
    for feature in linkage.cargo_features {
        if !is_cargo_feature_name(feature) {
            return Err(invalid(format!(
                "Cargo feature {feature:?} must match [A-Za-z_][A-Za-z0-9_-]*"
            )));
        }
    }
    for pair in linkage.cargo_features.windows(2) {
        if pair[0] == pair[1] {
            return Err(invalid(format!(
                "Cargo feature {:?} is duplicated",
                pair[0]
            )));
        }
        if pair[0] > pair[1] {
            return Err(invalid("Cargo features must be sorted".to_owned()));
        }
    }
    Ok(())
}

fn is_cargo_package_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn is_rust_identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_installer_path(path: &str) -> bool {
    let components = path.split("::").collect::<Vec<_>>();
    components.len() >= 2
        && components
            .iter()
            .all(|component| is_rust_identifier(component))
}

fn is_cargo_feature_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

#[cfg(test)]
mod tests {
    use mech_core::{FunctionArgs, FunctionCatalogBuilder, MechFunction, NativeFunctionLinkage};

    use super::*;

    fn unused_factory(_arguments: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        panic!("factory must not run during native planning")
    }

    #[test]
    fn runtime_ids_are_sorted_and_deduplicated() {
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_runtime_factory_with_linkage(
                "B",
                unused_factory,
                NativeFunctionLinkage {
                    package: "mech-b",
                    crate_name: "mech_b",
                    installer_path: "mech_b::__mech_native::install_b",
                    cargo_features: &["native-link", "runtime"],
                },
            )
            .unwrap();
        builder
            .insert_runtime_factory_with_linkage(
                "A",
                unused_factory,
                NativeFunctionLinkage {
                    package: "mech-a",
                    crate_name: "mech_a",
                    installer_path: "mech_a::__mech_native::install_a",
                    cargo_features: &["native-link", "runtime"],
                },
            )
            .unwrap();
        let catalog = builder.build().unwrap();
        let a = hash_str("A");
        let b = hash_str("B");

        let planned = analyze_runtime_function_ids([b, a, b], &catalog).unwrap();
        assert_eq!(planned.len(), 2);
        assert!(
            planned
                .windows(2)
                .all(|pair| pair[0].runtime_id < pair[1].runtime_id)
        );
    }

    #[test]
    fn unknown_runtime_function_fails_before_generation() {
        let error = analyze_runtime_function_ids([0x1234], &FunctionCatalog::empty()).unwrap_err();
        assert_eq!(error.kind_name(), "NativeRuntimeFunctionUnknown");
    }

    #[test]
    fn known_runtime_function_without_linkage_is_rejected() {
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_runtime_factory("KnownButUnlinked", unused_factory)
            .unwrap();
        let catalog = builder.build().unwrap();

        let error =
            analyze_runtime_function_ids([hash_str("KnownButUnlinked")], &catalog).unwrap_err();
        assert_eq!(error.kind_name(), "NativeRuntimeFunctionLinkageMissing");
    }
}
