use crate::*;
use crate::types::Ref;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use tabled::{
    builder::Builder,
    settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
    Tabled,
  };
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::slice::Iter;
use std::iter::Peekable;
use serde::ser::{Serialize, Serializer, SerializeStruct};

// Matrix ---------------------------------------------------------------------

pub trait ToMatrix: Clone {
  fn to_matrix(elements: Vec<Self>, rows: usize, cols: usize) -> Matrix<Self>;
}
  
macro_rules! impl_to_matrix {
  ($t:ty) => {
    impl ToMatrix for $t {
      fn to_matrix(elements: Vec<Self>, rows: usize, cols: usize) -> Matrix<Self> {
        match (rows,cols) {
          #[cfg(feature = "Matrix1")]
          (1,1) => Matrix::Matrix1(new_ref(Matrix1::from_element(elements[0].clone()))),
          #[cfg(feature = "Matrix2")]
          (2,2) => Matrix::Matrix2(new_ref(Matrix2::from_vec(elements))),
          #[cfg(feature = "Matrix3")]
          (3,3) => Matrix::Matrix3(new_ref(Matrix3::from_vec(elements))),
          #[cfg(feature = "Matrix4")]
          (4,4) => Matrix::Matrix4(new_ref(Matrix4::from_vec(elements))),
          #[cfg(feature = "Matrix2x3")]
          (2,3) => Matrix::Matrix2x3(new_ref(Matrix2x3::from_vec(elements))),
          #[cfg(feature = "Matrix3x2")]
          (3,2) => Matrix::Matrix3x2(new_ref(Matrix3x2::from_vec(elements))),
          #[cfg(feature = "RowVector2")]
          (1,2) => Matrix::RowVector2(new_ref(RowVector2::from_vec(elements))),
          #[cfg(feature = "RowVector3")]
          (1,3) => Matrix::RowVector3(new_ref(RowVector3::from_vec(elements))),
          #[cfg(feature = "RowVector4")]
          (1,4) => Matrix::RowVector4(new_ref(RowVector4::from_vec(elements))),
          #[cfg(feature = "Vector2")]
          (2,1) => Matrix::Vector2(new_ref(Vector2::from_vec(elements))),
          #[cfg(feature = "Vector2")]
          (3,1) => Matrix::Vector3(new_ref(Vector3::from_vec(elements))),
          #[cfg(feature = "Vector2")]
          (4,1) => Matrix::Vector4(new_ref(Vector4::from_vec(elements))),
          #[cfg(feature = "RowVectorD")]
          (1,n) => Matrix::RowDVector(new_ref(RowDVector::from_vec(elements))),
          #[cfg(feature = "VectorD")]
          (m,1) => Matrix::DVector(new_ref(DVector::from_vec(elements))),
          #[cfg(feature = "MatrixD")]
          (m,n) => Matrix::DMatrix(new_ref(DMatrix::from_vec(m,n,elements))),
        }}}};}

impl ToMatrix for usize {
  fn to_matrix(elements: Vec<Self>, rows: usize, cols: usize) -> Matrix<Self> {
    match (rows,cols) {
      (1,n) => Matrix::RowDVector(new_ref(RowDVector::from_vec(elements))),
      (m,1) => Matrix::DVector(new_ref(DVector::from_vec(elements))),
      (m,n) => Matrix::DMatrix(new_ref(DMatrix::from_vec(m,n,elements))),
    }
  }
}

impl_to_matrix!(Value);
#[cfg(feature = "Bool")]
impl_to_matrix!(bool);
#[cfg(feature = "U8")]
impl_to_matrix!(u8);
#[cfg(feature = "U16")]
impl_to_matrix!(u16);
#[cfg(feature = "U32")]
impl_to_matrix!(u32);
#[cfg(feature = "U64")]
impl_to_matrix!(u64);
#[cfg(feature = "U128")]
impl_to_matrix!(u128);
#[cfg(feature = "I8")]
impl_to_matrix!(i8);
#[cfg(feature = "I16")]
impl_to_matrix!(i16);
#[cfg(feature = "I32")]
impl_to_matrix!(i32);
#[cfg(feature = "I64")]
impl_to_matrix!(i64);
#[cfg(feature = "I128")]
impl_to_matrix!(i128);
#[cfg(feature = "F32")]
impl_to_matrix!(F32);
#[cfg(feature = "F64")]
impl_to_matrix!(F64);
  
