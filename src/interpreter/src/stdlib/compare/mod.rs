#[macro_use]
use crate::stdlib::*;

pub mod gt;
pub mod lt;
pub mod lte;
pub mod gte;
pub mod eq;
pub mod neq;

pub use self::gt::*;
pub use self::lt::*;
pub use self::lte::*;
pub use self::gte::*;
pub use self::eq::*;
pub use self::neq::*;

// ----------------------------------------------------------------------------
// Compare Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! impl_compare_binop {
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
    lhs: Ref<$arg1_type>,
    rhs: Ref<$arg2_type>,
    out: Ref<$out_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
    T: Debug + Clone + Sync + Send + 'static + 
    PartialEq + PartialOrd,
    Ref<$out_type>: ToValue
    {
    fn solve(&self) {
      let lhs_ptr = self.lhs.as_ptr();
      let rhs_ptr = self.rhs.as_ptr();
      let out_ptr = self.out.as_ptr();
      $op!(lhs_ptr,rhs_ptr,out_ptr);
    }
    fn out(&self) -> Value { self.out.to_value() }
    fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

#[macro_export]
macro_rules! impl_compare_fxns {
  ($lib:ident) => {
    impl_fxns!($lib,T,bool,impl_compare_binop);
  }
}

#[macro_export]
macro_rules! impl_compare_fxns_bool {
  ($lib:ident) => {
    impl_fxns!($lib,T,bool,impl_compare_binop);
  }
}