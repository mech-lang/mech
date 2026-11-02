use mech_core::*;

// These are the established public family spellings, independent of the new
// generic factory type names and the identity construction table.
fn families() -> &'static [(&'static str, &'static str)] {
    &[
        #[cfg(feature = "atan2")]
        ("Atan2", "math/atan2"),
        #[cfg(feature = "copysign")]
        ("Copysign", "math/copysign"),
        #[cfg(feature = "fdim")]
        ("Fdim", "math/fdim"),
        #[cfg(feature = "fmod")]
        ("Fmod", "math/fmod"),
        #[cfg(feature = "nextafter")]
        ("Nextafter", "math/nextafter"),
        #[cfg(feature = "remainder")]
        ("Remainder", "math/remainder"),
        #[cfg(feature = "jn")]
        ("Jn", "math/bessel/jn"),
        #[cfg(feature = "yn")]
        ("Yn", "math/bessel/yn"),
    ]
}

fn catalog() -> FunctionCatalog {
    let mut builder = FunctionCatalogBuilder::new();
    crate::catalog::install_runtime(&mut builder).unwrap();
    builder.build().unwrap()
}

#[cfg(all(feature = "native-link", feature = "f64"))]
#[test]
fn floating_binary_installers_are_exported_at_the_declared_crate_path() {
    type Installer = fn(&mut FunctionCatalogBuilder) -> MResult<()>;
    let installers: &[(&str, Installer)] = &[
        #[cfg(feature = "atan2")]
        ("Atan2F64", crate::__mech_native::install_atan2_f64),
        #[cfg(feature = "copysign")]
        ("CopysignF64", crate::__mech_native::install_copysign_s_f64),
        #[cfg(feature = "fdim")]
        ("FdimF64", crate::__mech_native::install_fdim_s_f64),
        #[cfg(feature = "fmod")]
        ("FmodF64", crate::__mech_native::install_fmod_s_f64),
        #[cfg(feature = "nextafter")]
        (
            "NextafterF64",
            crate::__mech_native::install_nextafter_s_f64,
        ),
        #[cfg(feature = "remainder")]
        (
            "RemainderF64",
            crate::__mech_native::install_remainder_s_f64,
        ),
        #[cfg(feature = "jn")]
        ("JnF64", crate::__mech_native::install_jn_s_f64),
        #[cfg(feature = "yn")]
        ("YnF64", crate::__mech_native::install_yn_s_f64),
    ];
    assert!(!installers.is_empty());
    for &(name, install) in installers {
        let mut builder = FunctionCatalogBuilder::new();
        install(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        assert_eq!(catalog.runtime_factory_count(), 1);
        assert_eq!(
            catalog
                .runtime_entry(RuntimeFunctionId::from_name(name))
                .unwrap()
                .name,
            name,
        );
    }
}

#[test]
fn floating_binary_existing_signatures_keep_runtime_ids_and_native_installers() {
    let catalog = catalog();
    let scalars = [
        #[cfg(feature = "f32")]
        (
            "f32",
            FunctionValueRepresentation::F32,
            FunctionMatrixElement::F32,
        ),
        #[cfg(feature = "f64")]
        (
            "f64",
            FunctionValueRepresentation::F64,
            FunctionMatrixElement::F64,
        ),
    ];
    // The pre-refactor signature inventory: each enabled same-shape factory
    // keeps its original identifier and installer, including mixed-case M2x3.
    let shapes = [
        ("", None),
        #[cfg(feature = "matrix1")]
        ("M1", Some(FunctionMatrixRepresentation::Matrix1)),
        #[cfg(feature = "matrix2")]
        ("M2", Some(FunctionMatrixRepresentation::Matrix2)),
        #[cfg(feature = "matrix3")]
        ("M3", Some(FunctionMatrixRepresentation::Matrix3)),
        #[cfg(feature = "matrix4")]
        ("M4", Some(FunctionMatrixRepresentation::Matrix4)),
        #[cfg(feature = "matrix2x3")]
        ("M2x3", Some(FunctionMatrixRepresentation::Matrix2x3)),
        #[cfg(feature = "matrix3x2")]
        ("M3x2", Some(FunctionMatrixRepresentation::Matrix3x2)),
        #[cfg(feature = "matrixd")]
        ("MD", Some(FunctionMatrixRepresentation::MatrixD)),
        #[cfg(feature = "row_vector2")]
        ("R2", Some(FunctionMatrixRepresentation::RowVector2)),
        #[cfg(feature = "row_vector3")]
        ("R3", Some(FunctionMatrixRepresentation::RowVector3)),
        #[cfg(feature = "row_vector4")]
        ("R4", Some(FunctionMatrixRepresentation::RowVector4)),
        #[cfg(feature = "row_vectord")]
        ("RD", Some(FunctionMatrixRepresentation::RowVectorD)),
        #[cfg(feature = "vector2")]
        ("V2", Some(FunctionMatrixRepresentation::Vector2)),
        #[cfg(feature = "vector3")]
        ("V3", Some(FunctionMatrixRepresentation::Vector3)),
        #[cfg(feature = "vector4")]
        ("V4", Some(FunctionMatrixRepresentation::Vector4)),
        #[cfg(feature = "vectord")]
        ("VD", Some(FunctionMatrixRepresentation::VectorD)),
    ];
    let mut checked = 0;
    for &(family, operation) in families() {
        for (scalar, scalar_representation, element) in scalars {
            for (shape, storage) in shapes {
                let name = format!("{family}{shape}{}", scalar.to_uppercase());
                let entry = catalog
                    .runtime_entry(RuntimeFunctionId::from_name(&name))
                    .unwrap_or_else(|| panic!("removed existing runtime identity {name}"));
                assert_eq!(entry.name, name);
                let representation = storage.map_or(scalar_representation, |storage| {
                    FunctionValueRepresentation::Matrix {
                        element,
                        storage: FunctionMatrixStoragePattern::Exact(storage),
                    }
                });
                let signature = RuntimeFunctionSignature::binary(
                    representation,
                    representation,
                    representation,
                );
                assert_eq!(entry.signature(), signature, "{name}");
                let installer_shape = if family == "Atan2" && matches!(shape, "MD" | "RD" | "VD") {
                    format!("_{}_d", shape[..1].to_lowercase())
                } else if shape.is_empty() {
                    if family == "Atan2" {
                        "".into()
                    } else {
                        "_s".into()
                    }
                } else {
                    format!("_{}", shape.to_lowercase())
                };
                let installer = format!(
                    "mech_math::__mech_native::install_{}{installer_shape}_{scalar}",
                    family.to_lowercase(),
                );
                assert_eq!(
                    entry.native_linkage.as_ref().unwrap().installer_path,
                    installer
                );
                let operation = OperationId::from_name(operation);
                assert_eq!(
                    catalog
                        .runtime_entries()
                        .filter(|candidate| {
                            candidate.operation_binding().permits(operation)
                                && candidate.signature() == signature
                        })
                        .count(),
                    1,
                    "{name} must have one canonical registration, not an alias pair",
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, families().len() * scalars.len() * shapes.len());
    assert!(checked > 0);
}

#[cfg(all(feature = "compiler", feature = "f64"))]
#[test]
fn floating_binary_unbound_compilers_emit_the_preserved_runtime_ids() {
    let catalog = catalog();
    for &(family, operation) in families() {
        let cases = [
            ("", ValueCell::from_exact(1.0_f64).unwrap()),
            #[cfg(feature = "matrixd")]
            (
                "MD",
                ValueCell::from_exact(nalgebra::DMatrix::from_element(2, 3, 1.0_f64)).unwrap(),
            ),
            #[cfg(feature = "matrix2x3")]
            (
                "M2x3",
                ValueCell::from_exact(nalgebra::Matrix2x3::from_element(1.0_f64)).unwrap(),
            ),
        ];
        for (shape, input) in cases {
            let name = format!("{family}{shape}F64");
            let entry = catalog
                .runtime_entry(RuntimeFunctionId::from_name(&name))
                .unwrap();
            // A distinct owning output with the exact physical representation
            // avoids any source-plan binding that could conceal a compiler's
            // independently reconstructed runtime name.
            let output = match shape {
                "" => ValueCell::from_exact(0.0_f64).unwrap(),
                #[cfg(feature = "matrixd")]
                "MD" => ValueCell::from_exact(nalgebra::DMatrix::<f64>::zeros(2, 3)).unwrap(),
                #[cfg(feature = "matrix2x3")]
                "M2x3" => ValueCell::from_exact(nalgebra::Matrix2x3::<f64>::zeros()).unwrap(),
                _ => unreachable!(),
            };
            let (function, _) = entry
                .bind_resolved_invocation(
                    OperationId::from_name(operation),
                    ExecutionTarget::DirectRuntime,
                    FunctionInvocation::binary(output, input.clone(), input),
                )
                .unwrap();
            let mut context = CompileCtx::new();
            let output = function.compile(&mut context).unwrap();
            let program = ParsedProgram::from_bytes(&context.finish(output).unwrap()).unwrap();
            let emitted = program
                .instructions
                .iter()
                .filter_map(|instruction| instruction.runtime_function())
                .collect::<Vec<_>>();
            assert_eq!(
                emitted,
                vec![RuntimeFunctionId::from_name(&name).raw()],
                "{name}"
            );
            program.validate_runtime_contracts(&catalog).unwrap();
        }
    }
}