pub trait ToIndex: Clone {
  fn to_index(elements: Vec<Self>) -> Matrix<Self>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Matrix<T> {
  #[cfg(feature = "RowVector4")]
  RowVector4(Ref<RowVector4<T>>),
  #[cfg(feature = "RowVector3")]
  RowVector3(Ref<RowVector3<T>>),
  #[cfg(feature = "RowVector2")]
  RowVector2(Ref<RowVector2<T>>),
  #[cfg(feature = "RowVectorD")]
  RowDVector(Ref<RowDVector<T>>),
  #[cfg(feature = "Vector4")]
  Vector4(Ref<Vector4<T>>),  
  #[cfg(feature = "Vector3")]
  Vector3(Ref<Vector3<T>>),
  #[cfg(feature = "Vector2")]
  Vector2(Ref<Vector2<T>>),
  #[cfg(feature = "VectorD")]
  DVector(Ref<DVector<T>>),
  #[cfg(feature = "Matrix4")]
  Matrix4(Ref<Matrix4<T>>),
  #[cfg(feature = "Matrix3")]
  Matrix3(Ref<Matrix3<T>>),
  #[cfg(feature = "Matrix2")]
  Matrix2(Ref<Matrix2<T>>),
  #[cfg(feature = "Matrix1")]
  Matrix1(Ref<Matrix1<T>>),
  #[cfg(feature = "Matrix3x2")]
  Matrix3x2(Ref<Matrix3x2<T>>),
  #[cfg(feature = "Matrix2x3")]
  Matrix2x3(Ref<Matrix2x3<T>>),
  #[cfg(feature = "MatrixD")]
  DMatrix(Ref<DMatrix<T>>),
}

impl<T: Clone + Serialize> Serialize for Matrix<T> {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    match self {
      #[cfg(feature = "RowVector4")]
      Matrix::RowVector4(x) => x.borrow().as_slice().serialize(serializer),
      #[cfg(feature = "RowVector3")]
      Matrix::RowVector3(x) => x.borrow().as_slice().serialize(serializer),
      #[cfg(feature = "RowVector2")]
      Matrix::RowVector2(x) => x.borrow().as_slice().serialize(serializer),
      #[cfg(feature = "RowVectorD")]
      Matrix::RowDVector(x) => x.borrow().as_slice().serialize(serializer),
      #[cfg(feature = "Vector4")]
      Matrix::Vector4(x) => x.borrow().as_slice().serialize(serializer),
      #[cfg(feature = "Vector3")]
      Matrix::Vector3(x) => x.borrow().as_slice().serialize(serializer),
      #[cfg(feature = "Vector2")]
      Matrix::Vector2(x) => x.borrow().as_slice().serialize(serializer),
      #[cfg(feature = "VectorD")]
      Matrix::DVector(x) => x.borrow().as_slice().serialize(serializer),
      #[cfg(feature = "Matrix4")]
      Matrix::Matrix4(x) => x.borrow().as_slice().serialize(serializer),
      #[cfg(feature = "Matrix3")]
      Matrix::Matrix3(x) => x.borrow().as_slice().serialize(serializer),
      #[cfg(feature = "Matrix2")]
      Matrix::Matrix2(x) => x.borrow().as_slice().serialize(serializer),
      #[cfg(feature = "Matrix1")]
      Matrix::Matrix1(x) => x.borrow().as_slice().serialize(serializer),
      #[cfg(feature = "Matrix3x2")]
      Matrix::Matrix3x2(x) => x.borrow().as_slice().serialize(serializer),
      #[cfg(feature = "Matrix2x3")]
      Matrix::Matrix2x3(x) => x.borrow().as_slice().serialize(serializer),
      #[cfg(feature = "MatrixD")]
      Matrix::DMatrix(x) => x.borrow().as_slice().serialize(serializer),
      _ => todo!(),
    }
  }
} 

pub trait CopyMat<T> {
  fn copy_into(&self, dst: &Ref<DMatrix<T>>, offset: usize) -> usize;
  fn copy_into_v(&self, dst: &Ref<DVector<T>>, offset: usize) -> usize;
  fn copy_into_r(&self, dst: &Ref<RowDVector<T>>, offset: usize) -> usize;
  fn copy_into_row_major(&self, dst: &Ref<DMatrix<T>>, offset: usize) -> usize;
}

