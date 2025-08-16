#[macro_use]
use crate::stdlib::*;

// Vertical Concatenate -----------------------------------------------------

macro_rules! vertcat_one_arg {
  ($fxn:ident, $e0:ident, $out:ident, $opt:ident) => {
    #[derive(Debug)]
    struct $fxn<T> {
      e0: Ref<$e0<T>>,
      out: Ref<$out<T>>,
    }
    impl<T> MechFunction for $fxn<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$out<T>>: ToValue
    {
      fn solve(&self) { 
        unsafe {
          let e0_ptr = (*(self.e0.as_ptr())).clone();
          let mut out_ptr = (&mut *(self.out.as_ptr()));
          $opt!(out_ptr,e0_ptr);
        }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
  };}

macro_rules! vertcat_two_args {
  ($fxn:ident, $e1:ident, $e2:ident, $out:ident, $opt:ident) => {
    #[derive(Debug)]
    struct $fxn<T> {
      e0: Ref<$e1<T>>,
      e1: Ref<$e2<T>>,
      out: Ref<$out<T>>,
    }
    impl<T> MechFunction for $fxn<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$out<T>>: ToValue
    {
      fn solve(&self) { 
        unsafe {
          let e0_ptr = (*(self.e0.as_ptr())).clone();
          let e1_ptr = (*(self.e1.as_ptr())).clone();
          let mut out_ptr = (&mut *(self.out.as_ptr()));
          $opt!(out_ptr,e0_ptr,e1_ptr);
        }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
  };}

macro_rules! vertcat_three_args {
  ($fxn:ident, $e0:ident, $e1:ident, $e2:ident, $out:ident, $opt:ident) => {
    #[derive(Debug)]
    struct $fxn<T> {
      e0: Ref<$e0<T>>,
      e1: Ref<$e1<T>>,
      e2: Ref<$e2<T>>,
      out: Ref<$out<T>>,
    }
    impl<T> MechFunction for $fxn<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$out<T>>: ToValue
    {
      fn solve(&self) { 
        unsafe {
          let e0_ptr = (*(self.e0.as_ptr())).clone();
          let e1_ptr = (*(self.e1.as_ptr())).clone();
          let e2_ptr = (*(self.e2.as_ptr())).clone();
          let mut out_ptr = (&mut *(self.out.as_ptr()));
          $opt!(out_ptr,e0_ptr,e1_ptr,e2_ptr);
        }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
  };} 
  
macro_rules! vertcat_four_args {
  ($fxn:ident, $e0:ident, $e1:ident, $e2:ident, $e3:ident, $out:ident, $opt:ident) => {
    #[derive(Debug)]
    struct $fxn<T> {
      e0: Ref<$e0<T>>,
      e1: Ref<$e1<T>>,
      e2: Ref<$e2<T>>,
      e3: Ref<$e3<T>>,
      out: Ref<$out<T>>,
    }
    impl<T> MechFunction for $fxn<T>
    where
      T: Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$out<T>>: ToValue
    {
      fn solve(&self) { 
        unsafe {
          let e0_ptr = (*(self.e0.as_ptr())).clone();
          let e1_ptr = (*(self.e1.as_ptr())).clone();
          let e2_ptr = (*(self.e2.as_ptr())).clone();
          let e3_ptr = (*(self.e3.as_ptr())).clone();
          let mut out_ptr = (&mut *(self.out.as_ptr()));
          $opt!(out_ptr,e0_ptr,e1_ptr,e2_ptr,e3_ptr);
        }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
  };}   

struct VerticalConcatenateTwoArgs<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  out: Ref<DMatrix<T>>,
}
impl<T> MechFunction for VerticalConcatenateTwoArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    let offset = self.e0.copy_into_row_major(&self.out,0);
    self.e1.copy_into_row_major(&self.out,offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateTwoArgs\n{:#?}", self.out) }
}
    
struct VerticalConcatenateThreeArgs<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  e2: Box<dyn CopyMat<T>>,
  out: Ref<DMatrix<T>>,
}
impl<T> MechFunction for VerticalConcatenateThreeArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    let mut offset = self.e0.copy_into_row_major(&self.out,0);
    offset += self.e1.copy_into_row_major(&self.out,offset);
    self.e2.copy_into_row_major(&self.out,offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateThreeArgs\n{:#?}", self.out) }
}

struct VerticalConcatenateFourArgs<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  e2: Box<dyn CopyMat<T>>,
  e3: Box<dyn CopyMat<T>>,
  out: Ref<DMatrix<T>>,
}
impl<T> MechFunction for VerticalConcatenateFourArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    let mut offset = self.e0.copy_into_row_major(&self.out,0);
    offset += self.e1.copy_into_row_major(&self.out,offset);
    offset += self.e2.copy_into_row_major(&self.out,offset);
    self.e3.copy_into_row_major(&self.out,offset);

  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateFourArgs\n{:#?}", self.out) }
}

struct VerticalConcatenateNArgs<T> {
  e0: Vec<Box<dyn CopyMat<T>>>,
  out: Ref<DMatrix<T>>,
}
impl<T> MechFunction for VerticalConcatenateNArgs<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    let mut offset = 0;
    for e in &self.e0 {
      offset += e.copy_into_row_major(&self.out,offset);
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateNArgs\n{:#?}", self.out) }
}

macro_rules! vertical_concatenate {
  ($name:ident, $vec_size:expr) => {
    paste!{
      #[derive(Debug)]
      struct $name<T> {
        out: Ref<[<$vec_size>]<T>>,
      }

      impl<T> MechFunction for $name<T> 
      where
        T: Debug + Clone + Sync + Send + PartialEq + 'static,
        Ref<[<$vec_size>]<T>>: ToValue
      {
        fn solve(&self) {}
        fn out(&self) -> Value { self.out.to_value() }
        fn to_string(&self) -> String { format!("{:#?}", self) }
      }
    }
  };}  

struct VerticalConcatenateVD2<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  out: Ref<DVector<T>>,
}

impl<T> MechFunction for VerticalConcatenateVD2<T> 
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
{
  fn solve(&self) {   
    let mut offset = self.e0.copy_into_v(&self.out,0);
    self.e1.copy_into_v(&self.out,offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateVD2\n{:#?}", self.out) }
}

struct VerticalConcatenateVD3<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  e2: Box<dyn CopyMat<T>>,
  out: Ref<DVector<T>>,
}

impl<T> MechFunction for VerticalConcatenateVD3<T> 
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
{
  fn solve(&self) {   
    let mut offset = self.e0.copy_into_v(&self.out,0);
    offset += self.e1.copy_into_v(&self.out,offset);
    self.e2.copy_into_v(&self.out,offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateVD3\n{:#?}", self.out) }
}

struct VerticalConcatenateVD4<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  e2: Box<dyn CopyMat<T>>,
  e3: Box<dyn CopyMat<T>>,
  out: Ref<DVector<T>>,
}

impl<T> MechFunction for VerticalConcatenateVD4<T> 
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
{
  fn solve(&self) {   
    let mut offset = self.e0.copy_into_v(&self.out,0);
    offset += self.e1.copy_into_v(&self.out,offset);
    offset += self.e2.copy_into_v(&self.out,offset);
    self.e3.copy_into_v(&self.out,offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateVD3\n{:#?}", self.out) }
}

struct VerticalConcatenateVDN<T> {
  scalar: Vec<(Ref<T>,usize)>,
  matrix: Vec<(Box<dyn CopyMat<T>>,usize)>,
  out: Ref<DVector<T>>,
}

impl<T> MechFunction for VerticalConcatenateVDN<T> 
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      for (e,i) in &self.matrix {
        e.copy_into_v(&self.out,*i);
      }
      for (e,i) in &self.scalar {
        out_ptr[*i] = e.borrow().clone();
      }
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateVDN\n{:#?}", self.out) }
}

#[cfg(feature = "matrix1")]
#[derive(Debug)]
struct VerticalConcatenateS1<T> {
  out: Ref<Matrix1<T>>,
}

#[cfg(feature = "matrix1")]
impl<T> MechFunction for VerticalConcatenateS1<T> 
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<Matrix1<T>>: ToValue
{
  fn solve(&self) {}
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

#[cfg(feature = "vector2")]
vertical_concatenate!(VerticalConcatenateS2,Vector2);
#[cfg(feature = "vector3")]
vertical_concatenate!(VerticalConcatenateS3,Vector3);
#[cfg(feature = "vector4")]
vertical_concatenate!(VerticalConcatenateS4,Vector4);
#[cfg(feature = "vector2")]
vertical_concatenate!(VerticalConcatenateV2,Vector2);
#[cfg(feature = "vector3")]
vertical_concatenate!(VerticalConcatenateV3,Vector3);
#[cfg(feature = "vector4")]
vertical_concatenate!(VerticalConcatenateV4,Vector4);
#[cfg(feature = "matrix2")]
vertical_concatenate!(VerticalConcatenateM2,Matrix2);
#[cfg(feature = "matrix3")]
vertical_concatenate!(VerticalConcatenateM3,Matrix3);
#[cfg(feature = "matrix2x3")]
vertical_concatenate!(VerticalConcatenateM2x3,Matrix2x3);
#[cfg(feature = "matrix3x2")]
vertical_concatenate!(VerticalConcatenateM3x2,Matrix3x2);
#[cfg(feature = "matrix4")]
vertical_concatenate!(VerticalConcatenateM4,Matrix4);
#[cfg(feature = "matrixd")]
vertical_concatenate!(VerticalConcatenateMD,DMatrix);
#[cfg(feature = "vectord")]
vertical_concatenate!(VerticalConcatenateVD,DVector);

#[cfg(feature = "vectord")]
#[derive(Debug)]
struct VerticalConcatenateSD<T> {
  out: Ref<DVector<T>>,
}
#[cfg(feature = "vectord")]
impl<T> MechFunction for VerticalConcatenateSD<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
{
  fn solve(&self) { }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

macro_rules! vertcat_m1m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
  };}
#[cfg(feature = "matrix1")]
vertcat_two_args!(VerticalConcatenateM1M1,Matrix1,Matrix1,Vector2,vertcat_m1m1);

macro_rules! vertcat_r2r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };}
#[cfg(feature = "vector2")]
vertcat_two_args!(VerticalConcatenateV2V2,Vector2,Vector2,Vector4,vertcat_r2r2);

macro_rules! vertcat_m1r3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
    $out[3] = $e1[2].clone();
  };}
#[cfg(all(feature = "matrix1", feature = "vector3", feature = "vector4"))]
vertcat_two_args!(VerticalConcatenateM1V3,Matrix1,Vector3,Vector4,vertcat_m1r3);

macro_rules! vertcat_r3m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e1[0].clone();
  };}
#[cfg(all(feature = "vector3", feature = "matrix1", feature = "vector4"))]
vertcat_two_args!(VerticalConcatenateV3M1,Vector3,Matrix1,Vector4,vertcat_r3m1);

macro_rules! vertcat_m1r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
  };
}
#[cfg(all(feature = "matrix1", feature = "vector2", feature = "vector3"))]
vertcat_two_args!(VerticalConcatenateM1V2, Matrix1, Vector2, Vector3, vertcat_m1r2);

macro_rules! vertcat_r2m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
  };
}
#[cfg(all(feature = "vector2", feature = "matrix1", feature = "vector3"))]
vertcat_two_args!(VerticalConcatenateV2M1, Vector2, Matrix1, Vector3, vertcat_r2m1);

macro_rules! vertcat_m1m1m1 {
  ($out:expr, $e0:expr,$e1:expr,$e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e2[0].clone();
  };
}
#[cfg(all(feature = "matrix1", feature = "vector3"))]
vertcat_three_args!(VerticalConcatenateM1M1M1,Matrix1,Matrix1,Matrix1,Vector3, vertcat_m1m1m1);

macro_rules! vertcat_m1m1r2 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e2[0].clone();
    $out[3] = $e2[1].clone();
  };
}
#[cfg(all(feature = "matrix1", feature = "vector2", feature = "vector4"))]
vertcat_three_args!(VerticalConcatenateM1M1V2, Matrix1, Matrix1, Vector2, Vector4, vertcat_m1m1r2);

macro_rules! vertcat_m1r2m1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
    $out[3] = $e2[0].clone();
  };
}
#[cfg(all(feature = "matrix1", feature = "vector2", feature = "vector4"))]
vertcat_three_args!(VerticalConcatenateM1V2M1, Matrix1, Vector2, Matrix1, Vector4, vertcat_m1r2m1);

