use crate::*;

#[cfg(feature = "no_std")]
use core::any::Any;
#[cfg(not(feature = "no_std"))]
use std::any::Any;

use nalgebra::{DMatrix, DVector, RowDVector};

// Matrix ---------------------------------------------------------------------

pub trait ToMatrix: Clone {
  fn to_matrix(elements: Vec<Self>, rows: usize, cols: usize) -> Matrix<Self>;
  fn to_matrixd(elements: Vec<Self>, rows: usize, cols: usize) -> Matrix<Self>;
}
  
macro_rules! impl_to_matrix {
  ($t:ty) => {
    impl ToMatrix for $t {
      fn to_matrix(elements: Vec<Self>, rows: usize, cols: usize) -> Matrix<Self> {
        match (rows,cols) {
          #[cfg(feature = "matrix1")]
          (1,1) => Matrix::Matrix1(Ref::new(Matrix1::from_element(elements[0].clone()))),
          #[cfg(feature = "matrix2")]
          (2,2) => Matrix::Matrix2(Ref::new(Matrix2::from_vec(elements))),
          #[cfg(feature = "matrix3")]
          (3,3) => Matrix::Matrix3(Ref::new(Matrix3::from_vec(elements))),
          #[cfg(feature = "matrix4")]
          (4,4) => Matrix::Matrix4(Ref::new(Matrix4::from_vec(elements))),
          #[cfg(feature = "matrix2x3")]
          (2,3) => Matrix::Matrix2x3(Ref::new(Matrix2x3::from_vec(elements))),
          #[cfg(feature = "matrix3x2")]
          (3,2) => Matrix::Matrix3x2(Ref::new(Matrix3x2::from_vec(elements))),
          #[cfg(feature = "row_vector2")]
          (1,2) => Matrix::RowVector2(Ref::new(RowVector2::from_vec(elements))),
          #[cfg(feature = "row_vector3")]
          (1,3) => Matrix::RowVector3(Ref::new(RowVector3::from_vec(elements))),
          #[cfg(feature = "row_vector4")]
          (1,4) => Matrix::RowVector4(Ref::new(RowVector4::from_vec(elements))),
          #[cfg(feature = "vector2")]
          (2,1) => Matrix::Vector2(Ref::new(Vector2::from_vec(elements))),
          #[cfg(feature = "vector3")]
          (3,1) => Matrix::Vector3(Ref::new(Vector3::from_vec(elements))),
          #[cfg(feature = "vector4")]
          (4,1) => Matrix::Vector4(Ref::new(Vector4::from_vec(elements))),
          (1,n) => Matrix::RowDVector(Ref::new(RowDVector::from_vec(elements))),
          (m,1) => Matrix::DVector(Ref::new(DVector::from_vec(elements))),
          (m,n) => Matrix::DMatrix(Ref::new(DMatrix::from_vec(m,n,elements))),
          _ => panic!("Cannot convert to matrix with rows: {rows} and cols: {cols}"),
        }
      }
      fn to_matrixd(elements: Vec<Self>, rows: usize, cols: usize) -> Matrix<Self> {
        match (rows,cols) {
          (1,n) => Matrix::RowDVector(Ref::new(RowDVector::from_vec(elements))),
          (m,1) => Matrix::DVector(Ref::new(DVector::from_vec(elements))),
          (m,n) => Matrix::DMatrix(Ref::new(DMatrix::from_vec(m,n,elements))),
          _ => panic!("Cannot convert to matrixd with rows: {rows} and cols: {cols}"),
        }
      }
    }
  };    
}

impl ToMatrix for usize {
  fn to_matrix(elements: Vec<Self>, rows: usize, cols: usize) -> Matrix<Self> {
    match (rows,cols) {
      (1,n) => Matrix::RowDVector(Ref::new(RowDVector::from_vec(elements))),
      (m,1) => Matrix::DVector(Ref::new(DVector::from_vec(elements))),
      (m,n) => Matrix::DMatrix(Ref::new(DMatrix::from_vec(m,n,elements))),
      _ => panic!("Cannot convert to matrix with rows: {rows} and cols: {cols}"),
    }
  }
  fn to_matrixd(elements: Vec<Self>, rows: usize, cols: usize) -> Matrix<Self> {
    match (rows,cols) {
      (1,n) => Matrix::RowDVector(Ref::new(RowDVector::from_vec(elements))),
      (m,1) => Matrix::DVector(Ref::new(DVector::from_vec(elements))),
      (m,n) => Matrix::DMatrix(Ref::new(DMatrix::from_vec(m,n,elements))),
      _ => panic!("Cannot convert to matrixd with rows: {rows} and cols: {cols}"),
    }
  }
}

