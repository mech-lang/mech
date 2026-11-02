#[cfg(feature = "project")]
use mech_core::MechSourceCode;
#[cfg(feature = "row_vectord")]
use mech_core::{CopyMat, Ref};
#[cfg(feature = "project")]
use mech_runtime::{InMemorySourceResolver, ResolvedSource};
#[cfg(feature = "row_vectord")]
use nalgebra::RowDVector;

#[cfg(feature = "row_vectord")]
#[test]
fn copy_mat_row_vector_preserves_the_v03_return_contract() {
    let source = Ref::new(RowDVector::from_vec(vec![1.0, 2.0, 3.0]));
    let destination = Ref::new(RowDVector::from_element(3, 0.0));

    let copied: usize = source.copy_into_r(&destination, 0);

    assert_eq!(copied, 3);
    assert_eq!(
        &*destination.borrow(),
        &RowDVector::from_vec(vec![1.0, 2.0, 3.0])
    );
}

#[cfg(feature = "project")]
#[test]
fn resolver_compat_builder_remains_infallible_and_has_a_fallible_peer() {
    let invalid = || {
        ResolvedSource::new(
            "",
            "memory:invalid",
            MechSourceCode::String("x := 1".to_string()),
        )
    };

    let resolver = InMemorySourceResolver::new().with_source("invalid", invalid());
    assert!(!resolver.contains("invalid"));
    assert!(
        InMemorySourceResolver::new()
            .try_with_source("invalid", invalid())
            .is_err()
    );
}
