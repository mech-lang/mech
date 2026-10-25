//! Independent contract for declaration versus canonical installation surfaces.
#![cfg(feature = "functions")]

#[test]
fn canonical_installers_keep_order_and_exclude_only_dynamic_cross_orientation() {
    macro_rules! declaration {
        ($rows:ident, $lib:ident, $suffix:ident, [$($feature:literal),*], $scalar:ty, $name:literal, $token:ident) => {
            $rows.push((stringify!($suffix), vec![$($feature),*]));
        };
    }
    macro_rules! canonical {
        ($rows:ident, $lib:ident, $suffix:ident, $scalar:ty, $name:literal, $token:ident) => {
            $rows.push(stringify!($suffix));
        };
    }
    let mut declarations: Vec<(&str, Vec<&str>)> = Vec::new();
    mech_core::__mech_for_each_exact_binop_runtime_factory_for_type!(
        declaration,
        declarations,
        Probe,
        f64,
        "f64",
        F64
    );
    let mut installers = Vec::new();
    mech_core::for_each_canonical_binop_factory!(canonical, installers, Probe, f64, "f64", F64);
    assert_eq!(
        installers,
        declarations
            .iter()
            .map(|row| row.0)
            .filter(|suffix| !matches!(*suffix, "VDRD" | "RDVD"))
            .collect::<Vec<_>>()
    );
    assert_eq!(installers[0], "SS");
    assert_eq!(declarations[0], ("SS", vec![]));
    assert_eq!(
        installers.len(),
        installers
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    #[cfg(all(feature = "vectord", feature = "row_vectord"))]
    {
        let index = declarations.iter().position(|row| row.0 == "VDRD").unwrap();
        assert_eq!(
            &declarations[index - 1..=index + 1],
            &[
                ("VDVD", vec!["vectord"]),
                ("VDRD", vec!["vectord", "row_vectord"]),
                ("RDVD", vec!["vectord", "row_vectord"]),
            ]
        );
        assert!(!installers.contains(&"VDRD") && !installers.contains(&"RDVD"));
    }
    #[cfg(all(
        feature = "matrix1",
        feature = "matrix2",
        feature = "matrix3",
        feature = "matrix4",
        feature = "matrix2x3",
        feature = "matrix3x2",
        feature = "matrixd",
        feature = "row_vector2",
        feature = "row_vector3",
        feature = "row_vector4",
        feature = "row_vectord",
        feature = "vector2",
        feature = "vector3",
        feature = "vector4",
        feature = "vectord"
    ))]
    {
        assert_eq!(declarations.len(), 84);
        assert_eq!(installers.len(), 82);
    }
    #[cfg(all(feature = "matrixd", feature = "vector2"))]
    assert_eq!(
        declarations.iter().find(|row| row.0 == "MDV2").unwrap().1,
        ["matrixd", "vector2"]
    );
}
