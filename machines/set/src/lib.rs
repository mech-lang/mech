#![cfg_attr(not(test), no_main)]
#![allow(warnings)]

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

#[macro_export]
macro_rules! impl_set_fxns {
    ($lib:ident) => {
        impl_fxns!($lib, T, T, impl_binop);
    };
}
