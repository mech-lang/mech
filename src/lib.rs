#![no_main]
#![allow(warnings)]

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

use mech_core::*;

use paste::paste;

use std::fmt::{Display, Debug};
use std::marker::PhantomData;

#[cfg(feature = "union")]
pub mod union;
//#[cfg(feature = "complement")]
//pub mod complement;
#[cfg(feature = "difference")]
pub mod difference;
#[cfg(feature = "sym_difference")]
pub mod sym_difference;
#[cfg(feature = "intersection")]
pub mod intersection;
#[cfg(feature = "insert")]
pub mod insert;
#[cfg(feature = "remove")]
pub mod remove;
#[cfg(feature = "subset")]
pub mod subset;
#[cfg(feature = "superset")]
pub mod superset;
#[cfg(feature = "proper_subset")]
pub mod proper_subset;
#[cfg(feature = "proper_superset")]
pub mod proper_superset;
#[cfg(feature = "element_of")
pub mod element_of;
#[cfg(feature = "not_element_of")
pub mod not_element_of;]
#[cfg(feature = "disjoint")]
pub mod disjoint;
#[cfg(feature = "cartesianproduct")]
pub mod cartesianproduct;
#[cfg(feature = "powerset")]
pub mod powerset;
#[cfg(feature = "equals")]
pub mod equals;
#[cfg(feature = "size")]
pub mod size;

//
#[cfg(feature = "union")]
pub use self::union::*;
//#[cfg(feature = "complement")]
//pub use self::complement::*;
#[cfg(feature = "difference")]
pub use self::difference::*;
#[cfg(feature = "sym_difference")]
pub use self::sym_difference::*;
#[cfg(feature = "intersection")]
pub use self::intersection::*;
#[cfg(feature = "insert")]
pub use self::insert::*;
#[cfg(feature = "remove")]
pub use self::remove::*;
#[cfg(feature = "subset")]
pub use self::subset::*;
#[cfg(feature = "superset")]
pub use self::superset::*;
#[cfg(feature = "proper_subset")]
pub use self::proper_subset::*;
#[cfg(feature = "proper_superset")]
pub use self::proper_superset::*;
#[cfg(feature = "element_of")]
pub use self::element_of::*;
#[cfg(feature = "not_element_of")]
pub use self::not_element_of::*;
#[cfg(feature = "disjoint")]
pub use self::disjoint::*;
#[cfg(feature = "cartesianproduct")]
pub use self::cartesianproduct::*;
#[cfg(feature = "powerset")]
pub use self::powerset::*;
#[cfg(feature = "equals")]
pub use self::equals::*;
#[cfg(feature = "size")]
pub use self::size::*;

//#[cfg(feature = "join")]
//pub mod join;
//
//#[cfg(feature = "join")]
//pub use self::join::*;
//#[cfg(feature = "op_assign")]
//pub use self::op_assign::*;
//#[cfg(feature = "ops")]
//pub use self::ops::*;


// ----------------------------------------------------------------------------
// Set Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! register_set_fxns {
  ($lib:ident, $($suffix:ident),* $(,)?) => {
    paste::paste! {
      $(
        register_fxn_descriptor!([<$lib $suffix>],
          i8, "i8",
          i16, "i16",
          i32, "i32",
          i64, "i64",
          i128, "i128",
          u8, "u8",
          u16, "u16",
          u32, "u32",
          u64, "u64",
          u128, "u128",
          F32, "f32",
          F64, "f64",
          C64, "c64",
          R64, "r64" 
        );
      )*
    }
  };
}

#[macro_export]
macro_rules! impl_set_fxns {
  ($lib:ident) => {
    impl_fxns!($lib,T,T,impl_binop);
    register_set_fxns!($lib,
        SS, SM1, SM2, SM3, SM4, SM2x3, SM3x2, SMD, SR2, SR3, SR4, SRD,
        SV2, SV3, SV4, SVD, M1S, M2S, M3S, M4S, M2x3S, M3x2S, MDS,
        R2S, R3S, R4S, RDS, V2S, V3S, V4S, VDS, M1M1, M2M2, M3M3, M4M4,
        M2x3M2x3, M3x2M3x2, MDMD, M2V2, M3V3, M4V4, M2x3V2, M3x2V3, MDVD,
        MDV2, MDV3, MDV4, V2M2, V3M3, V4M4, V2M2x3, V3M3x2, VDMD, V2MD,
        V3MD, V4MD, M2R2, M3R3, M4R4, M2x3R3, M3x2R2, MDRD, MDR2, MDR3,
        MDR4, R2M2, R3M3, R4M4, R3M2x3, R2M3x2, RDMD, R2MD, R3MD, R4MD,
        R2R2, R3R3, R4R4, RDRD, V2V2, V3V3, V4V4, VDVD, Union
    );
  }}

