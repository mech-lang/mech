#![cfg_attr(not(test), no_main)]
#![allow(warnings)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    pub use crate::catalog::__mech_native::*;
}

use indexmap::set::IndexSet;

use mech_core::*;

use paste::paste;

use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::sync::LazyLock;

static PURE_SET_BINARY_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| set_full_write_contract(ChangeDetectionPolicy::AlwaysChanged));
static PURE_SET_MEMBERSHIP_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| set_full_write_contract(ChangeDetectionPolicy::ExactScalar));

fn set_full_write_contract(
    change_detection: ChangeDetectionPolicy,
) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
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
#[cfg(feature = "setdata")]
pub use self::setdata::*;

// ----------------------------------------------------------------------------
// Set Library
// ----------------------------------------------------------------------------

fn normalize_set_element(value: LegacyValue) -> LegacyValue {
    match value {
        LegacyValue::MutableReference(reference) => reference.borrow().clone(),
        value => value,
    }
}

#[macro_export]
macro_rules! impl_set_fxns {
    ($lib:ident) => {
        impl_fxns!($lib, T, T, impl_binop);
    };
}
