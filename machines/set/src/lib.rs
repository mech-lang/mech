#![cfg_attr(not(test), no_main)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    pub use crate::catalog::__mech_native::*;
}

#[cfg(any(
    feature = "cartesian_product",
    feature = "difference",
    feature = "disjoint",
    feature = "element_of",
    feature = "equals",
    feature = "insert",
    feature = "intersection",
    feature = "not_element_of",
    feature = "not_equals",
    feature = "powerset",
    feature = "proper_subset",
    feature = "proper_superset",
    feature = "remove",
    all(feature = "size", feature = "u64"),
    feature = "subset",
    feature = "superset",
    feature = "symmetric_difference",
    feature = "union",
))]
use mech_core::*;

#[cfg(any(
    feature = "membership",
    feature = "modify",
    feature = "operations",
    feature = "relations",
    feature = "setdata"
))]
mod canonical;
#[cfg(any(
    feature = "cartesian_product",
    feature = "difference",
    feature = "disjoint",
    feature = "element_of",
    feature = "equals",
    feature = "insert",
    feature = "intersection",
    feature = "not_element_of",
    feature = "not_equals",
    feature = "powerset",
    feature = "proper_subset",
    feature = "proper_superset",
    feature = "remove",
    all(feature = "size", feature = "u64"),
    feature = "subset",
    feature = "superset",
    feature = "symmetric_difference",
    feature = "union",
))]
use std::sync::LazyLock;

#[cfg(any(
    feature = "cartesian_product",
    feature = "difference",
    feature = "intersection",
    feature = "symmetric_difference",
    feature = "union"
))]
static PURE_SET_BINARY_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_full_write_contract(2, ChangeDetectionPolicy::AlwaysChanged));
#[cfg(feature = "powerset")]
static PURE_SET_UNARY_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_full_write_contract(1, ChangeDetectionPolicy::AlwaysChanged));
#[cfg(any(
    feature = "disjoint",
    feature = "element_of",
    feature = "equals",
    feature = "not_element_of",
    feature = "not_equals",
    feature = "proper_subset",
    feature = "proper_superset",
    feature = "subset",
    feature = "superset"
))]
static PURE_SET_PREDICATE_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_full_write_contract(2, ChangeDetectionPolicy::ExactScalar));
#[cfg(any(feature = "insert", feature = "remove"))]
static PURE_SET_UPDATE_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_full_write_contract(2, ChangeDetectionPolicy::KernelReported));
#[cfg(all(feature = "size", feature = "u64"))]
static PURE_SET_SIZE_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_full_write_contract(1, ChangeDetectionPolicy::ExactScalar));

#[cfg(any(
    feature = "cartesian_product",
    feature = "difference",
    feature = "disjoint",
    feature = "element_of",
    feature = "equals",
    feature = "insert",
    feature = "intersection",
    feature = "not_element_of",
    feature = "not_equals",
    feature = "powerset",
    feature = "proper_subset",
    feature = "proper_superset",
    feature = "remove",
    all(feature = "size", feature = "u64"),
    feature = "subset",
    feature = "superset",
    feature = "symmetric_difference",
    feature = "union",
))]
fn pure_full_write_contract(
    input_count: usize,
    change_detection: ChangeDetectionPolicy,
) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            (0..input_count)
                .map(|_| InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

#[cfg(feature = "runtime")]
pub mod catalog;
#[cfg(feature = "runtime")]
pub use self::catalog::*;

#[cfg(feature = "membership")]
pub mod membership;
#[cfg(feature = "modify")]
pub mod modify;
#[cfg(feature = "operations")]
pub mod operations;
#[cfg(feature = "relations")]
pub mod relations;
#[cfg(feature = "setdata")]
pub mod setdata;

#[cfg(feature = "membership")]
pub use self::membership::*;
#[cfg(feature = "modify")]
pub use self::modify::*;
#[cfg(feature = "operations")]
pub use self::operations::*;
#[cfg(feature = "relations")]
pub use self::relations::*;
#[cfg(all(feature = "setdata", feature = "size", feature = "u64"))]
pub use self::setdata::*;

#[cfg(test)]
mod port_tests;

#[macro_export]
macro_rules! impl_set_fxns {
    ($lib:ident) => {
        impl_fxns!($lib, T, T, impl_binop);
    };
}