impl_to_matrix!(Value);
#[cfg(feature = "bool")]
impl_to_matrix!(bool);
#[cfg(feature = "u8")]
impl_to_matrix!(u8);
#[cfg(feature = "u16")]
impl_to_matrix!(u16);
#[cfg(feature = "u32")]
impl_to_matrix!(u32);
#[cfg(feature = "u64")]
impl_to_matrix!(u64);
#[cfg(feature = "u128")]
impl_to_matrix!(u128);
#[cfg(feature = "i8")]
impl_to_matrix!(i8);
#[cfg(feature = "i16")]
impl_to_matrix!(i16);
#[cfg(feature = "i32")]
impl_to_matrix!(i32);
#[cfg(feature = "i64")]
impl_to_matrix!(i64);
#[cfg(feature = "i128")]
impl_to_matrix!(i128);
#[cfg(feature = "f32")]
impl_to_matrix!(F32);
#[cfg(feature = "f64")]
impl_to_matrix!(F64);
#[cfg(feature = "string")]
impl_to_matrix!(String);
#[cfg(feature = "complex")]
impl_to_matrix!(ComplexNumber);
#[cfg(feature = "rational")]
impl_to_matrix!(RationalNumber);
  
pub trait ToIndex: Clone {
  fn to_index(elements: Vec<Self>) -> Matrix<Self>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Matrix<T> {
  #[cfg(feature = "row_vector4")]
  RowVector4(Ref<RowVector4<T>>),
  #[cfg(feature = "row_vector3")]
  RowVector3(Ref<RowVector3<T>>),
  #[cfg(feature = "row_vector2")]
  RowVector2(Ref<RowVector2<T>>),
  #[cfg(feature = "vector4")]
  Vector4(Ref<Vector4<T>>),  
  #[cfg(feature = "vector3")]
  Vector3(Ref<Vector3<T>>),
  #[cfg(feature = "vector2")]
  Vector2(Ref<Vector2<T>>),
  #[cfg(feature = "matrix4")]
  Matrix4(Ref<Matrix4<T>>),
  #[cfg(feature = "matrix3")]
  Matrix3(Ref<Matrix3<T>>),
  #[cfg(feature = "matrix2")]
  Matrix2(Ref<Matrix2<T>>),
  #[cfg(feature = "matrix1")]
  Matrix1(Ref<Matrix1<T>>),
  #[cfg(feature = "matrix3x2")]
  Matrix3x2(Ref<Matrix3x2<T>>),
  #[cfg(feature = "matrix2x3")]
  Matrix2x3(Ref<Matrix2x3<T>>),
  #[cfg(feature = "vectord")]
  DVector(Ref<DVector<T>>),
  #[cfg(feature = "row_vectord")]
  RowDVector(Ref<RowDVector<T>>),
  #[cfg(feature = "matrixd")]
  DMatrix(Ref<DMatrix<T>>),
}

pub trait CopyMat<T> {
  #[cfg(feature = "matrixd")]
  fn copy_into(&self, dst: &Ref<DMatrix<T>>, offset: usize) -> usize;
  #[cfg(feature = "vectord")]
  fn copy_into_v(&self, dst: &Ref<DVector<T>>, offset: usize) -> usize;
  #[cfg(feature = "row_vectord")]
  fn copy_into_r(&self, dst: &Ref<RowDVector<T>>, offset: usize) -> usize;
  #[cfg(feature = "matrixd")]
  fn copy_into_row_major(&self, dst: &Ref<DMatrix<T>>, offset: usize) -> usize;
}

macro_rules! copy_mat {
  ($matsize:ident) => {
    impl<T> CopyMat<T> for Ref<$matsize<T>> 
    where T: Clone 
    {
      #[cfg(feature = "matrixd")]
      fn copy_into(&self, dst: &Ref<DMatrix<T>>, offset: usize) -> usize {
        let src_ptr = unsafe { (*(self.as_ptr())).clone() };
        let mut dst_ptr = unsafe { &mut *(dst.as_mut_ptr()) };
        for i in 0..src_ptr.len() {
          dst_ptr[i + offset] = src_ptr[i].clone();
        }
        src_ptr.len()
      }
      #[cfg(feature = "vectord")]
      fn copy_into_v(&self, dst: &Ref<DVector<T>>, offset: usize) -> usize {
        let src_ptr = unsafe { (*(self.as_ptr())).clone() };
        let mut dst_ptr = unsafe { &mut *(dst.as_mut_ptr()) };
        for i in 0..src_ptr.len() {
          dst_ptr[i + offset] = src_ptr[i].clone();
        }
        src_ptr.len()
      }
      #[cfg(feature = "row_vectord")]
      fn copy_into_r(&self, dst: &Ref<RowDVector<T>>, offset: usize) -> usize {
        let src_ptr = unsafe { (*(self.as_ptr())).clone() };
        let mut dst_ptr = unsafe { &mut *(dst.as_mut_ptr()) };
        for i in 0..src_ptr.len() {
          dst_ptr[i + offset] = src_ptr[i].clone();
        }
        src_ptr.len()
      }
      #[cfg(feature = "matrixd")]
      fn copy_into_row_major(&self, dst: &Ref<DMatrix<T>>, offset: usize) -> usize {
        let src_ptr = unsafe { (*(self.as_ptr())).clone() };
        let mut dst_ptr = unsafe { &mut *(dst.as_mut_ptr()) };
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
      
#[cfg(feature = "matrix1")]
copy_mat!(Matrix1);
#[cfg(feature = "matrix2")]
copy_mat!(Matrix2);
#[cfg(feature = "matrix3")]
copy_mat!(Matrix3);
#[cfg(feature = "matrix4")]
copy_mat!(Matrix4);
#[cfg(feature = "matrix2x3")]
copy_mat!(Matrix2x3);
#[cfg(feature = "matrix3x2")]
copy_mat!(Matrix3x2);
#[cfg(feature = "vector2")]
copy_mat!(Vector2);
#[cfg(feature = "vector3")]
copy_mat!(Vector3);
#[cfg(feature = "vector4")]
copy_mat!(Vector4);
#[cfg(feature = "row_vector2")]
copy_mat!(RowVector2);
#[cfg(feature = "row_vector3")]
copy_mat!(RowVector3);
#[cfg(feature = "row_vector4")]
copy_mat!(RowVector4);
#[cfg(feature = "vectord")]
copy_mat!(DVector);
#[cfg(feature = "matrixd")]
copy_mat!(DMatrix);
#[cfg(feature = "row_vectord")]
copy_mat!(RowDVector);

impl<T> Hash for Matrix<T> 
where T: Hash + nalgebra::Scalar
{
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self {
      #[cfg(feature = "row_vector4")]
      Matrix::RowVector4(x) => x.borrow().hash(state),
      #[cfg(feature = "row_vector3")]
      Matrix::RowVector3(x) => x.borrow().hash(state),
      #[cfg(feature = "row_vector2")]
      Matrix::RowVector2(x) => x.borrow().hash(state),
      #[cfg(feature = "vector4")]
      Matrix::Vector4(x) => x.borrow().hash(state),
      #[cfg(feature = "vector3")]
      Matrix::Vector3(x) => x.borrow().hash(state),
      #[cfg(feature = "vector2")]
      Matrix::Vector2(x) => x.borrow().hash(state),

      #[cfg(feature = "matrix4")]
      Matrix::Matrix4(x) => x.borrow().hash(state),
      #[cfg(feature = "matrix3")]
      Matrix::Matrix3(x) => x.borrow().hash(state),
      #[cfg(feature = "matrix2")]
      Matrix::Matrix2(x) => x.borrow().hash(state),
      #[cfg(feature = "matrix1")]
      Matrix::Matrix1(x) => x.borrow().hash(state),
      #[cfg(feature = "matrix3x2")]
      Matrix::Matrix3x2(x) => x.borrow().hash(state),
      #[cfg(feature = "matrix2x3")]
      Matrix::Matrix2x3(x) => x.borrow().hash(state),
      Matrix::DVector(x) => x.borrow().hash(state),
      Matrix::RowDVector(x) => x.borrow().hash(state),
      Matrix::DMatrix(x) => x.borrow().hash(state),
      _ => panic!("Hashing not implemented for this matrix type"),
    }
  }
}

#[cfg(feature = "pretty_print")]
impl<T> PrettyPrint for Matrix<T>
where T: Debug + Display + Clone + PartialEq + 'static + PrettyPrint
{
  fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    match self {
      #[cfg(feature = "row_vector4")]
      Matrix::RowVector4(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| x.pretty_print()).collect::<Vec<_>>()));}
      #[cfg(feature = "row_vector3")]
      Matrix::RowVector3(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| x.pretty_print()).collect::<Vec<_>>()));}
      #[cfg(feature = "row_vector2")]
      Matrix::RowVector2(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| x.pretty_print()).collect::<Vec<_>>()));}
      Matrix::RowDVector(vec) => {
        let vec_brrw = vec.borrow();
        let vec_str = if vec_brrw.ncols() > 20 {
          let mut vec_str = vec_brrw.row(0).iter().take(10).chain(vec_brrw.row(0).iter().rev().take(9).rev()).map(|x| x.pretty_print()).collect::<Vec<_>>();
          vec_str.insert(10,"...".to_string());
          vec_str
        } else {
          vec_brrw.row(0).iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>()
        };
        builder.push_record(vec_str);
      }
      #[cfg(feature = "vector4")]
      Matrix::Vector4(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| x.pretty_print()).collect::<Vec<_>>()));}
      #[cfg(feature = "vector3")]
      Matrix::Vector3(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| x.pretty_print()).collect::<Vec<_>>()));}
      #[cfg(feature = "vector2")]
      Matrix::Vector2(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| x.pretty_print()).collect::<Vec<_>>()));}
      Matrix::DVector(vec) => {
        let vec_brrw = vec.borrow();
        let vec_str = if vec_brrw.nrows() > 20 {
          let mut vec_str = vec_brrw.column(0).iter().take(10).chain(vec_brrw.column(0).iter().rev().take(9).rev()).map(|x| x.pretty_print()).collect::<Vec<_>>();
          vec_str.insert(10,"...".to_string());
          vec_str
        } else {
          vec_brrw.column(0).iter().map(|x| x.pretty_print()).collect::<Vec<_>>()
        };
        for r in vec_str {
          builder.push_record(vec![r]);
        }
      }
      #[cfg(feature = "matrix4")]
      Matrix::Matrix4(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| x.pretty_print()).collect::<Vec<_>>()));}
      #[cfg(feature = "matrix3")]
      Matrix::Matrix3(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| x.pretty_print()).collect::<Vec<_>>()));}
      #[cfg(feature = "matrix2")]
      Matrix::Matrix2(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| x.pretty_print()).collect::<Vec<_>>()));}
      #[cfg(feature = "matrix1")]
      Matrix::Matrix1(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| x.pretty_print()).collect::<Vec<_>>()));}
      #[cfg(feature = "matrix3x2")]
      Matrix::Matrix3x2(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| x.pretty_print()).collect::<Vec<_>>()));}
      #[cfg(feature = "matrix2x3")]
      Matrix::Matrix2x3(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| x.pretty_print()).collect::<Vec<_>>()));}
      Matrix::DMatrix(vec) => {let vec_brrw = vec.borrow();(0..vec_brrw.nrows()).for_each(|i| builder.push_record(vec_brrw.row(i).iter().map(|x| x.pretty_print()).collect::<Vec<_>>()));}
      _ => todo!(),
    };
    let matrix_style = Style::empty()
      .top(' ')
      .left('┃')
      .right('┃')
      .bottom(' ')
      .vertical(' ')
      .intersection_bottom(' ')
      .corner_top_left('┏')
      .corner_top_right('┓')
      .corner_bottom_left('┗')
      .corner_bottom_right('┛');
    let mut table = builder.build();
    table.with(matrix_style);
    format!("{table}")
  }
}

