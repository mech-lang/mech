#![cfg(feature = "native-link")]

use mech_core::{FunctionCatalogBuilder, RuntimeFunctionId};

#[cfg(all(feature = "f64", feature = "variable_define"))]
#[test]
fn variable_define_f64_installer_preserves_id_and_rejects_duplicates() {
    const EXPECTED_ID: u64 = 0x0023_b6ad_86b6_55e7;

    assert_eq!(
        RuntimeFunctionId::from_name("VariableDefineF64").raw(),
        EXPECTED_ID
    );

    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::__mech_native::install_variable_define_f64(&mut builder).unwrap();
    let duplicate = mech_engine::__mech_native::install_variable_define_f64(&mut builder)
        .expect_err("an exact installer must reject duplicate insertion");
    assert_eq!(
        duplicate.kind_name(),
        "FunctionCatalogDuplicateRuntimeFactory"
    );

    let catalog = builder.build().unwrap();
    assert_eq!(catalog.runtime_factory_count(), 1);
    let entry = catalog
        .runtime_entry(RuntimeFunctionId::from_raw(EXPECTED_ID))
        .expect("the exact installer must insert its one factory");
    assert_eq!(entry.name, "VariableDefineF64");
}

#[cfg(all(feature = "f64", feature = "matrix_horzcat", feature = "row_vectord"))]
#[test]
fn variadic_horzcat_f64_installer_preserves_id_and_rejects_duplicates() {
    const EXPECTED_ID: u64 = 0x006c_13ae_b8d2_1f6c;

    assert_eq!(
        RuntimeFunctionId::from_name("HorizontalConcatenateRDN<f64>").raw(),
        EXPECTED_ID
    );

    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::__mech_native::install_horizontal_concatenate_rdn_f64(&mut builder).unwrap();
    let duplicate =
        mech_engine::__mech_native::install_horizontal_concatenate_rdn_f64(&mut builder)
            .expect_err("an exact installer must reject duplicate insertion");
    assert_eq!(
        duplicate.kind_name(),
        "FunctionCatalogDuplicateRuntimeFactory"
    );

    let catalog = builder.build().unwrap();
    assert_eq!(catalog.runtime_factory_count(), 1);
    let entry = catalog
        .runtime_entry(RuntimeFunctionId::from_raw(EXPECTED_ID))
        .expect("the exact installer must insert its one factory");
    assert_eq!(entry.name, "HorizontalConcatenateRDN<f64>");
}

#[cfg(all(
    feature = "f64",
    feature = "matrix_horzcat",
    feature = "matrix_vertcat",
    feature = "matrixd",
    feature = "row_vectord"
))]
#[test]
fn normal_aggregate_contains_the_compiler_emitted_dynamic_constructors() {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    let catalog = builder.build().unwrap();

    for name in [
        "HorizontalConcatenateRDN<f64>",
        "VerticalConcatenateNArgs<f64>",
    ] {
        let entry = catalog
            .runtime_entry(RuntimeFunctionId::from_name(name))
            .unwrap_or_else(|| {
                panic!("ordinary runtime registration must contain compiler-emitted {name}")
            });
        assert_eq!(entry.name, name);
    }
}