macro_rules! vertcat_r2m1m1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e2[0].clone();
  };
}
#[cfg(all(feature = "vector2", feature = "matrix1", feature = "vector4"))]
vertcat_three_args!(VerticalConcatenateV2M1M1, Vector2, Matrix1, Matrix1, Vector4, vertcat_r2m1m1);

#[derive(Debug)]
struct VerticalConcatenateM1M1M1M1<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<Matrix1<T>>,
  e3: Ref<Matrix1<T>>,
  out: Ref<Vector4<T>>,
}
impl<T> MechFunction for VerticalConcatenateM1M1M1M1<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<Vector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let e3_ptr = (*(self.e3.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_ptr[0].clone();
      out_ptr[3] = e3_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

macro_rules! vertcat_r2r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[2] = $e0[1].clone();
    $out[1] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };
}
#[cfg(all(feature = "vector2", feature = "matrix2"))]
vertcat_two_args!(VerticalConcatenateR2R2, RowVector2, RowVector2, Matrix2, vertcat_r2r2);

macro_rules! vertcat_r3r3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[2] = $e0[1].clone();
    $out[4] = $e0[2].clone();
    $out[1] = $e1[0].clone();
    $out[3] = $e1[1].clone();
    $out[5] = $e1[2].clone();
  };
}
#[cfg(all(feature = "row_vector3", feature = "matrix2x3"))]
vertcat_two_args!(VerticalConcatenateR3R3, RowVector3, RowVector3, Matrix2x3, vertcat_r3r3);