fn quoted<T: Display + Any>(val: &T) -> String {
  if let Some(s) = (val as &dyn Any).downcast_ref::<String>() {
    format!("<div class='mech-string'>\"{}\"</div>", s)
  } else if let Some(s) = (val as &dyn Any).downcast_ref::<bool>() {
    format!("<div class='mech-boolean'>{}</div<", s)
  } else {
    format!("<div class='mech-number'>{}</div>", val)
  }
}

impl<T> Matrix<T> 
where T: Debug + Display + Clone + PartialEq + 'static + PrettyPrint
{

  pub fn to_html(&self) -> String {
    let size = self.shape();
    let mut html = String::new();
    html.push_str("<table class='mech-matrix'>");
    for i in 0..size[0] {
      html.push_str("<tr>");
      for j in 0..size[1] {
        let value = self.index2d(i+1, j+1);
        html.push_str(&format!("<td>{}</td>", quoted(&value)));
      }
      html.push_str("</tr>");
    }
    format!("<div class='mech-matrix-outer'><div class='mech-matrix-inner'></div>{}</div>", html)
  }

}

impl<T> Matrix<T> 
where T: Debug + Clone + PartialEq + 'static
{

  pub fn append(&mut self, other: &Matrix<T>) -> MResult<()> {
    match (self, other) {
      (Matrix::DVector(lhs), Matrix::DVector(rhs)) => {
        let mut lhs = lhs.borrow_mut();
        let rhs = rhs.borrow();
        let old_len = lhs.len();
        lhs.resize_vertically_mut(old_len + rhs.len(), rhs[0].clone());
        for (i, val) in rhs.iter().enumerate() {
          lhs[old_len + i] = val.clone();
        }
        Ok(())
      }
      (Matrix::RowDVector(lhs), Matrix::RowDVector(rhs)) => {
        let mut lhs = lhs.borrow_mut();
        let rhs = rhs.borrow();
        let old_len = lhs.len();
        lhs.resize_horizontally_mut(old_len + rhs.len(), rhs[0].clone());
        for (i, val) in rhs.iter().enumerate() {
          lhs[old_len + i] = val.clone();
        }
        Ok(())
      }
      _ => {
        return Err(MechError{
          id: line!(),
          file: file!().to_string(),
          tokens: vec![],
          msg: "".to_string(),
          kind: MechErrorKind::None,
        });
      }    
    }
  }

  pub fn push(&mut self, value: T) -> MResult<()> {
    match self {
      Matrix::RowDVector(vec) => {
          let mut vec = vec.borrow_mut();
          let new_len = vec.ncols() + 1;
          vec.resize_horizontally_mut(new_len, value.clone()); // row vector: increase columns
          Ok(())
      }
      Matrix::DVector(vec) => {
          let mut vec = vec.borrow_mut();
          let new_len = vec.nrows() + 1;
          vec.resize_vertically_mut(new_len, value.clone()); // column vector: increase rows
          Ok(())
      }
      _ => {
        return Err(MechError{
          id: line!(),
          file: file!().to_string(),
          tokens: vec![],
          msg: "".to_string(),
          kind: MechErrorKind::None,
        });
      }
    }
  }

  pub fn rows(&self) -> usize {
    let size = self.shape();
    size[0]
  }

  pub fn cols(&self) -> usize {
    let size = self.shape();
    size[1]
  }

  pub fn size_of(&self) -> usize {
    let vec = self.as_vec();
    vec.capacity() * size_of::<T>()
  }       

  pub fn resize_vertically(&mut self, new_size: usize, fill_value: T) -> MResult<()> {
    match self {
      Matrix::RowDVector(vec) => {
        let mut vec = vec.borrow_mut();
        vec.resize_horizontally_mut(new_size, fill_value);
        Ok(())
      }
      Matrix::DVector(vec) => {
        let mut vec = vec.borrow_mut();
        vec.resize_vertically_mut(new_size, fill_value);
        Ok(())
      }
      _ => {
        return Err(MechError{
          id: line!(),
          file: file!().to_string(),
          tokens: vec![],
          msg: "".to_string(),
          kind: MechErrorKind::None,
        });
      }
    }
  }

  pub fn get_copyable_matrix(&self) -> Box<dyn CopyMat<T>> {
    match self {
      #[cfg(feature = "row_vector4")]
      Matrix::RowVector4(ref x) => Box::new(x.clone()),
      #[cfg(feature = "row_vector3")]
      Matrix::RowVector3(ref x) => Box::new(x.clone()),
      #[cfg(feature = "row_vector2")]
      Matrix::RowVector2(ref x) => Box::new(x.clone()),
      Matrix::RowDVector(ref x) => Box::new(x.clone()),
      #[cfg(feature = "vector4")]
      Matrix::Vector4(ref x) => Box::new(x.clone()),
      #[cfg(feature = "vector3")]
      Matrix::Vector3(ref x) => Box::new(x.clone()),
      #[cfg(feature = "vector2")]
      Matrix::Vector2(ref x) => Box::new(x.clone()),
      Matrix::DVector(ref x) => Box::new(x.clone()),
      #[cfg(feature = "matrix4")]
      Matrix::Matrix4(ref x) => Box::new(x.clone()),
      #[cfg(feature = "matrix3")]
      Matrix::Matrix3(ref x) => Box::new(x.clone()),
      #[cfg(feature = "matrix2")]
      Matrix::Matrix2(ref x) => Box::new(x.clone()),
      #[cfg(feature = "matrix1")]
      Matrix::Matrix1(ref x) => Box::new(x.clone()),
      #[cfg(feature = "matrix3x2")]
      Matrix::Matrix3x2(ref x) => Box::new(x.clone()),
      #[cfg(feature = "matrix2x3")]
      Matrix::Matrix2x3(ref x) => Box::new(x.clone()),
      Matrix::DMatrix(ref x) => Box::new(x.clone()),
      _ => panic!("Unsupported matrix size"),
    }
  }

  pub fn shape(&self) -> Vec<usize> {
    let shape = match self {
      #[cfg(feature = "row_vector4")]
      Matrix::RowVector4(x) => x.borrow().shape(),
      #[cfg(feature = "row_vector3")]
      Matrix::RowVector3(x) => x.borrow().shape(),
      #[cfg(feature = "row_vector2")]
      Matrix::RowVector2(x) => x.borrow().shape(),
      Matrix::RowDVector(x) => x.borrow().shape(),
      #[cfg(feature = "vector4")]
      Matrix::Vector4(x) => x.borrow().shape(),
      #[cfg(feature = "vector3")]
      Matrix::Vector3(x) => x.borrow().shape(),
      #[cfg(feature = "vector2")]
      Matrix::Vector2(x) => x.borrow().shape(),
      Matrix::DVector(x) => x.borrow().shape(),
      #[cfg(feature = "matrix4")]
      Matrix::Matrix4(x) => x.borrow().shape(),
      #[cfg(feature = "matrix3")]
      Matrix::Matrix3(x) => x.borrow().shape(),
      #[cfg(feature = "matrix2")]
      Matrix::Matrix2(x) => x.borrow().shape(),
      #[cfg(feature = "matrix1")]
      Matrix::Matrix1(x) => x.borrow().shape(),
      #[cfg(feature = "matrix3x2")]
      Matrix::Matrix3x2(x) => x.borrow().shape(),
      #[cfg(feature = "matrix2x3")]
      Matrix::Matrix2x3(x) => x.borrow().shape(),
      Matrix::DMatrix(x) => x.borrow().shape(),
      _ => panic!("Unsupported matrix size"),
    };
    vec![shape.0, shape.1]
  }

  pub fn index1d(&self, ix: usize) -> T {
    match self {
      #[cfg(feature = "row_vector4")]
      Matrix::RowVector4(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "row_vector3")]
      Matrix::RowVector3(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "row_vector2")]
      Matrix::RowVector2(x) => (*x.borrow().index(ix-1)).clone(),
      Matrix::RowDVector(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "vector4")]
      Matrix::Vector4(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "vector3")]
      Matrix::Vector3(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "vector2")]
      Matrix::Vector2(x) => (*x.borrow().index(ix-1)).clone(),
      Matrix::DVector(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "matrix4")]
      Matrix::Matrix4(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "matrix3")]
      Matrix::Matrix3(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "matrix2")]
      Matrix::Matrix2(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "matrix1")]
      Matrix::Matrix1(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "matrix3x2")]
      Matrix::Matrix3x2(x) => (*x.borrow().index(ix-1)).clone(),
      #[cfg(feature = "matrix2x3")]
      Matrix::Matrix2x3(x) => (*x.borrow().index(ix-1)).clone(),
      Matrix::DMatrix(x) => (*x.borrow().index(ix-1)).clone(),
      _ => panic!("Unsupported matrix size"),
    }
  }

  pub fn set_index1d(&self, index: usize, value: T) {
    match self {
      #[cfg(feature = "row_vector4")]
      Matrix::RowVector4(v) => v.borrow_mut()[index] = value,
      #[cfg(feature = "row_vector3")]
      Matrix::RowVector3(v) => v.borrow_mut()[index] = value,
      #[cfg(feature = "row_vector2")]
      Matrix::RowVector2(v) => v.borrow_mut()[index] = value,
      Matrix::RowDVector(v) => v.borrow_mut()[index] = value,
      #[cfg(feature = "vector4")]
      Matrix::Vector4(v) => v.borrow_mut()[index] = value,
      #[cfg(feature = "vector3")]
      Matrix::Vector3(v) => v.borrow_mut()[index] = value,
      #[cfg(feature = "vector2")]
      Matrix::Vector2(v) => v.borrow_mut()[index] = value,
      Matrix::DVector(v) => v.borrow_mut()[index] = value,
      #[cfg(feature = "matrix1")]
      Matrix::Matrix1(m) => m.borrow_mut()[index] = value,
      #[cfg(feature = "matrix2")]
      Matrix::Matrix2(m) => m.borrow_mut()[index] = value,
      #[cfg(feature = "matrix3")]
      Matrix::Matrix3(m) => m.borrow_mut()[index] = value,
      #[cfg(feature = "matrix4")]
      Matrix::Matrix4(m) => m.borrow_mut()[index] = value,
      #[cfg(feature = "matrix2x3")]
      Matrix::Matrix2x3(m) => m.borrow_mut()[index] = value,
      #[cfg(feature = "matrix3x2")]
      Matrix::Matrix3x2(m) => m.borrow_mut()[index] = value,
      Matrix::DMatrix(m) => m.borrow_mut()[index] = value,
      _ => panic!("Unsupported matrix size"),
    }
  }

  pub fn set(&self, elements: Vec<T>) {
    match self {
      #[cfg(feature = "row_vector4")]
      Matrix::RowVector4(x) => {
        let mut x = x.borrow_mut();
        x[0] = elements[0].clone();
        x[1] = elements[1].clone();
        x[2] = elements[2].clone();
        x[3] = elements[3].clone();
      }
      #[cfg(feature = "row_vector3")]
      Matrix::RowVector3(x) => {
        let mut x = x.borrow_mut();
        x[0] = elements[0].clone();
        x[1] = elements[1].clone();
        x[2] = elements[2].clone();
      }
      #[cfg(feature = "row_vector2")]
      Matrix::RowVector2(x) => {
        let mut x = x.borrow_mut();
        x[0] = elements[0].clone();
        x[1] = elements[1].clone();
      }
      Matrix::RowDVector(x) => {let mut x = x.borrow_mut();for i in 0..elements.len() {x[i] = elements[i].clone()}},
      #[cfg(feature = "vector4")]
      Matrix::Vector4(x) => {
        let mut x = x.borrow_mut();
        x[0] = elements[0].clone();
        x[1] = elements[1].clone();
        x[2] = elements[2].clone();
        x[3] = elements[3].clone();
      }
      #[cfg(feature = "vector3")]
      Matrix::Vector3(x) => {
        let mut x = x.borrow_mut();
        x[0] = elements[0].clone();
        x[1] = elements[1].clone();
        x[2] = elements[2].clone();
      }
      #[cfg(feature = "vector2")]
      Matrix::Vector2(x) => {
        let mut x = x.borrow_mut();
        x[0] = elements[0].clone();
        x[1] = elements[1].clone();
      }
      Matrix::DVector(x) => {let mut x = x.borrow_mut();for i in 0..elements.len() {x[i] = elements[i].clone()}},
      #[cfg(feature = "matrix4")]
      Matrix::Matrix4(x) => {
        let mut x = x.borrow_mut();
        x[0] = elements[0].clone();
        x[1] = elements[1].clone();
        x[2] = elements[2].clone();
        x[3] = elements[3].clone();
        x[4] = elements[4].clone();
        x[5] = elements[5].clone();
        x[6] = elements[6].clone();
        x[7] = elements[7].clone();
        x[8] = elements[8].clone();
        x[9] = elements[9].clone();
        x[10] = elements[10].clone();
        x[11] = elements[11].clone();
        x[12] = elements[12].clone();
        x[13] = elements[13].clone();
        x[14] = elements[14].clone();
        x[15] = elements[15].clone();
      }
      #[cfg(feature = "matrix3")]
      Matrix::Matrix3(x) => {
        let mut x = x.borrow_mut();
        x[0] = elements[0].clone();
        x[1] = elements[1].clone();
        x[2] = elements[2].clone();
        x[3] = elements[3].clone();
        x[4] = elements[4].clone();
        x[5] = elements[5].clone();
        x[6] = elements[6].clone();
        x[7] = elements[7].clone();
        x[8] = elements[8].clone();
      }
      #[cfg(feature = "matrix2")]
      Matrix::Matrix2(x) => {
        let mut x = x.borrow_mut();
        x[0] = elements[0].clone();
        x[1] = elements[1].clone();
        x[2] = elements[2].clone();
        x[3] = elements[3].clone();
      }
      #[cfg(feature = "matrix1")]
      Matrix::Matrix1(x) => {let mut x = x.borrow_mut();x[0] = elements[0].clone();},
      #[cfg(feature = "matrix3x2")]
      Matrix::Matrix3x2(x) => {
        let mut x = x.borrow_mut();
        x[0] = elements[0].clone();
        x[1] = elements[1].clone();
        x[2] = elements[2].clone();
        x[3] = elements[3].clone();
        x[4] = elements[4].clone();
        x[5] = elements[5].clone();
      }
      #[cfg(feature = "matrix2x3")]
      Matrix::Matrix2x3(x) => {
        let mut x = x.borrow_mut();
        x[0] = elements[0].clone();
        x[1] = elements[1].clone();
        x[2] = elements[2].clone();
        x[3] = elements[3].clone();
        x[4] = elements[4].clone();
        x[5] = elements[5].clone();
      }
      Matrix::DMatrix(x) => {let mut x = x.borrow_mut();for i in 0..elements.len() {x[i] = elements[i].clone()}},
      _ => panic!("Unsupported matrix size"),
    }
  }

  pub fn index2d(&self, row: usize, col: usize) -> T {
    match self {
      #[cfg(feature = "row_vector4")]
      Matrix::RowVector4(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "row_vector3")]
      Matrix::RowVector3(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "row_vector2")]
      Matrix::RowVector2(x) => (*x.borrow().index((row-1,col-1))).clone(),
      Matrix::RowDVector(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "vector4")]
      Matrix::Vector4(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "vector3")]
      Matrix::Vector3(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "vector2")]
      Matrix::Vector2(x) => (*x.borrow().index((row-1,col-1))).clone(),
      Matrix::DVector(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "matrix4")]
      Matrix::Matrix4(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "matrix3")]
      Matrix::Matrix3(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "matrix2")]
      Matrix::Matrix2(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "matrix1")]
      Matrix::Matrix1(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "matrix3x2")]
      Matrix::Matrix3x2(x) => (*x.borrow().index((row-1,col-1))).clone(),
      #[cfg(feature = "matrix2x3")]
      Matrix::Matrix2x3(x) => (*x.borrow().index((row-1,col-1))).clone(),
      Matrix::DMatrix(x) => (*x.borrow().index((row-1,col-1))).clone(),
      _ => panic!("Unsupported matrix type for as_vec"),
    }
  }

  pub fn as_vec(&self) -> Vec<T> {
    match self {
      #[cfg(feature = "row_vector4")]
      Matrix::RowVector4(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "row_vector3")]
      Matrix::RowVector3(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "row_vector2")]
      Matrix::RowVector2(x) => x.borrow().as_slice().to_vec(),
      Matrix::RowDVector(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "vector4")]
      Matrix::Vector4(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "vector3")]
      Matrix::Vector3(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "vector2")]
      Matrix::Vector2(x) => x.borrow().as_slice().to_vec(),
      Matrix::DVector(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "matrix4")]
      Matrix::Matrix4(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "matrix3")]
      Matrix::Matrix3(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "matrix2")]
      Matrix::Matrix2(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "matrix1")]
      Matrix::Matrix1(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "matrix3x2")]
      Matrix::Matrix3x2(x) => x.borrow().as_slice().to_vec(),
      #[cfg(feature = "matrix2x3")]
      Matrix::Matrix2x3(x) => x.borrow().as_slice().to_vec(),
      Matrix::DMatrix(x) => x.borrow().as_slice().to_vec(),
      _ => panic!("Unsupported matrix type for as_vec"),
    }
  }

}