#[cfg(all(
    feature = "native-plan",
    feature = "bool",
    feature = "f64",
    feature = "string",
    feature = "variable_define",
    feature = "matrix2",
    feature = "matrix_horzcat",
    feature = "matrix_vertcat",
    feature = "matrixd",
    feature = "row_vector2",
    feature = "row_vectord",
    feature = "vector2",
    feature = "vectord"
))]
#[test]
fn native_plan_aggregate_carries_exact_representative_linkage() {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    let catalog = builder.build().unwrap();

    let linked = catalog
        .runtime_entries()
        .filter_map(|entry| {
            entry
                .native_linkage
                .as_ref()
                .map(|linkage| (entry.name.as_str(), linkage))
        })
        .collect::<Vec<_>>();

    assert_eq!(linked.len(), 5);

    let variable = linked
        .iter()
        .find(|(name, _)| *name == "VariableDefineF64")
        .expect("VariableDefineF64 must carry native linkage")
        .1;
    assert_eq!(variable.package, "mech-engine");
    assert_eq!(variable.crate_name, "mech_engine");
    assert_eq!(
        variable.installer_path,
        "mech_engine::__mech_native::install_variable_define_f64"
    );
    assert_eq!(
        variable.cargo_features,
        [
            "bool",
            "f64",
            "native-link",
            "runtime",
            "string",
            "variable_define",
        ]
    );

    let variadic = linked
        .iter()
        .find(|(name, _)| *name == "HorizontalConcatenateRDN<f64>")
        .expect("HorizontalConcatenateRDN<f64> must carry native linkage")
        .1;
    assert_eq!(variadic.package, "mech-engine");
    assert_eq!(variadic.crate_name, "mech_engine");
    assert_eq!(
        variadic.installer_path,
        "mech_engine::__mech_native::install_horizontal_concatenate_rdn_f64"
    );
    assert_eq!(
        variadic.cargo_features,
        [
            "bool",
            "f64",
            "matrix_horzcat",
            "matrixd",
            "native-link",
            "row_vectord",
            "runtime",
            "vectord",
        ]
    );

    assert_ne!(variable.installer_path, variadic.installer_path);

    let horizontal_s2 = linked
        .iter()
        .find(|(name, _)| *name == "HorizontalConcatenateS2<f64>")
        .expect("fixed horizontal constructor must carry native linkage")
        .1;
    assert_eq!(
        horizontal_s2.installer_path,
        "mech_engine::__mech_native::install_horizontal_concatenate_s2_f64"
    );
    assert_eq!(
        horizontal_s2.cargo_features,
        [
            "bool",
            "f64",
            "matrix_horzcat",
            "native-link",
            "row_vector2",
            "runtime",
            "vector2",
        ]
    );

    let vertical_n_args = linked
        .iter()
        .find(|(name, _)| *name == "VerticalConcatenateNArgs<f64>")
        .expect("dynamic vertical constructor must carry native linkage")
        .1;
    assert_eq!(
        vertical_n_args.installer_path,
        "mech_engine::__mech_native::install_vertical_concatenate_n_args_f64"
    );
    assert_eq!(
        vertical_n_args.cargo_features,
        [
            "bool",
            "f64",
            "matrix_vertcat",
            "matrixd",
            "native-link",
            "row_vectord",
            "runtime",
            "vectord",
        ]
    );

    let vertical_r2_r2 = linked
        .iter()
        .find(|(name, _)| *name == "VerticalConcatenateR2R2<f64Matrix2RowVector2RowVector2>")
        .expect("fixed vertical constructor must carry native linkage")
        .1;
    assert_eq!(
        vertical_r2_r2.installer_path,
        "mech_engine::__mech_native::install_vertical_concatenate_r2_r2_f64"
    );
    assert_eq!(
        vertical_r2_r2.cargo_features,
        [
            "bool",
            "f64",
            "matrix2",
            "matrix_vertcat",
            "native-link",
            "row_vector2",
            "runtime",
            "vector2",
        ]
    );
}

#[cfg(all(
    feature = "f64",
    feature = "variable_define",
    feature = "matrix2",
    feature = "matrix_horzcat",
    feature = "matrix_vertcat",
    feature = "matrixd",
    feature = "row_vector2",
    feature = "row_vectord",
    feature = "vector2",
    feature = "vectord"
))]
#[test]
fn generated_catalog_code_can_call_every_engine_phase1_installer() {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::__mech_native::install_variable_define_f64(&mut builder).unwrap();
    mech_engine::__mech_native::install_horizontal_concatenate_rdn_f64(&mut builder).unwrap();
    mech_engine::__mech_native::install_horizontal_concatenate_s2_f64(&mut builder).unwrap();
    mech_engine::__mech_native::install_vertical_concatenate_n_args_f64(&mut builder).unwrap();
    mech_engine::__mech_native::install_vertical_concatenate_r2_r2_f64(&mut builder).unwrap();

    let catalog = builder.build().unwrap();
    assert_eq!(catalog.runtime_factory_count(), 5);
}
