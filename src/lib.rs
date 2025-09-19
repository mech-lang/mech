#![feature(step_trait)]
#![no_main]
#![allow(warnings)]
#[macro_use]
extern crate mech_core;
#[cfg(feature = "matrix")]
extern crate nalgebra as na;
extern crate paste;

use paste::paste;

#[cfg(feature = "vector3")]
use nalgebra::Vector3;
#[cfg(feature = "vectord")]
use nalgebra::DVector;
#[cfg(feature = "vector2")]
use nalgebra::Vector2;
#[cfg(feature = "vector4")]
use nalgebra::Vector4;
#[cfg(feature = "rowdvector")]
use nalgebra::RowDVector;
#[cfg(feature = "row_vectord")]
use nalgebra::RowDVector;
#[cfg(feature = "matrix1")]
use nalgebra::Matrix1;
#[cfg(feature = "matrix3")]
use nalgebra::Matrix3;
#[cfg(feature = "matrix4")]
use nalgebra::Matrix4;
#[cfg(feature = "row_vector3")]
use nalgebra::RowVector3;
#[cfg(feature = "row_vector4")]
use nalgebra::RowVector4;
#[cfg(feature = "row_vector2")]
use nalgebra::RowVector2;
#[cfg(feature = "matrixd")]
use nalgebra::DMatrix;
#[cfg(feature = "matrix2x3")]
use nalgebra::Matrix2x3;
#[cfg(feature = "matrix3x2")]
use nalgebra::Matrix3x2;
#[cfg(feature = "matrix2")]
use nalgebra::Matrix2;

use std::ops::*;
#[cfg(feature = "range")]
use num_traits::{Zero, One};
use std::fmt::Debug;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

#[cfg(feature = "exclusive")]
pub mod exclusive;
#[cfg(feature = "inclusive")]
pub mod inclusive;

#[cfg(feature = "exclusive")]
pub use self::exclusive::*;
#[cfg(feature = "inclusive")]
pub use self::inclusive::*;

// ----------------------------------------------------------------------------
// Range Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! register_range {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt) => {
    paste! {
      inventory::submit! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), ">") ,
          ptr: $fxn_name::<$scalar, $row1<$scalar>>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! impl_range_match_arms {
  ($fxn:ident, $arg1:expr, $arg2:expr, $($ty:tt, $feat:tt);+ $(;)?) => {
    paste! {
      match ($arg1, $arg2) {
        $(
          #[cfg(feature = $feat)]
          (Value::[<$ty:camel>](from), Value::[<$ty:camel>](to))  => {
            let from_val = *from.borrow();
            let to_val = *to.borrow();
            let diff = to_val - from_val;
            if diff < $ty::zero() {
              return Err(MechError {file: file!().to_string(),tokens: vec![],msg: "Range size must be > 0".to_string(),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind,});
            }
            let size = (diff + $ty::one()).try_into().map_err(|_| MechError {file: file!().to_string(),tokens: vec![],msg: "Range size overflow".to_string(),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind,})?;            
            let mut vec = vec![from_val; size];
            match size {
              0 => Err(MechError {file: file!().to_string(),tokens: vec![],msg: "Range size must be > 0".to_string(),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind,}),
              #[cfg(feature = "matrix1")]
              1 => {
                register_range!($fxn, $ty, $feat, Matrix1);
                Ok(Box::new($fxn::<$ty,Matrix1<$ty>>{from: from.clone(), to: to.clone(), out: Ref::new(Matrix1::from_element(vec[0])), phantom: PhantomData::default()}))
              }
              #[cfg(all(not(feature = "matrix1"), feature = "matrixd")  )]
              1 => {
                register_range!($fxn, $ty, $feat, DMatrix);
                Ok(Box::new($fxn::<$ty,DMatrix<$ty>>{from: from.clone(), to: to.clone(), out: Ref::new(DMatrix::from_element(1,1,vec[0])), phantom: PhantomData::default()}))
              }
              #[cfg(feature = "row_vector2")]
              2 => {
                register_range!($fxn, $ty, $feat, RowVector2);
                Ok(Box::new($fxn::<$ty,RowVector2<$ty>>{from: from.clone(), to: to.clone(), out: Ref::new(RowVector2::from_vec(vec)), phantom: PhantomData::default()}))
              }
              #[cfg(feature = "row_vector3")]
              3 => {              
                register_range!($fxn, $ty, $feat, RowVector3);
                Ok(Box::new($fxn::<$ty,RowVector3<$ty>>{from: from.clone(), to: to.clone(), out: Ref::new(RowVector3::from_vec(vec)), phantom: PhantomData::default()}))
              }
              #[cfg(feature = "row_vector4")]
              4 => {
                register_range!($fxn, $ty, $feat, RowVector4);
                Ok(Box::new($fxn::<$ty,RowVector4<$ty>>{from: from.clone(), to: to.clone(), out: Ref::new(RowVector4::from_vec(vec)), phantom: PhantomData::default()}))
              }
              #[cfg(feature = "row_vectord")]
              n => {
                register_range!($fxn, $ty, $feat, RowDVector);
                Ok(Box::new($fxn::<$ty,RowDVector<$ty>>{from: from.clone(), to: to.clone(), out: Ref::new(RowDVector::from_vec(vec)), phantom: PhantomData::default()}))
              }
            }
          }
        )+
        x => Err(MechError {file: file!().to_string(),tokens: vec![],msg: format!("{:?}", x),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind,})
      }
    }
  }
}