macro_rules! impl_to_value_for_matrix {
  ($t:ty, $variant:ident) => {
    impl ToValue for Matrix<$t> {
      fn to_value(&self) -> Value {
        Value::$variant(self.clone())
      }
    }
  };
}

impl_to_value_for_matrix!(Value, MatrixValue);
#[cfg(feature = "f64")]
impl_to_value_for_matrix!(F64, MatrixF64);
#[cfg(feature = "f32")]
impl_to_value_for_matrix!(F32, MatrixF32);
#[cfg(feature = "i8")]
impl_to_value_for_matrix!(i8, MatrixI8);
#[cfg(feature = "i16")]
impl_to_value_for_matrix!(i16, MatrixI16);
#[cfg(feature = "i32")]
impl_to_value_for_matrix!(i32, MatrixI32);
#[cfg(feature = "i64")]
impl_to_value_for_matrix!(i64, MatrixI64);
#[cfg(feature = "i128")]
impl_to_value_for_matrix!(i128, MatrixI128);
#[cfg(feature = "u8")]
impl_to_value_for_matrix!(u8, MatrixU8);
#[cfg(feature = "u16")]
impl_to_value_for_matrix!(u16, MatrixU16);
#[cfg(feature = "u32")]
impl_to_value_for_matrix!(u32, MatrixU32);
#[cfg(feature = "u64")]
impl_to_value_for_matrix!(u64, MatrixU64);
#[cfg(feature = "u128")]
impl_to_value_for_matrix!(u128, MatrixU128);
#[cfg(feature = "bool")]
impl_to_value_for_matrix!(bool, MatrixBool);
#[cfg(feature = "string")]
impl_to_value_for_matrix!(String, MatrixString);
#[cfg(feature = "complex")]
impl_to_value_for_matrix!(ComplexNumber, MatrixComplexNumber);
#[cfg(feature = "rational")]
impl_to_value_for_matrix!(RationalNumber, MatrixRationalNumber);