macro_rules! copy_mat {
  ($matsize:ident) => {
    impl<T> CopyMat<T> for Ref<$matsize<T>> 
    where T: Clone 
    {
      fn copy_into(&self, dst: &Ref<DMatrix<T>>, offset: usize) -> usize {
        let src_ptr = unsafe { (*(self.as_ptr())).clone() };
        let mut dst_ptr = unsafe { &mut *(dst.as_ptr()) };
        for i in 0..src_ptr.len() {
            dst_ptr[i + offset] = src_ptr[i].clone();
        }
        src_ptr.len()
      }
      fn copy_into_v(&self, dst: &Ref<DVector<T>>, offset: usize) -> usize {
        let src_ptr = unsafe { (*(self.as_ptr())).clone() };
        let mut dst_ptr = unsafe { &mut *(dst.as_ptr()) };
        for i in 0..src_ptr.len() {
            dst_ptr[i + offset] = src_ptr[i].clone();
        }
        src_ptr.len()
      }
      fn copy_into_r(&self, dst: &Ref<RowDVector<T>>, offset: usize) -> usize {
        let src_ptr = unsafe { (*(self.as_ptr())).clone() };
        let mut dst_ptr = unsafe { &mut *(dst.as_ptr()) };
        for i in 0..src_ptr.len() {
            dst_ptr[i + offset] = src_ptr[i].clone();
        }
        src_ptr.len()
      }
      fn copy_into_row_major(&self, dst: &Ref<DMatrix<T>>, offset: usize) -> usize {
        let src_ptr = unsafe { (*(self.as_ptr())).clone() };
        let mut dst_ptr = unsafe { &mut *(dst.as_ptr()) };
        let src_rows = src_ptr.nrows();
        let dest_rows = dst_ptr.nrows();

        let stride = dest_rows - src_rows;
        let mut offset = offset;
        for ix in 0..src_ptr.len() {
            dst_ptr[offset] = src_ptr[ix].clone();
            offset += ((ix + 1) % src_rows == 0) as usize * stride + 1;
        }
        src_rows
      }}};}
      
copy_mat!(DMatrix);
copy_mat!(Matrix1);
copy_mat!(Matrix2);
copy_mat!(Matrix3);
copy_mat!(Matrix4);
copy_mat!(Matrix2x3);
copy_mat!(Matrix3x2);
copy_mat!(Vector2);
copy_mat!(Vector3);
copy_mat!(Vector4);
copy_mat!(DVector);
copy_mat!(RowVector2);
copy_mat!(RowVector3);
copy_mat!(RowVector4);
copy_mat!(RowDVector);

impl<T> Hash for Matrix<T> 
where T: Hash + na::Scalar
{
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self {
      #[cfg(feature = "RowVector4")]
      Matrix::RowVector4(x) => x.borrow().hash(state),
      #[cfg(feature = "RowVector3")]
      Matrix::RowVector3(x) => x.borrow().hash(state),
      #[cfg(feature = "RowVector2")]
      Matrix::RowVector2(x) => x.borrow().hash(state),
      #[cfg(feature = "RowVectorD")]
      Matrix::RowDVector(x) => x.borrow().hash(state),
      #[cfg(feature = "Vector4")]
      Matrix::Vector4(x) => x.borrow().hash(state),
      #[cfg(feature = "Vector3")]
      Matrix::Vector3(x) => x.borrow().hash(state),
      #[cfg(feature = "Vector2")]
      Matrix::Vector2(x) => x.borrow().hash(state),
      #[cfg(feature = "VectorD")]
      Matrix::DVector(x) => x.borrow().hash(state),
      #[cfg(feature = "Matrix4")]
      Matrix::Matrix4(x) => x.borrow().hash(state),
      #[cfg(feature = "Matrix3")]
      Matrix::Matrix3(x) => x.borrow().hash(state),
      #[cfg(feature = "Matrix2")]
      Matrix::Matrix2(x) => x.borrow().hash(state),
      #[cfg(feature = "Matrix1")]
      Matrix::Matrix1(x) => x.borrow().hash(state),
      #[cfg(feature = "Matrix3x2")]
      Matrix::Matrix3x2(x) => x.borrow().hash(state),
      #[cfg(feature = "Matrix2x3")]
      Matrix::Matrix2x3(x) => x.borrow().hash(state),
      #[cfg(feature = "MatrixD")]
      Matrix::DMatrix(x) => x.borrow().hash(state),
    }
  }
}