macro_rules! vertcat_r2m2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[3] = $e0[1].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
    $out[4] = $e1[2].clone();
    $out[5] = $e1[3].clone();
  };
}
#[cfg(all(feature = "row_vector2", feature = "matrix2", feature = "matrix3x2"))]
vertcat_two_args!(VerticalConcatenateR2M2, RowVector2, Matrix2, Matrix3x2, vertcat_r2m2);

macro_rules! vertcat_m2r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[3] = $e0[2].clone();
    $out[4] = $e0[3].clone();
    $out[2] = $e1[0].clone();
    $out[5] = $e1[1].clone();
  };
}
#[cfg(all(feature = "matrix2", feature = "row_vector2", feature = "matrix3x2"))]
vertcat_two_args!(VerticalConcatenateM2R2, Matrix2, RowVector2, Matrix3x2, vertcat_m2r2);

macro_rules! vertcat_m2x3r3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[3] = $e0[2].clone();
    $out[4] = $e0[3].clone();
    $out[6] = $e0[4].clone();
    $out[7] = $e0[5].clone();
    $out[2] = $e1[0].clone();
    $out[5] = $e1[1].clone();
    $out[8] = $e1[2].clone();
  };
}
#[cfg(all(feature = "matrix2x3", feature = "row_vector3", feature = "matrix3"))]
vertcat_two_args!(VerticalConcatenateM2x3R3, Matrix2x3, RowVector3, Matrix3, vertcat_m2x3r3);