macro_rules! to_value_ndmatrix {
  ($($nd_matrix_kind:ident, $matrix_kind:ident, $base_type:ty, $type_string:tt),+ $(,)?) => {
    $(
      #[cfg(all(feature = "matrix", feature = $type_string))]
      impl ToValue for Ref<$nd_matrix_kind<$base_type>> {
        fn to_value(&self) -> Value {
          Value::$matrix_kind(Matrix::<$base_type>::$nd_matrix_kind(self.clone()))
        }
      }
    )+
  };}

macro_rules! impl_to_value_matrix {
  ($matrix_kind:ident) => {
    to_value_ndmatrix!(
      $matrix_kind, MatrixIndex,  usize, "matrix",
      $matrix_kind, MatrixBool,   bool, "bool",
      $matrix_kind, MatrixI8,     i8, "i8",
      $matrix_kind, MatrixI16,    i16, "i16",
      $matrix_kind, MatrixI32,    i32, "i32",
      $matrix_kind, MatrixI64,    i64, "i64",
      $matrix_kind, MatrixI128,   i128, "i128",
      $matrix_kind, MatrixU8,     u8, "u8",
      $matrix_kind, MatrixU16,    u16, "u16",
      $matrix_kind, MatrixU32,    u32, "u32",
      $matrix_kind, MatrixU64,    u64, "u64",
      $matrix_kind, MatrixU128,   u128, "u128",
      $matrix_kind, MatrixF32,    F32, "f32",
      $matrix_kind, MatrixF64,    F64, "f64",
      $matrix_kind, MatrixString, String, "string",
      $matrix_kind, MatrixRationalNumber, RationalNumber, "rational",
      $matrix_kind, MatrixComplexNumber, ComplexNumber, "complex",
    );
  }
}

