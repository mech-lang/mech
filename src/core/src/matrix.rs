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

// Matrix ---------------------------------------------------------------------

pub trait ToMatrix: Clone {
  fn to_matrix(elements: Vec<Self>, rows: usize, cols: usize) -> Matrix<Self>;
}
  
macro_rules! impl_to_matrix {
  ($t:ty) => {
    impl ToMatrix for $t {
      fn to_matrix(elements: Vec<Self>, rows: usize, cols: usize) -> Matrix<Self> {
        match (rows,cols) {
          (1,1) => Matrix::DMatrix(new_ref(DMatrix::from_vec(1,1,elements))),
          (1,n) => Matrix::RowDVector(new_ref(RowDVector::from_vec(elements))),
          (m,1) => Matrix::DVector(new_ref(DVector::from_vec(elements))),
          (m,n) => Matrix::DMatrix(new_ref(DMatrix::from_vec(m,n,elements))),
        }}}};}

impl ToMatrix for usize {
  fn to_matrix(elements: Vec<Self>, rows: usize, cols: usize) -> Matrix<Self> {
    match (rows,cols) {
      (1,1) => Matrix::DMatrix(new_ref(DMatrix::from_vec(1,1,elements))),
      (1,n) => Matrix::RowDVector(new_ref(RowDVector::from_vec(elements))),
      (m,1) => Matrix::DVector(new_ref(DVector::from_vec(elements))),
      (m,n) => Matrix::DMatrix(new_ref(DMatrix::from_vec(m,n,elements))),
    }
  }
}

impl_to_matrix!(Value);
impl_to_matrix!(bool);
impl_to_matrix!(u8);
impl_to_matrix!(u16);
impl_to_matrix!(u32);
impl_to_matrix!(u64);
impl_to_matrix!(u128);
impl_to_matrix!(i8);
impl_to_matrix!(i16);
impl_to_matrix!(i32);
impl_to_matrix!(i64);
impl_to_matrix!(i128);
impl_to_matrix!(F32);
impl_to_matrix!(F64);
  
pub trait ToIndex: Clone {
  fn to_index(elements: Vec<Self>) -> Matrix<Self>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Matrix<T> {
  DMatrix(Ref<DMatrix<T>>),
  DVector(Ref<DVector<T>>),
  RowDVector(Ref<RowDVector<T>>),
}
  
impl<T> Hash for Matrix<T> 
where T: Hash + na::Scalar
{
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self {
      Matrix::DMatrix(x) => x.borrow().hash(state),
      Matrix::RowDVector(x) => x.borrow().hash(state),
      Matrix::DVector(x) => x.borrow().hash(state),
    }
  }
}

impl<T> Matrix<T> 
where T: Debug + Clone + PartialEq + 'static
{

  pub fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    match self {
      Matrix::DMatrix(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      Matrix::RowDVector(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      Matrix::DVector(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()));}
      _ => todo!(),
    };
    let mut table = builder.build();
    table.with(Style::modern());
    format!("{table}")
  }

  pub fn shape(&self) -> Vec<usize> {
    let shape = match self {
      Matrix::DMatrix(x) => x.borrow().shape(),
      Matrix::RowDVector(x) => x.borrow().shape(),
      Matrix::DVector(x) => x.borrow().shape(),
    };
    vec![shape.0, shape.1]
  }

  pub fn index1d(&self, ix: usize) -> T {
    match self {
      Matrix::DMatrix(x) => (*x.borrow().index(ix-1)).clone(),
      Matrix::RowDVector(x) => (*x.borrow().index(ix-1)).clone(),
      Matrix::DVector(x) => (*x.borrow().index(ix-1)).clone(),
    }
  }

  pub fn index2d(&self, row: usize, col: usize) -> T {
    match self {
      Matrix::DMatrix(x) => (*x.borrow().index((row-1,col-1))).clone(),
      Matrix::RowDVector(x) => (*x.borrow().index((row-1,col-1))).clone(),
      Matrix::DVector(x) => (*x.borrow().index((row-1,col-1))).clone(),
    }
  }

  pub fn as_vec(&self) -> Vec<T> {
    match self {
      Matrix::DMatrix(x) => x.borrow().as_slice().to_vec(),
      Matrix::RowDVector(x) => x.borrow().as_slice().to_vec(),
      Matrix::DVector(x) => x.borrow().as_slice().to_vec(),
    }
  }
}


impl ToValue for Matrix<Value> {

  fn to_value(&self) -> Value {
    Value::MatrixValue(self.clone())
  }
  
}