macro_rules! vertcat_r3m2x3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[3] = $e0[1].clone();
    $out[6] = $e0[2].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
    $out[4] = $e1[2].clone();
    $out[5] = $e1[3].clone();
    $out[7] = $e1[4].clone();
    $out[8] = $e1[5].clone();
  };
}
#[cfg(all(feature = "row_vector3", feature = "matrix2x3", feature = "matrix3"))]
vertcat_two_args!(VerticalConcatenateR3M2x3, RowVector3, Matrix2x3, Matrix3, vertcat_r3m2x3);


macro_rules! vertcat_mdv4 {
  ($out:expr, $e0:expr, $e1:expr) => {
    let e0_len = $e0.len();
    for i in 0..e0_len {
      $out[i] = $e0[i].clone();
    }
    let offset = e0_len;
    $out[offset] = $e1[0].clone();
    $out[offset + 1] = $e1[1].clone();
    $out[offset + 2] = $e1[2].clone();
    $out[offset + 3] = $e1[3].clone();
  };
}
#[cfg(all(feature = "matrixd", feature = "row_vector4", feature = "matrix4"))]
vertcat_two_args!(VerticalConcatenateMDR4, DMatrix, RowVector4, Matrix4, vertcat_mdv4);

macro_rules! vertcat_mdmd {
  ($out:expr, $e0:expr, $e1:expr) => {
    let dest_rows = $out.nrows();
    let mut offset = 0;
    let mut dest_ix = 0;

    let src_rows = $e0.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e0.len() {
      $out[dest_ix] = $e0[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
    offset += src_rows;

    let src_rows = $e1.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e1.len() {
      $out[dest_ix] = $e1[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
  };
}
#[cfg(all(feature = "matrixd", feature = "matrix4"))]
vertcat_two_args!(VerticalConcatenateMDMD, DMatrix, DMatrix, Matrix4, vertcat_mdmd);
#[cfg(all(feature = "matrixd", feature = "matrix4", feature = "row_vector4"))]
vertcat_two_args!(VerticalConcatenateR4MD, RowVector4, DMatrix, Matrix4, vertcat_mdmd);


macro_rules! vertcat_mdmdmd {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    let dest_rows = $out.nrows();
    let mut offset = 0;
    let mut dest_ix = 0;

    let src_rows = $e0.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e0.len() {
      $out[dest_ix] = $e0[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
    offset += src_rows;

    let src_rows = $e1.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e1.len() {
      $out[dest_ix] = $e1[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
    offset += src_rows;

    let src_rows = $e2.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e2.len() {
      $out[dest_ix] = $e2[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
  };
}

#[cfg(all(feature = "matrixd", feature = "matrix4", feature = "row_vector4"))]
vertcat_three_args!(VerticalConcatenateR2R2R2, RowVector2, RowVector2, RowVector2, Matrix3x2, vertcat_mdmdmd);
#[cfg(all(feature = "row_vector3", feature = "row_vector3", feature = "row_vector3", feature = "matrix3"))]
vertcat_three_args!(VerticalConcatenateR3R3R3, RowVector3, RowVector3, RowVector3, Matrix3, vertcat_mdmdmd);
#[cfg(all(feature = "row_vector4", feature = "row_vector4", feature = "matrixd", feature = "matrix4"))]
vertcat_three_args!(VerticalConcatenateR4R4MD, RowVector4, RowVector4, DMatrix, Matrix4, vertcat_mdmdmd);
#[cfg(all(feature = "row_vector4", feature = "matrixd", feature = "row_vector4", feature = "matrix4"))]
vertcat_three_args!(VerticalConcatenateR4MDR4, RowVector4, DMatrix, RowVector4, Matrix4, vertcat_mdmdmd);
#[cfg(all(feature = "matrixd", feature = "row_vector4", feature = "row_vector4", feature = "matrix4"))]
vertcat_three_args!(VerticalConcatenateMDR4R4, DMatrix, RowVector4, RowVector4, Matrix4, vertcat_mdmdmd);

macro_rules! vertcat_mdmdmdmd {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr, $e3:expr) => {
    let dest_rows = $out.nrows();
    let mut offset = 0;
    let mut dest_ix = 0;

    let src_rows = $e0.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e0.len() {
      $out[dest_ix] = $e0[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
    offset += src_rows;

    let src_rows = $e1.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e1.len() {
      $out[dest_ix] = $e1[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
    offset += src_rows;

    let src_rows = $e2.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e2.len() {
      $out[dest_ix] = $e2[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
    offset += src_rows;

    let src_rows = $e3.nrows();
    let stride = dest_rows - src_rows;
    dest_ix = offset;
    for ix in 0..$e3.len() {
      $out[dest_ix] = $e3[ix].clone();
      dest_ix += ((ix + 1) % src_rows == 0) as usize * stride + 1;
    }
  };
}

#[cfg(all(feature = "matrix4", feature = "row_vector4"))]
vertcat_four_args!(VerticalConcatenateR4R4R4R4, RowVector4, RowVector4, RowVector4, RowVector4, Matrix4, vertcat_mdmdmdmd);

macro_rules! impl_vertcat_arms {
  ($kind:ident, $args:expr, $default:expr) => {
    paste!{
    {
      let arguments = $args;  
      let rows = arguments[0].shape()[0];
      let rows:usize = arguments.iter().fold(0, |acc, x| acc + x.shape()[0]);
      let columns:usize = arguments[0].shape()[1];
      let nargs = arguments.len();
      let kinds: Vec<ValueKind> = arguments.iter().map(|x| x.kind()).collect::<Vec<ValueKind>>();
      let no_refs = !kinds.iter().any(|x| {
        match x {
          ValueKind::Reference(_) => true,
          ValueKind::Matrix(_,_) => true,
          _ => false,
      }});
      if no_refs {
        let mat: Vec<$kind> = arguments.iter().flat_map(|v| v.[<as_vec $kind:lower>]().unwrap()).collect::<Vec<$kind>>();
        fn to_column_major<T: Clone>(out: &[Value], row_n: usize, col_n: usize, extract_fn: impl Fn(&Value) -> Option<Vec<T>> + Clone) -> Vec<T> {
          (0..col_n).flat_map(|col| out.iter().map({let value = extract_fn.clone();move |row| value(row).unwrap()[col].clone()})).collect()
        }
        let mat = to_column_major(&arguments, rows, columns, |v| v.[<as_vec $kind:lower>]());
        match (rows,columns) {
          #[cfg(feature = "matrix1")]
          (1,1) => {return Ok(Box::new(VerticalConcatenateS1{out:new_ref(Matrix1::from_vec(mat))}));}
          #[cfg(feature = "vector2")]
          (2,1) => {return Ok(Box::new(VerticalConcatenateS2{out:new_ref(Vector2::from_vec(mat))}));}
          #[cfg(feature = "vector3")]
          (3,1) => {return Ok(Box::new(VerticalConcatenateS3{out:new_ref(Vector3::from_vec(mat))}));}
          #[cfg(feature = "vector4")]
          (4,1) => {return Ok(Box::new(VerticalConcatenateS4{out:new_ref(Vector4::from_vec(mat))}));}
          #[cfg(feature = "vectord")]
          (m,1) => {return Ok(Box::new(VerticalConcatenateSD{out:new_ref(DVector::from_vec(mat))}));}
          #[cfg(feature = "matrix2")]
          (2,2) => {return Ok(Box::new(VerticalConcatenateM2{out:new_ref(Matrix2::from_vec(mat))}));}
          #[cfg(feature = "matrix3")]
          (3,3) => {return Ok(Box::new(VerticalConcatenateM3{out:new_ref(Matrix3::from_vec(mat))}));}
          #[cfg(feature = "matrix4")]
          (4,4) => {return Ok(Box::new(VerticalConcatenateM4{out:new_ref(Matrix4::from_vec(mat))}));}
          #[cfg(feature = "matrix2x3")]
          (2,3) => {return Ok(Box::new(VerticalConcatenateM2x3{out:new_ref(Matrix2x3::from_vec(mat))}));}
          #[cfg(feature = "matrix3x2")]
          (3,2) => {return Ok(Box::new(VerticalConcatenateM3x2{out:new_ref(Matrix3x2::from_vec(mat))}));}
          #[cfg(feature = "matrixd")]
          (m,n) => {return Ok(Box::new(VerticalConcatenateMD{out:new_ref(DMatrix::from_vec(m,n,mat))}));}
        }
      } else {
        match (nargs,rows,columns) {
          #[cfg(feature = "vector2")]
          (1,2,1) => {
            match &arguments[..] {
              // r2
              [Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0))] => {
                return Ok(Box::new(VerticalConcatenateV2{out: e0.clone()}));
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vector3")]
          (1,3,1) => {
            match &arguments[..] {
              // r3
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e0)) => {
                    return Ok(Box::new(VerticalConcatenateV3{out: e0.clone()}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vector4")]
          (1,4,1) => {
            match &arguments[..] {
              // r4
              [Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e0))] => {
                return Ok(Box::new(VerticalConcatenateV4{out: e0.clone()}));
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vectord")]
          (1,m,1) => {
            match &arguments[..] {
              // rd
              [Value::[<Matrix $kind:camel>](Matrix::DVector(ref e0))] => {
                return Ok(Box::new(VerticalConcatenateVD{out: e0.clone()}));
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vector2")]
          (2,2,1) => {
            let mut out = Vector2::from_element($default);
            match &arguments[..] {
              // m1m1
              [Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateM1M1{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vector3")]
          (2,3,1) => {
            let mut out = Vector3::from_element($default);
            match &arguments[..] {
              //m1v2
              [Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateM1V2{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
              }
              //v2m1
              [Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateV2M1{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vector4")]
          (2,4,1) => {
            let mut out = Vector4::from_element($default);
            match &arguments[..] {
              // m1v3
              [Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateM1V3{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
              }
              // v3m1
              [Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateV3M1{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
              }
              // v2v2
              [Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateV2V2{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
              }              
              _ => todo!(),
            }
          } 
          #[cfg(feature = "vectord")]
          (2,m,1) => {
            let mut out = DVector::from_element(m,$default);
            match &arguments[..] {
              [Value::[<Matrix $kind:camel>](e0),Value::[<Matrix $kind:camel>](e1)] => {
                let e0 = e0.get_copyable_matrix();
                let e1 = e1.get_copyable_matrix();
                return Ok(Box::new(VerticalConcatenateVD2{e0, e1, out: new_ref(out)}));
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vector3")]
          (3,3,1) => {  
            let mut out = Vector3::from_element($default);
            match &arguments[..] {
              // m1 m1 m1
              [Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))] => {
                return Ok(Box::new(VerticalConcatenateM1M1M1{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
              }    
              _ => todo!()
            }
          }
          #[cfg(feature = "vector4")]
          (3,4,1) => {
            let mut out = Vector4::from_element($default);
            match &arguments[..] {
              // m1 m1 r2
              [Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e2))] => {
                return Ok(Box::new(VerticalConcatenateM1M1V2{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
              }
              // m1 r2 m1
              [Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))] => {
                return Ok(Box::new(VerticalConcatenateM1V2M1{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
              }
              // r2 m1 m1
              [Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))] => {
                return Ok(Box::new(VerticalConcatenateV2M1M1{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
              }
              _ => todo!()
            }
          }
          #[cfg(feature = "vectord")]
          (3,m,1) => {
            let mut out = DVector::from_element(m,$default);
            match &arguments[..] {
              [Value::[<Matrix $kind:camel>](e0),Value::[<Matrix $kind:camel>](e1),Value::[<Matrix $kind:camel>](e2)] => {
                let e0 = e0.get_copyable_matrix();
                let e1 = e1.get_copyable_matrix();
                let e2 = e2.get_copyable_matrix();
                return Ok(Box::new(VerticalConcatenateVD3{e0, e1, e2, out: new_ref(out)}));
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vector4")]
          (4,4,1) => {
            let mut out = Vector4::from_element($default);
            match &arguments[..] {
              // m1 m1 m1 m1
              [Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e3))] => {
                return Ok(Box::new(VerticalConcatenateM1M1M1M1{ e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: new_ref(out) }));
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vectord")]
          (4,m,1) => {
            let mut out = DVector::from_element(m,$default);
            match &arguments[..] {
              [Value::[<Matrix $kind:camel>](e0),Value::[<Matrix $kind:camel>](e1),Value::[<Matrix $kind:camel>](e2),Value::[<Matrix $kind:camel>](e3)] => {
                let e0 = e0.get_copyable_matrix();
                let e1 = e1.get_copyable_matrix();
                let e2 = e2.get_copyable_matrix();
                let e3 = e3.get_copyable_matrix();
                return Ok(Box::new(VerticalConcatenateVD4{e0, e1, e2, e3, out: new_ref(out)}));
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "vectord")]
          (l,m,1) => {
            let mut out = DVector::from_element(m,$default);
            let mut matrix_args: Vec<(Box<dyn CopyMat<$kind>>,usize)> = vec![];
            let mut scalar_args: Vec<(Ref<$kind>,usize)> = vec![];
            let mut i = 0;
            for arg in arguments.iter() {
              match &arg {
                Value::[<$kind:camel>](e0) => {
                  scalar_args.push((e0.clone(),i));
                  i += 1;
                }
                Value::[<Matrix $kind:camel>](e0) => {
                  matrix_args.push((e0.get_copyable_matrix(),i));
                  i += e0.shape()[0];
                }
                _ => todo!(),
              }
            }
            return Ok(Box::new(VerticalConcatenateVDN{scalar: scalar_args, matrix: matrix_args, out: new_ref(out)}));
          }
          #[cfg(feature = "matrix2")]
          (2,2,2) => {
            let mut out = Matrix2::from_element($default);
            match &arguments[..] {
              // v2v2
              [Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1))] => {return Ok(Box::new(VerticalConcatenateR2R2{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));}
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrix2x3")]
          (2,2,3) => {
            let mut out = Matrix2x3::from_element($default);
            match &arguments[..] {
              // r3r3
              [Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e1))] => {return Ok(Box::new(VerticalConcatenateR3R3{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));}
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrix3x2")]
          (2,3,2) => {
            let mut out = Matrix3x2::from_element($default);
            match &arguments[..] {
              // v2m2
              [Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix2(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateR2M2{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
              }
              // m2v2
              [Value::[<Matrix $kind:camel>](Matrix::Matrix2(ref e0)), Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateM2R2{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
              }
              _ => todo!(),
            }
            
          }
          #[cfg(feature = "matrix3")]
          (2,3,3) => {
            let mut out = Matrix3::from_element($default);
            match &arguments[..] {
              // v3m3x2
              [Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix2x3(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateR3M2x3 { e0: e0.clone(), e1: e1.clone(), out: new_ref(out) }));
              }
              // m3x2v3
              [Value::[<Matrix $kind:camel>](Matrix::Matrix2x3(ref e0)), Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e1))] => {
                return Ok(Box::new(VerticalConcatenateM2x3R3 { e0: e0.clone(), e1: e1.clone(), out: new_ref(out) }));
              }
              _ => todo!(),
            }
            
          }
          #[cfg(feature = "matrix4")]
          (2,4,4) => {
            let mut out = Matrix4::from_element($default);
            match &arguments[..] {
              // v4md
              [Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e0)), Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e1))] => Ok(Box::new(VerticalConcatenateR4MD{e0:e0.clone(),e1:e1.clone(),out:new_ref(out)})),
              // mdv4
              [Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e0)), Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e1))] => Ok(Box::new(VerticalConcatenateMDR4{e0:e0.clone(),e1:e1.clone(),out:new_ref(out)})),
              // mdmd
              [Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e0)), Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e1))] => Ok(Box::new(VerticalConcatenateMDMD{e0:e0.clone(),e1:e1.clone(),out:new_ref(out)})),
              _ => todo!(),
            }
            
          }
          #[cfg(feature = "matrixd")]
          (2,m,n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            match &arguments[..] {
              [Value::[<Matrix $kind:camel>](m0), Value::[<Matrix $kind:camel>](m1)] => {
                let e0 = m0.get_copyable_matrix();
                let e1 = m1.get_copyable_matrix();
                Ok(Box::new(VerticalConcatenateTwoArgs{e0, e1, out: new_ref(out)}))
              }
              _ => todo!(),
            }            
          }
          #[cfg(feature = "matrix3x2")]
          (3,3,2) => {
            let mut out = Matrix3x2::from_element($default);
            match &arguments[..] {
              // r2r2r2
              [Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e2))]=>Ok(Box::new(VerticalConcatenateR2R2R2{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:new_ref(out)})),
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrix3")]
          (3,3,3) => {
            let mut out = Matrix3::from_element($default);
            match &arguments[..] {
              // v3v3v3
              [Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e2))]=>Ok(Box::new(VerticalConcatenateR3R3R3{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:new_ref(out)})),
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrix4")]
          (3,4,4) => {
            let mut out = Matrix4::from_element($default);
            match &arguments[..] {
               // v4v4md
              [Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e1)),Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e2))]=>Ok(Box::new(VerticalConcatenateR4R4MD{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:new_ref(out)})),
              // v4mdv4
              [Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e0)),Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e2))]=>Ok(Box::new(VerticalConcatenateR4MDR4{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:new_ref(out)})),
              // mdv4v4
              [Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e2))]=>Ok(Box::new(VerticalConcatenateMDR4R4{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:new_ref(out)})),
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrixd")]
          (3,m,n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            match &arguments[..] {
              [Value::[<Matrix $kind:camel>](m0),Value::[<Matrix $kind:camel>](m1),Value::[<Matrix $kind:camel>](m2)] => {
                let e0 = m0.get_copyable_matrix();
                let e1 = m1.get_copyable_matrix();
                let e2 = m2.get_copyable_matrix();
                Ok(Box::new(VerticalConcatenateThreeArgs{e0,e1,e2,out:new_ref(out)}))
              }   
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrix4")]
          (4,4,4) => {
            let mut out = Matrix4::from_element($default);
            match &arguments[..] {
              // v4v4v4v4
              [Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e2)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e3))]=>Ok(Box::new(VerticalConcatenateR4R4R4R4{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),e3:e3.clone(),out:new_ref(out)})),
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrixd")]
          (4,m,n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            match &arguments[..] {
              [Value::[<Matrix $kind:camel>](m0),Value::[<Matrix $kind:camel>](m1),Value::[<Matrix $kind:camel>](m2),Value::[<Matrix $kind:camel>](m3)] => {
                let e0 = m0.get_copyable_matrix();
                let e1 = m1.get_copyable_matrix();
                let e2 = m2.get_copyable_matrix();
                let e3 = m3.get_copyable_matrix();
                Ok(Box::new(VerticalConcatenateFourArgs{e0,e1,e2,e3,out:new_ref(out)}))
              }   
              _ => todo!(),
            }
          }
          #[cfg(feature = "matrixd")]
          (l,m,n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            let mut args = vec![];
            for arg in arguments {
              match arg {
                Value::[<Matrix $kind:camel>](m0) => {
                  let e0 = m0.get_copyable_matrix();
                  args.push(e0);
                }
                _ => todo!(),
              }
            }
            Ok(Box::new(VerticalConcatenateNArgs{e0: args, out:new_ref(out)}))
          }
          _ => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind});}
        }
  }}}}}

fn impl_vertcat_fxn(arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {

  let kinds: Vec<ValueKind> = arguments.iter().map(|x| x.kind()).collect::<Vec<ValueKind>>();
  let target_kind = kinds[0].clone();

  #[cfg(feature = "f64")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::F64) { return impl_vertcat_arms!(F64, arguments, F64::default()) } }

  #[cfg(feature = "f32")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::F32) { return impl_vertcat_arms!(F32, arguments, F32::default()) } }

  #[cfg(feature = "u8")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::U8)  { return impl_vertcat_arms!(u8,  arguments, u8::default()) } }

  #[cfg(feature = "u16")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::U16) { return impl_vertcat_arms!(u16, arguments, u16::default()) } }

  #[cfg(feature = "u32")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::U32) { return impl_vertcat_arms!(u32, arguments, u32::default()) } }

  #[cfg(feature = "u64")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::U64) { return impl_vertcat_arms!(u64, arguments, u64::default()) } }

  #[cfg(feature = "u128")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::U128){ return impl_vertcat_arms!(u128, arguments, u128::default()) } }

  #[cfg(feature = "bool")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::Bool) { return impl_vertcat_arms!(bool, arguments, bool::default()) } }

  #[cfg(feature = "string")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::String) { return impl_vertcat_arms!(String, arguments, String::default()) } }

  #[cfg(feature = "rational")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::RationalNumber) { return impl_vertcat_arms!(RationalNumber, arguments, RationalNumber::default()) } }

  #[cfg(feature = "complex")]
  { if ValueKind::is_compatible(target_kind.clone(), ValueKind::ComplexNumber) { return impl_vertcat_arms!(ComplexNumber, arguments, ComplexNumber::default()) } }

  Err(MechError {
    file: file!().to_string(),
    tokens: vec![],
    msg: format!("Vertical concatenation not implemented for type {:?}", target_kind),
    id: line!(),
    kind: MechErrorKind::UnhandledFunctionArgumentKind,
  })
}


pub struct MaxtrixVertCat {}
impl NativeFunctionCompiler for MaxtrixVertCat {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    impl_vertcat_fxn(arguments)
  }
}