impl<T> Matrix<T> 
where T: Debug + Clone + PartialEq + 'static
{

  pub fn size_of(&self) -> usize {
    let vec = self.as_vec();
    vec.capacity() * size_of::<T>()
  }

  pub fn get_copyable_matrix(&self) -> Box<dyn CopyMat<T>> {
    match self {
      #[cfg(feature = "RowVector4")]
      Matrix::RowVector4(ref x) => Box::new(x.clone()),
      #[cfg(feature = "RowVector3")]
      Matrix::RowVector3(ref x) => Box::new(x.clone()),
      #[cfg(feature = "RowVector2")]
      Matrix::RowVector2(ref x) => Box::new(x.clone()),
      #[cfg(feature = "RowVectorD")]
      Matrix::RowDVector(ref x) => Box::new(x.clone()),
      #[cfg(feature = "Vector4")]
      Matrix::Vector4(ref x) => Box::new(x.clone()),
      #[cfg(feature = "Vector3")]
      Matrix::Vector3(ref x) => Box::new(x.clone()),
      #[cfg(feature = "Vector2")]
      Matrix::Vector2(ref x) => Box::new(x.clone()),
      #[cfg(feature = "VectorD")]
      Matrix::DVector(ref x) => Box::new(x.clone()),
      #[cfg(feature = "Matrix4")]
      Matrix::Matrix4(ref x) => Box::new(x.clone()),
      #[cfg(feature = "Matrix3")]
      Matrix::Matrix3(ref x) => Box::new(x.clone()),
      #[cfg(feature = "Matrix2")]
      Matrix::Matrix2(ref x) => Box::new(x.clone()),
      #[cfg(feature = "Matrix1")]
      Matrix::Matrix1(ref x) => Box::new(x.clone()),
      #[cfg(feature = "Matrix3x2")]
      Matrix::Matrix3x2(ref x) => Box::new(x.clone()),
      #[cfg(feature = "Matrix2x3")]
      Matrix::Matrix2x3(ref x) => Box::new(x.clone()),
      #[cfg(feature = "MatrixD")]
      Matrix::DMatrix(ref x) => Box::new(x.clone()),
    }
  }


  pub fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    match self {
      #[cfg(feature = "RowVector4")]
      Matrix::RowVector4(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      #[cfg(feature = "RowVector3")]
      Matrix::RowVector3(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      #[cfg(feature = "RowVector2")]
      Matrix::RowVector2(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      #[cfg(feature = "RowVectorD")]
      Matrix::RowDVector(vec) => {
        let vec_brrw = vec.borrow();
        let vec_str = if vec_brrw.ncols() > 20 {
          let mut vec_str = vec_brrw.row(0).iter().take(10).chain(vec_brrw.row(0).iter().rev().take(9).rev()).map(|x| format!("{:?}", x)).collect::<Vec<_>>();
          vec_str.insert(10,"...".to_string());
          vec_str
        } else {
          vec_brrw.row(0).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()
        };
        builder.push_record(vec_str);
      }
      #[cfg(feature = "Vector4")]
      Matrix::Vector4(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      #[cfg(feature = "Vector3")]
      Matrix::Vector3(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      #[cfg(feature = "Vector2")]
      Matrix::Vector2(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      #[cfg(feature = "VectorD")]
      Matrix::DVector(vec) => {
        let vec_brrw = vec.borrow();
        let vec_str = if vec_brrw.nrows() > 20 {
          let mut vec_str = vec_brrw.column(0).iter().take(10).chain(vec_brrw.column(0).iter().rev().take(9).rev()).map(|x| format!("{:?}", x)).collect::<Vec<_>>();
          vec_str.insert(10,"...".to_string());
          vec_str
        } else {
          vec_brrw.column(0).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()
        };
        for r in vec_str {
          builder.push_record(vec![r]);
        }
      }
      #[cfg(feature = "Matrix4")]
      Matrix::Matrix4(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      #[cfg(feature = "Matrix3")]
      Matrix::Matrix3(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      #[cfg(feature = "Matrix2")]
      Matrix::Matrix2(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      #[cfg(feature = "Matrix1")]
      Matrix::Matrix1(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      #[cfg(feature = "Matrix3x2")]
      Matrix::Matrix3x2(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      #[cfg(feature = "Matrix2x3")]
      Matrix::Matrix2x3(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      #[cfg(feature = "MatrixD")]
      Matrix::DMatrix(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      _ => todo!(),
    };
    let mut table = builder.build();
    table.with(Style::modern_rounded());
    format!("{table}")
  }

  pub fn shape(&self) -> Vec<usize> {
    let shape = match self {
      #[cfg(feature = "RowVector4")]
      Matrix::RowVector4(x) => x.borrow().shape(),
      #[cfg(feature = "RowVector3")]
      Matrix::RowVector3(x) => x.borrow().shape(),
      #[cfg(feature = "RowVector2")]
      Matrix::RowVector2(x) => x.borrow().shape(),
      #[cfg(feature = "RowVectorD")]
      Matrix::RowDVector(x) => x.borrow().shape(),
      #[cfg(feature = "Vector4")]
      Matrix::Vector4(x) => x.borrow().shape(),
      #[cfg(feature = "Vector3")]
      Matrix::Vector3(x) => x.borrow().shape(),
      #[cfg(feature = "Vector2")]
      Matrix::Vector2(x) => x.borrow().shape(),
      #[cfg(feature = "VectorD")]
      Matrix::DVector(x) => x.borrow().shape(),
      #[cfg(feature = "Matrix4")]
      Matrix::Matrix4(x) => x.borrow().shape(),
      #[cfg(feature = "Matrix3")]
      Matrix::Matrix3(x) => x.borrow().shape(),
      #[cfg(feature = "Matrix2")]
      Matrix::Matrix2(x) => x.borrow().shape(),
      #[cfg(feature = "Matrix1")]
      Matrix::Matrix1(x) => x.borrow().shape(),
      #[cfg(feature = "Matrix3x2")]
      Matrix::Matrix3x2(x) => x.borrow().shape(),
      #[cfg(feature = "Matrix2x3")]
      Matrix::Matrix2x3(x) => x.borrow().shape(),
      #[cfg(feature = "MatrixD")]
      Matrix::DMatrix(x) => x.borrow().shape(),
    };
    vec![shape.0, shape.1]
  }

  pub fn index1d(&self, ix: usize) -> T {
    match self {
      #[cfg(feature = "RowVector4")]
      Matrix::RowVector4(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "RowVector3")]
      Matrix::RowVector3(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "RowVector2")]
      Matrix::RowVector2(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "RowVectorD")]
      Matrix::RowDVector(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "Vector4")]
      Matrix::Vector4(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "Vector3")]
      Matrix::Vector3(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "Vector2")]
      Matrix::Vector2(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "VectorD")]
      Matrix::DVector(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "Matrix4")]
      Matrix::Matrix4(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "Matrix3")]
      Matrix::Matrix3(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "Matrix2")]
      Matrix::Matrix2(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "Matrix1")]
      Matrix::Matrix1(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "Matrix3x2")]
      Matrix::Matrix3x2(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "Matrix2x3")]
      Matrix::Matrix2x3(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "MatrixD")]
      Matrix::DMatrix(x) => (*x.borrow().index(ix-1)).clone(),
    }
  }

  pub fn index2d(&self, row: usize, col: usize) -> T {
    match self {
      #[cfg(feature = "RowVector4")]
      Matrix::RowVector4(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "RowVector3")]
      Matrix::RowVector3(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "RowVector2")]
      Matrix::RowVector2(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "RowVectorD")]
      Matrix::RowDVector(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "Vector4")]
      Matrix::Vector4(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "Vector3")]
      Matrix::Vector3(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "Vector2")]
      Matrix::Vector2(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "VectorD")]
      Matrix::DVector(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "Matrix4")]
      Matrix::Matrix4(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "Matrix3")]
      Matrix::Matrix3(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "Matrix2")]
      Matrix::Matrix2(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "Matrix1")]
      Matrix::Matrix1(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "Matrix3x2")]
      Matrix::Matrix3x2(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "Matrix2x3")]
      Matrix::Matrix2x3(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "MatrixD")]
      Matrix::DMatrix(x) => (*x.borrow().index((row-1,col-1))).clone(),
    }
  }

  pub fn as_vec(&self) -> Vec<T> {
    match self {
      #[cfg(feature = "RowVector4")]
      Matrix::RowVector4(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "RowVector3")]
      Matrix::RowVector3(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "RowVector2")]
      Matrix::RowVector2(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "RowVectorD")]
      Matrix::RowDVector(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "Vector4")]
      Matrix::Vector4(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "Vector3")]
      Matrix::Vector3(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "Vector2")]
      Matrix::Vector2(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "VectorD")]
      Matrix::DVector(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "Matrix4")]
      Matrix::Matrix4(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "Matrix3")]
      Matrix::Matrix3(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "Matrix2")]
      Matrix::Matrix2(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "Matrix1")]
      Matrix::Matrix1(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "Matrix3x2")]
      Matrix::Matrix3x2(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "Matrix2x3")]
      Matrix::Matrix2x3(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "MatrixD")]
      Matrix::DMatrix(x) => x.borrow().as_slice().to_vec(),
    }
  }

}


impl ToValue for Matrix<Value> {

  fn to_value(&self) -> Value {
    Value::MatrixValue(self.clone())
  }
  
}