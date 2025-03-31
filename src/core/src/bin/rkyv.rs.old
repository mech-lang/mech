#![feature(trivial_bounds)]

use rkyv::{Archive, Deserialize, Serialize, Portable, Place};
use rkyv::vec::{ArchivedVec, VecResolver};
use rkyv::ser::{Writer,Allocator};
use rkyv::primitive::ArchivedF32;
use nalgebra::{Matrix2, Matrix3};
use rkyv::rancor::{Error, Fallible};
use rkyv::to_bytes;
use rkyv::access_unchecked;
use std::rc::Rc;


// Define the Matrix enum
#[repr(u8)]
#[derive(Debug, Portable)]
enum Matrix<T> {
    Matrix2(Matrix2<T>),
    Matrix3(Matrix3<T>),
}

// Archived enum representation
#[derive(Archive, Serialize, Deserialize, Debug)]
enum ArchivedMatrix<T> {
  Matrix2([[T; 2]; 2]), // Store as a 2x2 array
  Matrix3([[T; 3]; 3]), // Store as a 3x3 array
}

impl<T: Archive> Archive for Matrix<T> {
  type Archived = ArchivedVec<T::Archived>;
  type Resolver = VecResolver;

  fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
  match self {
      Matrix::Matrix2(mat) => {
        ArchivedVec::resolve_from_slice(mat.as_slice(), resolver, out);
      }
      Matrix::Matrix3(mat) => {
        ArchivedVec::resolve_from_slice(mat.as_slice(), resolver, out);
      }
    }
  }
}

impl<T: Serialize<S>, S: Fallible + Allocator + Writer + ?Sized> Serialize<S>
    for Matrix<T>
{
  fn serialize(
      &self,
      serializer: &mut S,
  ) -> Result<Self::Resolver, S::Error> {
    match self {
      Matrix::Matrix2(mat) => {
        ArchivedVec::<T::Archived>::serialize_from_slice(mat.as_slice(),serializer)
      }
      Matrix::Matrix3(mat) => {
        ArchivedVec::<T::Archived>::serialize_from_slice(mat.as_slice(),serializer)
      }
    }
  }
}

#[derive(Debug, Archive, Serialize)]
enum Value {
  U32(Rc<u32>),
  F32(Rc<f32>),
  MatrixF32(Rc<Matrix<f32>>),
  MatrixU32(Rc<Matrix<u32>>),
}

fn main() {
    let v = Value::MatrixF32(Matrix::Matrix2(Matrix2::new(1.0,2.0,3.0,4.0)).into());

    let buf = to_bytes::<Error>(&v).expect("failed to serialize");
    let archived = unsafe { access_unchecked::<ArchivedValue>(buf.as_ref()) };
    let value = match archived {
      /*ArchivedValue::U32(x) => {
        println!("{:?}", x);
      }
      ArchivedValue::F32(x) => {
        println!("{:?}", x);
      }
      ArchivedValue::MatrixF32(mat) => {
        let data: Vec<f32> = mat.as_slice().iter().map(|&x| x.into()).collect();
        let m = Matrix2::from_vec(data);
        Value::MatrixF32(Rc::new(Matrix::Matrix2(m)))
      }*/
      ArchivedValue::MatrixF32(mat) => {
        let data: Vec<f32> = mat.as_slice().iter().map(|&x| x.into()).collect();
        let m = Matrix2::from_vec(data);
        Value::MatrixF32(Rc::new(Matrix::Matrix2(m)))
      }
      _ => todo!(),
    };
    println!("{:?}", value);
}