#[cfg(feature = "matrix2x3")]
impl_to_value_matrix!(Matrix2x3);
#[cfg(feature = "matrix3x2")]
impl_to_value_matrix!(Matrix3x2);
#[cfg(feature = "matrix1")]
impl_to_value_matrix!(Matrix1);
#[cfg(feature = "matrix2")]
impl_to_value_matrix!(Matrix2);
#[cfg(feature = "matrix3")]
impl_to_value_matrix!(Matrix3);
#[cfg(feature = "matrix4")]
impl_to_value_matrix!(Matrix4);
#[cfg(feature = "vector2")]
impl_to_value_matrix!(Vector2);
#[cfg(feature = "vector3")]
impl_to_value_matrix!(Vector3);
#[cfg(feature = "vector4")]
impl_to_value_matrix!(Vector4);
#[cfg(feature = "row_vector2")]
impl_to_value_matrix!(RowVector2);
#[cfg(feature = "row_vector3")]
impl_to_value_matrix!(RowVector3);
#[cfg(feature = "row_vector4")]
impl_to_value_matrix!(RowVector4);
#[cfg(feature = "row_vectord")]
impl_to_value_matrix!(RowDVector);
#[cfg(feature = "vectord")]
impl_to_value_matrix!(DVector);
#[cfg(feature = "matrixd")]
impl_to_value_matrix!(DMatrix);