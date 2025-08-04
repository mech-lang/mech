extern crate nalgebra as na;
extern crate mech_core;

use na::*;
use num_traits::*;
use std::ops::*;
use std::fmt::Debug;
use mech_core::matrix::Matrix;

//pub mod sum_column;
pub mod sum_row;

//pub use self::sum_column::*;
pub use self::sum_row::*;

#[macro_export]  
macro_rules! impl_stats_urop {
  ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      arg: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + 'static + 
      Add<Output = T> + AddAssign +
      Zero + One +
      PartialEq + PartialOrd,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(arg_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

