#[macro_use]
use crate::stdlib::*;

// Horizontal Concatenate -----------------------------------------------------

macro_rules! horzcat_one_arg {
  ($fxn:ident, $e0:ident, $out:ident, $opt:ident) => {
    #[derive(Debug)]
    struct $fxn<T> {
      e0: Ref<$e0<T>>,
      out: Ref<$out<T>>,
    }
    impl<T> MechFunction for $fxn<T>
    where
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
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
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  };}

macro_rules! horzcat_two_args {
  ($fxn:ident, $e1:ident, $e2:ident, $out:ident, $opt:ident) => {
    #[derive(Debug)]
    struct $fxn<T> {
      e0: Ref<$e1<T>>,
      e1: Ref<$e2<T>>,
      out: Ref<$out<T>>,
    }
    impl<T> MechFunction for $fxn<T>
    where
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
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
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  };}

macro_rules! horzcat_three_args {
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
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
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
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  };} 
  
macro_rules! horzcat_four_args {
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
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
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
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  };}   

struct HorizontalConcatenateTwoArgs<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  out: Ref<DMatrix<T>>,
}
impl<T> MechFunction for HorizontalConcatenateTwoArgs<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    let offset = self.e0.copy_into(&self.out,0);
    self.e1.copy_into(&self.out,offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("HorizontalConcatenateTwoArgs{:?}", self.out) }
}
    
struct HorizontalConcatenateThreeArgs<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  e2: Box<dyn CopyMat<T>>,
  out: Ref<DMatrix<T>>,
}
impl<T> MechFunction for HorizontalConcatenateThreeArgs<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    let mut offset = self.e0.copy_into(&self.out,0);
    offset += self.e1.copy_into(&self.out,offset);
    self.e2.copy_into(&self.out,offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("HorizontalConcatenateThreeArgs{:?}", self.out) }
}

struct HorizontalConcatenateFourArgs<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  e2: Box<dyn CopyMat<T>>,
  e3: Box<dyn CopyMat<T>>,
  out: Ref<DMatrix<T>>,
}
impl<T> MechFunction for HorizontalConcatenateFourArgs<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    let mut offset = self.e0.copy_into(&self.out,0);
    offset += self.e1.copy_into(&self.out,offset);
    offset += self.e2.copy_into(&self.out,offset);
    self.e3.copy_into(&self.out,offset);

  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("HorizontalConcatenateFourArgs{:?}", self.out) }
}

struct HorizontalConcatenateNArgs<T> {
  e0: Vec<Box<dyn CopyMat<T>>>,
  out: Ref<DMatrix<T>>,
}
impl<T> MechFunction for HorizontalConcatenateNArgs<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    let mut offset = 0;
    for e in &self.e0 {
      offset += e.copy_into(&self.out,offset);
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("HorizontalConcatenateNArgs{:?}", self.out) }
}

macro_rules! horizontal_concatenate {
  ($name:ident, $vec_size:expr) => {
    paste!{
      #[derive(Debug)]
      struct $name<T> {
        out: Ref<[<RowVector $vec_size>]<T>>,
      }

      impl<T> MechFunction for $name<T> 
      where
        T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
        Ref<[<RowVector $vec_size>]<T>>: ToValue
      {
        fn solve(&self) {}
        fn out(&self) -> Value { self.out.to_value() }
        fn to_string(&self) -> String { format!("{:?}", self) }
      }
    }
  };}  

#[derive(Debug)]
struct HorizontalConcatenateRD<T> {
  out: Ref<RowDVector<T>>,
}

impl<T> MechFunction for HorizontalConcatenateRD<T> 
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowDVector<T>>: ToValue
{
  fn solve(&self) {}
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateRD2<T> {
  e0: Matrix<T>,
  e1: Matrix<T>,
  out: Ref<RowDVector<T>>,
}

impl<T> MechFunction for HorizontalConcatenateRD2<T> 
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowDVector<T>>: ToValue
{
  fn solve(&self) {          
    unsafe {
      let e0_ptr = self.e0.as_vec();
      let e1_ptr = self.e1.as_vec();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      let mut i = 0;
      for ix in 0..e0_ptr.len() {
        out_ptr[i] = e0_ptr[ix].clone();
        i += 1;
      }
      for ix in 0..e1_ptr.len() {
        out_ptr[i] = e1_ptr[ix].clone();
        i += 1;
      }
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateRD3<T> {
  e0: Matrix<T>,
  e1: Matrix<T>,
  e2: Matrix<T>,
  out: Ref<RowDVector<T>>,
}

impl<T> MechFunction for HorizontalConcatenateRD3<T> 
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowDVector<T>>: ToValue
{
  fn solve(&self) {          
    unsafe {
      let e0_ptr = self.e0.as_vec();
      let e1_ptr = self.e1.as_vec();
      let e2_ptr = self.e2.as_vec();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      let mut i = 0;
      for ix in 0..e0_ptr.len() {
        out_ptr[i] = e0_ptr[ix].clone();
        i += 1;
      }
      for ix in 0..e1_ptr.len() {
        out_ptr[i] = e1_ptr[ix].clone();
        i += 1;
      }
      for ix in 0..e2_ptr.len() {
        out_ptr[i] = e2_ptr[ix].clone();
        i += 1;
      }
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateRD4<T> {
  e0: Matrix<T>,
  e1: Matrix<T>,
  e2: Matrix<T>,
  e3: Matrix<T>,
  out: Ref<RowDVector<T>>,
}

impl<T> MechFunction for HorizontalConcatenateRD4<T> 
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowDVector<T>>: ToValue
{
  fn solve(&self) {          
    unsafe {
      let e0_ptr = self.e0.as_vec();
      let e1_ptr = self.e1.as_vec();
      let e2_ptr = self.e2.as_vec();
      let e3_ptr = self.e3.as_vec();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      let mut i = 0;
      for ix in 0..e0_ptr.len() {
        out_ptr[i] = e0_ptr[ix].clone();
        i += 1;
      }
      for ix in 0..e1_ptr.len() {
        out_ptr[i] = e1_ptr[ix].clone();
        i += 1;
      }
      for ix in 0..e2_ptr.len() {
        out_ptr[i] = e2_ptr[ix].clone();
        i += 1;
      }
      for ix in 0..e3_ptr.len() {
        out_ptr[i] = e3_ptr[ix].clone();
        i += 1;
      }
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateRDN<T> {
  scalar: Vec<(Ref<T>,usize)>,
  matrix: Vec<(Matrix<T>,usize)>,
  out: Ref<RowDVector<T>>,
}

impl<T> MechFunction for HorizontalConcatenateRDN<T> 
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowDVector<T>>: ToValue
{
  fn solve(&self) {
    unsafe {
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      for (e,i) in &self.matrix {
        let e0_ptr = e.as_vec();
        let mut i = *i;
        for ix in 0..e0_ptr.len() {
          out_ptr[i] = e0_ptr[ix].clone();
          i += 1;
        }
      }
      for (e,i) in &self.scalar {
        out_ptr[*i] = e.borrow().clone();
      }
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateS1<T> {
  out: Ref<Matrix1<T>>,
}

impl<T> MechFunction for HorizontalConcatenateS1<T> 
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<Matrix1<T>>: ToValue
{
  fn solve(&self) {}
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

horizontal_concatenate!(HorizontalConcatenateS2,2);
horizontal_concatenate!(HorizontalConcatenateS3,3);
horizontal_concatenate!(HorizontalConcatenateS4,4);
horizontal_concatenate!(HorizontalConcatenateR2,2);
horizontal_concatenate!(HorizontalConcatenateR3,3);
horizontal_concatenate!(HorizontalConcatenateR4,4);

#[derive(Debug)]
struct HorizontalConcatenateSD<T> {
  out: Ref<RowDVector<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSD<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowDVector<T>>: ToValue
{
  fn solve(&self) { }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

macro_rules! horzcat_single {
  ($name:ident,$shape:ident) => {
    #[derive(Debug)]
    struct $name<T> {
      out: Ref<$shape<T>>,
    }
    impl<T> MechFunction for $name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$shape<T>>: ToValue
    {
      fn solve(&self) { }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  }
}

horzcat_single!(HorizontalConcatenateM1,Matrix1);
horzcat_single!(HorizontalConcatenateM2,Matrix2);
horzcat_single!(HorizontalConcatenateM3,Matrix3);
horzcat_single!(HorizontalConcatenateM4,Matrix4);
horzcat_single!(HorizontalConcatenateM2x3,Matrix2x3);
horzcat_single!(HorizontalConcatenateM3x2,Matrix3x2);
horzcat_single!(HorizontalConcatenateMD,DMatrix);
horzcat_single!(HorizontalConcatenateV2,Vector2);
horzcat_single!(HorizontalConcatenateV3,Vector3);
horzcat_single!(HorizontalConcatenateV4,Vector4);
horzcat_single!(HorizontalConcatenateVD,DVector);

macro_rules! horzcat_sr2 {
  ($out:expr, $e0:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e0[1].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateSR2, RowVector2, RowVector3, horzcat_sr2);

macro_rules! horzcat_r2s {
  ($out:expr, $e0:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateR2S, RowVector2, RowVector3, horzcat_r2s);

macro_rules! horzcat_sm1 {
  ($out:expr, $e0:expr) => {
    $out[1] = $e0[0].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateSM1, Matrix1, RowVector2, horzcat_sm1);

macro_rules! horzcat_m1s {
  ($out:expr, $e0:expr) => {
    $out[0] = $e0[0].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateM1S, Matrix1, RowVector2, horzcat_m1s);

macro_rules! horzcat_sssm1 {
  ($out:expr, $e0:expr) => {
    $out[3] = $e0[0].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateSSSM1, Matrix1, RowVector4, horzcat_sssm1);

macro_rules! horzcat_ssm1s {
  ($out:expr, $e0:expr) => {
    $out[2] = $e0[0].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateSSM1S, Matrix1, RowVector4, horzcat_ssm1s);

macro_rules! horzcat_sm1ss {
  ($out:expr, $e0:expr) => {
    $out[1] = $e0[0].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateSM1SS, Matrix1, RowVector4, horzcat_sm1ss);

macro_rules! horzcat_m1sss {
  ($out:expr, $e0:expr) => {
    $out[0] = $e0[0].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateM1SSS, Matrix1, RowVector4, horzcat_m1sss);

macro_rules! horzcat_sr3 {
  ($out:expr, $e0:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e0[1].clone();
    $out[3] = $e0[2].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateSR3, RowVector3, RowVector4, horzcat_sr3);

macro_rules! horzcat_r3s {
  ($out:expr, $e0:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateR3S, RowVector3, RowVector4, horzcat_r3s);

macro_rules! horzcat_ssm1 {
  ($out:expr, $e0:expr) => {
    $out[2] = $e0[0].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateSSM1, Matrix1, RowVector3, horzcat_ssm1);

macro_rules! horzcat_sm1s {
  ($out:expr, $e0:expr) => {
    $out[1] = $e0[0].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateSM1S, Matrix1, RowVector3, horzcat_sm1s);

macro_rules! horzcat_m1ss {
  ($out:expr, $e0:expr) => {
    $out[0] = $e0[0].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateM1SS, Matrix1, RowVector3, horzcat_m1ss);

macro_rules! horzcat_ssr2 {
  ($out:expr, $e0:expr) => {
    $out[2] = $e0[0].clone();
    $out[3] = $e0[1].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateSSR2, RowVector2, RowVector4, horzcat_ssr2);

macro_rules! horzcat_sr2s {
  ($out:expr, $e0:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e0[1].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateSR2S, RowVector2, RowVector4, horzcat_sr2s);

macro_rules! horzcat_r2ss {
  ($out:expr, $e0:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
  };
}
horzcat_one_arg!(HorizontalConcatenateR2SS, RowVector2, RowVector4, horzcat_r2ss);

macro_rules! horzcat_m1m1s {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
  };}
horzcat_two_args!(HorizontalConcatenateM1M1S,Matrix1,Matrix1,RowVector3,horzcat_m1m1s);

macro_rules! horzcat_m1m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
  };}
horzcat_two_args!(HorizontalConcatenateM1M1,Matrix1,Matrix1,RowVector2,horzcat_m1m1);

macro_rules! horzcat_m1sm1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[2] = $e1[0].clone();
  };}
horzcat_two_args!(HorizontalConcatenateM1SM1,Matrix1,Matrix1,RowVector3,horzcat_m1sm1);

macro_rules! horzcat_sm1m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e1[0].clone();
  };}
horzcat_two_args!(HorizontalConcatenateSM1M1,Matrix1,Matrix1,RowVector3,horzcat_sm1m1);

macro_rules! horzcat_r2r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };}
horzcat_two_args!(HorizontalConcatenateR2R2,RowVector2,RowVector2,RowVector4,horzcat_r2r2);

macro_rules! horzcat_m1r3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
    $out[3] = $e1[2].clone();
  };}
horzcat_two_args!(HorizontalConcatenateM1R3,Matrix1,RowVector3,RowVector4,horzcat_m1r3);

macro_rules! horzcat_r3m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e1[0].clone();
  };}
horzcat_two_args!(HorizontalConcatenateR3M1,RowVector3,Matrix1,RowVector4,horzcat_r3m1);

macro_rules! horzcat_sm1r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };}
horzcat_two_args!(HorizontalConcatenateSM1R2,Matrix1,RowVector2,RowVector4,horzcat_sm1r2);

macro_rules! horzcat_m1sr2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };}
horzcat_two_args!(HorizontalConcatenateM1SR2,Matrix1,RowVector2,RowVector4,horzcat_m1sr2);
  
macro_rules! horzcat_sm1sm1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[1] = $e0[0].clone();
    $out[3] = $e1[0].clone();
  };} 
horzcat_two_args!(HorizontalConcatenateSM1SM1,Matrix1,Matrix1,RowVector4,horzcat_sm1sm1);

macro_rules! horzcat_m1r2s {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
  };} 
horzcat_two_args!(HorizontalConcatenateM1R2S,Matrix1,RowVector2,RowVector4,horzcat_m1r2s);

macro_rules! horzcat_r2m1s {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
  };} 
horzcat_two_args!(HorizontalConcatenateR2M1S,RowVector2,Matrix1,RowVector4,horzcat_r2m1s);

macro_rules! horzcat_r2sm1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[3] = $e1[0].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateR2SM1, RowVector2, Matrix1, RowVector4, horzcat_r2sm1);

macro_rules! horzcat_sr2m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e0[1].clone();
    $out[3] = $e1[0].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateSR2M1, RowVector2, Matrix1, RowVector4, horzcat_sr2m1);

macro_rules! horzcat_ssm1m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[2] = $e0[0].clone();
    $out[3] = $e1[0].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateSSM1M1, Matrix1, Matrix1, RowVector4, horzcat_ssm1m1);

macro_rules! horzcat_m1m1ss {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateM1M1SS, Matrix1, Matrix1, RowVector4, horzcat_m1m1ss);

macro_rules! horzcat_sm1m1s {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e1[0].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateSM1M1S, Matrix1, Matrix1, RowVector4, horzcat_sm1m1s);

macro_rules! horzcat_m1ssm1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[3] = $e1[0].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateM1SSM1, Matrix1, Matrix1, RowVector4, horzcat_m1ssm1);

macro_rules! horzcat_m1sm1s {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[2] = $e1[0].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateM1SM1S, Matrix1, Matrix1, RowVector4, horzcat_m1sm1s);

macro_rules! horzcat_m1r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateM1R2, Matrix1, RowVector2, RowVector3, horzcat_m1r2);

macro_rules! horzcat_r2m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateR2M1, RowVector2, Matrix1, RowVector3, horzcat_r2m1);

macro_rules! horzcat_m1m1m1 {
  ($out:expr, $e0:expr,$e1:expr,$e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e2[0].clone();
  };
}
horzcat_three_args!(HorizontalConcatenateM1M1M1,Matrix1,Matrix1,Matrix1,RowVector3, horzcat_m1m1m1);

macro_rules! horzcat_m1m1r2 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e2[0].clone();
    $out[3] = $e2[1].clone();
  };
}
horzcat_three_args!(HorizontalConcatenateM1M1R2, Matrix1, Matrix1, RowVector2, RowVector4, horzcat_m1m1r2);

macro_rules! horzcat_m1r2m1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
    $out[3] = $e2[0].clone();
  };
}
horzcat_three_args!(HorizontalConcatenateM1R2M1, Matrix1, RowVector2, Matrix1, RowVector4, horzcat_m1r2m1);

macro_rules! horzcat_r2m1m1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e2[0].clone();
  };
}
horzcat_three_args!(HorizontalConcatenateR2M1M1, RowVector2, Matrix1, Matrix1, RowVector4, horzcat_r2m1m1);

macro_rules! horzcat_sm1m1m1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e2[0].clone();
  };
}
horzcat_three_args!(HorizontalConcatenateSM1M1M1, Matrix1, Matrix1, Matrix1, RowVector4, horzcat_sm1m1m1);

macro_rules! horzcat_m1sm1m1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e2[0].clone();
  };
}
horzcat_three_args!(HorizontalConcatenateM1SM1M1, Matrix1, Matrix1, Matrix1, RowVector4, horzcat_m1sm1m1);

macro_rules! horzcat_m1m1sm1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[3] = $e2[0].clone();
  };
}
horzcat_three_args!(HorizontalConcatenateM1M1SM1, Matrix1, Matrix1, Matrix1, RowVector4, horzcat_m1m1sm1);

macro_rules! horzcat_m1m1m1s {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e2[0].clone();
  };
}
horzcat_three_args!(HorizontalConcatenateM1M1M1S, Matrix1, Matrix1, Matrix1, RowVector4, horzcat_m1m1m1s);

#[derive(Debug)]
struct HorizontalConcatenateM1M1M1M1<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<Matrix1<T>>,
  e3: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1M1M1M1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
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
  fn to_string(&self) -> String { format!("{:?}", self) }
}

macro_rules! horzcat_v2v2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateV2V2, Vector2, Vector2, Matrix2, horzcat_v2v2);

macro_rules! horzcat_v3v3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e1[0].clone();
    $out[4] = $e1[1].clone();
    $out[5] = $e1[2].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateV3V3, Vector3, Vector3, Matrix3x2, horzcat_v3v3);

macro_rules! horzcat_v2m2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
    $out[4] = $e1[2].clone();
    $out[5] = $e1[3].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateV2M2, Vector2, Matrix2, Matrix2x3, horzcat_v2m2);

macro_rules! horzcat_m2v2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e0[3].clone();
    $out[4] = $e1[0].clone();
    $out[5] = $e1[1].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateM2V2, Matrix2, Vector2, Matrix2x3, horzcat_m2v2);

macro_rules! horzcat_m3x2v3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e0[3].clone();
    $out[4] = $e0[4].clone();
    $out[5] = $e0[5].clone();
    $out[6] = $e1[0].clone();
    $out[7] = $e1[1].clone();
    $out[8] = $e1[2].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateM3x2V3, Matrix3x2, Vector3, Matrix3, horzcat_m3x2v3);

macro_rules! horzcat_v3m3x2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e1[0].clone();
    $out[4] = $e1[1].clone();
    $out[5] = $e1[2].clone();
    $out[6] = $e1[3].clone();
    $out[7] = $e1[4].clone();
    $out[8] = $e1[5].clone();
  };
}
horzcat_two_args!(HorizontalConcatenateV3M3x2, Vector3, Matrix3x2, Matrix3, horzcat_v3m3x2);

macro_rules! horzcat_v4md {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e0[3].clone();
    let offset = 4;
    for i in 0..$e1.len() {
      $out[i + offset] = $e1[i].clone();
    }
  };
}
horzcat_two_args!(HorizontalConcatenateV4MD, Vector4, DMatrix, Matrix4, horzcat_v4md);

macro_rules! horzcat_mdv4 {
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
horzcat_two_args!(HorizontalConcatenateMDV4, DMatrix, Vector4, Matrix4, horzcat_mdv4);

macro_rules! horzcat_mdmd {
  ($out:expr, $e0:expr, $e1:expr) => {
    let e0_len = $e0.len();
    for i in 0..e0_len {
      $out[i] = $e0[i].clone();
    }
    let offset = e0_len;
    for i in 0..$e1.len() {
      $out[i + offset] = $e1[i].clone();
    }
  };
}
horzcat_two_args!(HorizontalConcatenateMDMD, DMatrix, DMatrix, Matrix4, horzcat_mdmd);

macro_rules! horzcat_mdmdmd {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    let e0_len = $e0.len();
    for i in 0..e0_len {
      $out[i] = $e0[i].clone();
    }
    let offset = e0_len;
    for i in 0..$e1.len() {
      $out[i + offset] = $e1[i].clone();
    }
    let offset = offset + $e1.len();
    for i in 0..$e2.len() {
      $out[i + offset] = $e2[i].clone();
    }
  };
}

horzcat_three_args!(HorizontalConcatenateV2V2V2, Vector2, Vector2, Vector2, Matrix2x3, horzcat_mdmdmd);
horzcat_three_args!(HorizontalConcatenateV3V3V3, Vector3, Vector3, Vector3, Matrix3, horzcat_mdmdmd);
horzcat_three_args!(HorizontalConcatenateV4V4MD, Vector4, Vector4, DMatrix, Matrix4, horzcat_mdmdmd);
horzcat_three_args!(HorizontalConcatenateV4MDV4, Vector4, DMatrix, Vector4, Matrix4, horzcat_mdmdmd);
horzcat_three_args!(HorizontalConcatenateMDV4V4, DMatrix, Vector4, Vector4, Matrix4, horzcat_mdmdmd);


macro_rules! horzcat_mdmdmdmd {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr, $e3:expr) => {
    let e0_len = $e0.len();
    for i in 0..e0_len {
      $out[i] = $e0[i].clone();
    }
    let offset = e0_len;
    for i in 0..$e1.len() {
      $out[i + offset] = $e1[i].clone();
    }
    let offset = offset + $e1.len();
    for i in 0..$e2.len() {
      $out[i + offset] = $e2[i].clone();
    }
    let offset = offset + $e2.len();
    for i in 0..$e3.len() {
      $out[i + offset] = $e3[i].clone();
    }
  };
}

horzcat_four_args!(HorizontalConcatenateV4V4V4V4, Vector4, Vector4, Vector4, Vector4, Matrix4, horzcat_mdmdmdmd);

macro_rules! impl_horzcat_arms {
  ($kind:ident, $args:expr, $default:expr) => {
    paste!{
    {
      let arguments = $args;   
      let rows = arguments[0].shape()[0];
      let columns:usize = arguments.iter().fold(0, |acc, x| acc + x.shape()[1]);
      let rows:usize = arguments[0].shape()[0];
      let nargs = arguments.len();
      let kinds: Vec<ValueKind> = arguments.iter().map(|x| x.kind()).collect::<Vec<ValueKind>>();
      let no_refs = !kinds.iter().any(|x| {
        match x {
          ValueKind::Reference(_) => true,
          _ => false,
      }});
      if no_refs {
        let mat: Vec<$kind> = arguments.iter().flat_map(|v| v.[<as_vec $kind:lower>]().unwrap()).collect::<Vec<$kind>>();
        match &mat[..] {
          [e0]             => {return Ok(Box::new(HorizontalConcatenateS1{out:new_ref(Matrix1::from_vec(mat))}));}
          [e0, e1]         => {return Ok(Box::new(HorizontalConcatenateS2{out:new_ref(RowVector2::from_vec(mat))}));}
          [e0, e1, e2]     => {return Ok(Box::new(HorizontalConcatenateS3{out:new_ref(RowVector3::from_vec(mat))}));}
          [e0, e1, e2, e3] => {return Ok(Box::new(HorizontalConcatenateS4{out:new_ref(RowVector4::from_vec(mat))}));}
          _ => {return Ok(Box::new(HorizontalConcatenateSD{out:new_ref(RowDVector::from_vec(mat))}));}
        }      
      } else {
        match (nargs,rows,columns) {
          #[cfg(feature = "Matrix1")]
          (1,1,1) => {
            let mut out = Matrix1::from_element($default);
            match &arguments[..] {
              // m1
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)) => {
                    return Ok(Box::new(HorizontalConcatenateM1{out: e0.clone()}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "RowVector2")]
          (1,1,2) => {
            let mut out = RowVector2::from_element($default);
            match &arguments[..] {
              // r2
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)) => {
                    return Ok(Box::new(HorizontalConcatenateR2{out: e0.clone()}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "RowVector3")]
          (1,1,3) => {
            let mut out = RowVector3::from_element($default);
            match &arguments[..] {
              // r3
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e0)) => {
                    return Ok(Box::new(HorizontalConcatenateR3{out: e0.clone()}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "RowVector4")]
          (1,1,4) => {
            let mut out = RowVector4::from_element($default);
            match &arguments[..] {
              // r4
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e0)) => {
                    return Ok(Box::new(HorizontalConcatenateR4{out: e0.clone()}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "RowVectorD")]
          (1,1,n) => {
            let mut out = RowVector4::from_element($default);
            match &arguments[..] {
              // rd
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::RowDVector(ref e0)) => {
                    return Ok(Box::new(HorizontalConcatenateRD{out: e0.clone()}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "RowVector2")]
          (2,1,2) => {
            let mut out = RowVector2::from_element($default);
            match &arguments[..] {
              // s1m1
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1)] => {
                match *e1.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSM1{e0: e1.clone(), out: new_ref(out)}));
                  }
                  Value::[<$kind:camel>](ref e1) => {
                    out[0] = e0.borrow().clone();
                    out[1] = e1.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateR2{out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              // m1s1
              [Value::MutableReference(e0), Value::[<$kind:camel>](e1)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)) => {
                    out[1] = e1.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateM1S{e0: e0.clone(), out: new_ref(out)}));
                  }
                  Value::[<$kind:camel>](ref e0) => {
                    out[0] = e0.borrow().clone();
                    out[1] = e1.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateR2{out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }              
              // m1m1
              [Value::MutableReference(e0), Value::MutableReference(e1)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))) => {
                    return Ok(Box::new(HorizontalConcatenateM1M1{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  (Value::[<$kind:camel>](ref e0),Value::[<$kind:camel>](ref e1)) => {
                    out[0] = e0.borrow().clone();
                    out[1] = e1.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateR2{out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }      
              _ => todo!(),
            }
          }
          #[cfg(feature = "RowVector3")]
          (2,1,3) => {
            let mut out = RowVector3::from_element($default);
            match &arguments[..] {
              //sr2
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1)] => {
                match *e1.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1)) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSR2{e0: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              //r2s
              [Value::MutableReference(e0),Value::[<$kind:camel>](e1)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)) => {
                    out[2] = e1.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateR2S{e0: e0.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              [Value::MutableReference(e0),Value::MutableReference(e1)] => {
                match (&*e0.borrow(),&*e1.borrow()) {
                  //m1r2
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1))) => {
                    return Ok(Box::new(HorizontalConcatenateM1R2{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  //r2m1
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))) => {
                    return Ok(Box::new(HorizontalConcatenateR2M1{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "RowVector4")]
          (2,1,4) => {
            let mut out = RowVector4::from_element($default);
            match &arguments[..] {
              // s r3
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1)] => {
                match *e1.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e1)) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSR3{e0: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              // r3 s
              [Value::MutableReference(e0),Value::[<$kind:camel>](e1)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e0)) => {
                    out[3] = e1.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateR3S{e0: e0.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              [Value::MutableReference(e0),Value::MutableReference(e1)] => {
                match (&*e0.borrow(),&*e1.borrow()) {
                  // m1 r3
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e1))) => {
                    return Ok(Box::new(HorizontalConcatenateM1R3{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  // r3 m1
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))) => {
                    return Ok(Box::new(HorizontalConcatenateR3M1{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  // r2 r2
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1))) => {
                    return Ok(Box::new(HorizontalConcatenateR2R2{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          } 
          #[cfg(feature = "RowVectorD")]
          (2,1,n) => {
            let mut out = RowDVector::from_element(n,$default);
            match &arguments[..] {
              [Value::MutableReference(e0),Value::MutableReference(e1)] => {
                match (&*e0.borrow(),&*e1.borrow()) {
                  (Value::[<Matrix $kind:camel>](e0),Value::[<Matrix $kind:camel>](e1)) => {
                    return Ok(Box::new(HorizontalConcatenateRD2{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "RowVector3")]
          (3,1,3) => {  
            let mut out = RowVector3::from_element($default);
            match &arguments[..] {
              // s s m1
              [Value::[<$kind:camel>](e0), Value::[<$kind:camel>](e1), Value::MutableReference(e2)] => {
                match *e2.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2)) => {
                    out[0] = e0.borrow().clone();
                    out[1] = e1.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSSM1{e0: e2.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              // s m1 s
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1), Value::[<$kind:camel>](e2)] => {
                match *e1.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)) => {
                    out[0] = e0.borrow().clone();
                    out[2] = e2.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSM1S{e0: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              // m1 s s
              [Value::MutableReference(e0), Value::[<$kind:camel>](e1), Value::[<$kind:camel>](e2)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)) => {
                    out[1] = e1.borrow().clone();
                    out[2] = e2.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateM1SS{e0: e0.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              // m1 m1 s
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::[<$kind:camel>](e2)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))) => {
                    out[2] = e2.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateM1M1S{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              // m1 s m1
              [Value::MutableReference(e0), Value::[<$kind:camel>](e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(), e2.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    out[1] = e1.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateM1SM1{e0: e0.clone(), e1: e2.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              // s m1 m1
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e1.borrow().clone(), e2.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSM1M1{e0: e1.clone(), e1: e2.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }    
              // m1 m1 m1
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(), e1.borrow().clone(), e2.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    return Ok(Box::new(HorizontalConcatenateM1M1M1{e0: e1.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }           
              _ => todo!()
            }
          }
          #[cfg(feature = "RowVector4")]
          (3,1,4) => {
            let mut out = RowVector4::from_element($default);
            match &arguments[..] {
              // s s r2
              [Value::[<$kind:camel>](e0), Value::[<$kind:camel>](e1), Value::MutableReference(e2)] => {
                match *e2.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e2)) => {
                    out[0] = e0.borrow().clone();
                    out[1] = e1.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSSR2{e0: e2.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              // s r2 s1
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1), Value::[<$kind:camel>](e2)] => {
                match *e1.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1)) => {
                    out[0] = e0.borrow().clone();
                    out[3] = e2.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSR2S{e0: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }      
              // r2 s s
              [Value::MutableReference(e0), Value::[<$kind:camel>](e1), Value::[<$kind:camel>](e2)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)) => {
                    out[2] = e1.borrow().clone();
                    out[3] = e2.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateR2SS{e0: e0.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }    
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(),e1.borrow().clone(),e2.borrow().clone()) {
                  // m1 m1 r2
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),
                   Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e2))) => {
                    return Ok(Box::new(HorizontalConcatenateM1M1R2{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
                  }
                  // m1 r2 m1
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),
                   Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    return Ok(Box::new(HorizontalConcatenateM1R2M1{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
                  }
                  // r2 m1 m1
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    return Ok(Box::new(HorizontalConcatenateR2M1M1{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }        
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e1.borrow().clone(),e2.borrow().clone()) {
                  // s m1 r2
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e2))) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSM1R2{e0: e1.clone(), e1: e2.clone(), out: new_ref(out)}));
                  }
                  // s r2 m1
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSR2M1 { e0: e1.clone(), e1: e2.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              [Value::MutableReference(e0), Value::[<$kind:camel>](e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(), e2.borrow().clone()) {
                  // m1 s r2
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e2))) => {
                    out[1] = e1.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateM1SR2 { e0: e0.clone(), e1: e2.clone(), out: new_ref(out) }));
                  }
                  // r2 s m1
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    out[2] = e1.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateR2SM1 { e0: e0.clone(), e1: e2.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::[<$kind:camel>](e2)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  // m1 r2 s
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1))) => {
                    out[3] = e2.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateM1R2S { e0: e0.clone(), e1: e1.clone(), out: new_ref(out) }));
                  }
                  // r2 m1 s
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))) => {
                    out[3] = e2.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateR2M1S { e0: e0.clone(), e1: e1.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!()
            }
          }
          #[cfg(feature = "RowVectorD")]
          (3,1,n) => {
            let mut out = RowDVector::from_element(n,$default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(),e1.borrow().clone(),e2.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](e0),Value::[<Matrix $kind:camel>](e1),Value::[<Matrix $kind:camel>](e2)) => {
                    return Ok(Box::new(HorizontalConcatenateRD3{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "RowVector4")]
          (4,1,4) => {
            let mut out = RowVector4::from_element($default);
            match &arguments[..] {
             // s s s m1
              [Value::[<$kind:camel>](e0), Value::[<$kind:camel>](e1), Value::[<$kind:camel>](e2), Value::MutableReference(e3)] => {
                match (e3.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e3))) => {
                    out[0] = e0.borrow().clone();
                    out[1] = e1.borrow().clone();
                    out[2] = e2.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSSSM1 { e0: e3.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              // s s m1 s
              [Value::[<$kind:camel>](e0), Value::[<$kind:camel>](e1), Value::MutableReference(e2), Value::[<$kind:camel>](e3)] => {
                match (e2.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    out[0] = e0.borrow().clone();
                    out[1] = e1.borrow().clone();
                    out[3] = e3.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSSM1S { e0: e2.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              
              // s m1 s s
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1), Value::[<$kind:camel>](e2), Value::[<$kind:camel>](e3)] => {
                match (e1.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))) => {
                    out[0] = e0.borrow().clone();
                    out[2] = e2.borrow().clone();
                    out[3] = e3.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSM1SS { e0: e1.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              // m1 s s s
              [Value::MutableReference(e0), Value::[<$kind:camel>](e1), Value::[<$kind:camel>](e2), Value::[<$kind:camel>](e3)] => {
                match (e0.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0))) => {
                    out[1] = e1.borrow().clone();
                    out[2] = e2.borrow().clone();
                    out[3] = e3.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateM1SSS { e0: e0.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              
              // s s m1 m1
              [Value::[<$kind:camel>](e0), Value::[<$kind:camel>](e1), Value::MutableReference(e2), Value::MutableReference(e3)] => {
                match (e2.borrow().clone(), e3.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e3))) => {
                    out[0] = e0.borrow().clone();
                    out[1] = e1.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSSM1M1 { e0: e2.clone(), e1: e3.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              
              // m1 m1 s s
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::[<$kind:camel>](e2), Value::[<$kind:camel>](e3)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))) => {
                    out[2] = e2.borrow().clone();
                    out[3] = e3.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateM1M1SS { e0: e0.clone(), e1: e1.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              
              // s m1 m1 s
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1), Value::MutableReference(e2), Value::[<$kind:camel>](e3)] => {
                match (e1.borrow().clone(), e2.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    out[0] = e0.borrow().clone();
                    out[3] = e3.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSM1M1S { e0: e1.clone(), e1: e2.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              
              // m1 s s m1
              [Value::MutableReference(e0), Value::[<$kind:camel>](e1), Value::[<$kind:camel>](e2), Value::MutableReference(e3)] => {
                match (e0.borrow().clone(), e3.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e3))) => {
                    out[1] = e1.borrow().clone();
                    out[2] = e2.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateM1SSM1 { e0: e0.clone(), e1: e3.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              
              // m1 s m1 s
              [Value::MutableReference(e0), Value::[<$kind:camel>](e1), Value::MutableReference(e2), Value::[<$kind:camel>](e3)] => {
                match (e0.borrow().clone(), e2.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    out[1] = e1.borrow().clone();
                    out[3] = e3.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateM1SM1S { e0: e0.clone(), e1: e2.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              
              // s m1 s m1
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1), Value::[<$kind:camel>](e2), Value::MutableReference(e3)] => {
                match (e1.borrow().clone(), e3.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e3))) => {
                    out[0] = e0.borrow().clone();
                    out[2] = e2.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSM1SM1 { e0: e1.clone(), e1: e3.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }

              // s m1 m1 m1
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1), Value::MutableReference(e2), Value::MutableReference(e3)] => {
                match (e1.borrow().clone(), e2.borrow().clone(), e3.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e3))) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSM1M1M1 { e0: e1.clone(), e1: e2.clone(), e2: e3.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }

              // m1 s m1 m1
              [Value::MutableReference(e0), Value::[<$kind:camel>](e1), Value::MutableReference(e2), Value::MutableReference(e3)] => {
                match (e0.borrow().clone(), e2.borrow().clone(), e3.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e3))) => {
                    out[1] = e1.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateM1SM1M1 { e0: e0.clone(), e1: e2.clone(), e2: e3.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }

              // m1 m1 s m1
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::[<$kind:camel>](e2), Value::MutableReference(e3)] => {
                match (e0.borrow().clone(), e1.borrow().clone(), e3.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e3))) => {
                    out[2] = e2.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateM1M1SM1 { e0: e0.clone(), e1: e1.clone(), e2: e3.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }

              // m1 m1 m1 s
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2), Value::[<$kind:camel>](e3)] => {
                match (e0.borrow().clone(), e1.borrow().clone(), e2.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    out[3] = e3.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateM1M1M1S { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }

              // m1 m1 m1 m1
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2), Value::MutableReference(e3)] => {
                match (e0.borrow().clone(), e1.borrow().clone(), e2.borrow().clone(), e3.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e3))) => {
                    return Ok(Box::new(HorizontalConcatenateM1M1M1M1 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "RowVectorD")]
          (4,1,n) => {
            let mut out = RowDVector::from_element(n,$default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2), Value::MutableReference(e3)] => {
                match (e0.borrow().clone(), e1.borrow().clone(), e2.borrow().clone(), e3.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](e0),Value::[<Matrix $kind:camel>](e1),Value::[<Matrix $kind:camel>](e2),Value::[<Matrix $kind:camel>](e3)) => {
                    return Ok(Box::new(HorizontalConcatenateRD4{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "RowVectorD")]
          (m,1,n) => {
            let mut out = RowDVector::from_element(n,$default);
            let mut matrix_args: Vec<(Matrix<$kind>,usize)> = vec![];
            let mut scalar_args: Vec<(Ref<$kind>,usize)> = vec![];
            let mut i = 0;
            for arg in arguments.iter() {
              match &arg {
                Value::[<$kind:camel>](e0) => {
                  scalar_args.push((e0.clone(),i));
                  i += 1;
                }
                Value::MutableReference(e0) => {
                  match e0.borrow().clone() {
                    Value::[<Matrix $kind:camel>](e0) => {
                      matrix_args.push((e0.clone(),i));
                      i += e0.shape()[0];
                    }
                    Value::[<$kind:camel>](e0) => {
                      scalar_args.push((e0.clone(),i));
                      i += 1;
                    }
                    _ => todo!(),
                  }
                }
                _ => todo!(),
              }
            }
            return Ok(Box::new(HorizontalConcatenateRDN{scalar: scalar_args, matrix: matrix_args, out: new_ref(out)}));
          }
          #[cfg(feature = "Vector2")]
          (1,2,1) => {
            // v2
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)) => {return Ok(Box::new(HorizontalConcatenateV2{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "Matrix2")]
          (1,2,2) => {
            // m2
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix2(ref e0)) => {return Ok(Box::new(HorizontalConcatenateM2{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "Matrix2x3")]
          (1,2,3) => {
            // m2x3
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix2x3(ref e0)) => {return Ok(Box::new(HorizontalConcatenateM2x3{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "Vector3")]
          (1,3,1) => {
            // v3
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e0)) => {return Ok(Box::new(HorizontalConcatenateV3{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "Matrix3x2")]
          (1,3,2) => {
            // m3x2
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix3x2(ref e0)) => {return Ok(Box::new(HorizontalConcatenateM3x2{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "Matrix3")]
          (1,3,3) => {
            // m3
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix3(ref e0)) => {return Ok(Box::new(HorizontalConcatenateM3{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "Vector4")]
          (1,4,1) => {
            // v4
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e0)) => {return Ok(Box::new(HorizontalConcatenateV4{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "Matrix4")]
          (1,4,4) => {
            // m4
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix4(ref e0)) => {return Ok(Box::new(HorizontalConcatenateM4{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "MatrixD")]
          (1,m,n) => {
            // md
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e0)) => {return Ok(Box::new(HorizontalConcatenateMD{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "Matrix2")]
          (2,2,2) => {
            let mut out = Matrix2::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  // v2v2
                  (Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1))) => {return Ok(Box::new(HorizontalConcatenateV2V2{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));}
                  _ => todo!(),
                }
              }  
              _ => todo!(),
            }
          }
          #[cfg(feature = "Matrix3x2")]
          (2,3,2) => {
            let mut out = Matrix3x2::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  // v3v3
                  (Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e1))) => {return Ok(Box::new(HorizontalConcatenateV3V3{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));}
                  _ => todo!(),
                }
              }  
              _ => todo!(),
            }
          }
          #[cfg(feature = "Matrix2x3")]
          (2,2,3) => {
            let mut out = Matrix2x3::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  // v2m2
                  (Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix2(ref e1))) => {return Ok(Box::new(HorizontalConcatenateV2M2{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));}
                  // m2v2
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix2(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1))) => {return Ok(Box::new(HorizontalConcatenateM2V2{e0: e0.clone(),e1: e1.clone(),out: new_ref(out),}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "Matrix3")]
          (2,3,3) => {
            let mut out = Matrix3::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  // v3m3x2
                  (Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix3x2(ref e1))) => {return Ok(Box::new(HorizontalConcatenateV3M3x2 { e0: e0.clone(), e1: e1.clone(), out: new_ref(out) }));}
                  // m3x2v3
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix3x2(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e1))) => {return Ok(Box::new(HorizontalConcatenateM3x2V3 { e0: e0.clone(), e1: e1.clone(), out: new_ref(out) }));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "Matrix4")]
          (2,4,4) => {
            let mut out = Matrix4::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  // v4md
                  (Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e0)),Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e1)))=>Ok(Box::new(HorizontalConcatenateV4MD{e0:e0.clone(),e1:e1.clone(),out:new_ref(out)})),
                  // mdv4
                  (Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e1)))=>Ok(Box::new(HorizontalConcatenateMDV4{e0:e0.clone(),e1:e1.clone(),out:new_ref(out)})),
                  // mdmd
                  (Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e0)),Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e1)))=>Ok(Box::new(HorizontalConcatenateMDMD{e0:e0.clone(),e1:e1.clone(),out:new_ref(out)})),
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "MatrixD")]
          (2,m,n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](m0),Value::[<Matrix $kind:camel>](m1)) => {
                    let e0 = m0.get_copyable_matrix();
                    let e1 = m1.get_copyable_matrix();
                    Ok(Box::new(HorizontalConcatenateTwoArgs{e0,e1,out:new_ref(out)}))
                  }   
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "Matrix2x3")]
          (3, 2, 3) => {
            let mut out = Matrix2x3::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(), e1.borrow().clone(),e2.borrow().clone()) {
                  // v2v2v2
                  (Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e2)))=>Ok(Box::new(HorizontalConcatenateV2V2V2{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:new_ref(out)})),
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "Matrix3")]
          (3, 3, 3) => {
            let mut out = Matrix3::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(), e1.borrow().clone(),e2.borrow().clone()) {
                  // v3v3v3
                  (Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e2)))=>Ok(Box::new(HorizontalConcatenateV3V3V3{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:new_ref(out)})),
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "Matrix4")]
          (3, 4, 4) => {
            let mut out = Matrix4::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(), e1.borrow().clone(),e2.borrow().clone()) {
                  // v4v4md
                  (Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e1)),Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e2)))=>Ok(Box::new(HorizontalConcatenateV4V4MD{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:new_ref(out)})),
                  // v4mdv4
                  (Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e0)),Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e2)))=>Ok(Box::new(HorizontalConcatenateV4MDV4{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:new_ref(out)})),
                  // mdv4v4
                  (Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e2)))=>Ok(Box::new(HorizontalConcatenateMDV4V4{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:new_ref(out)})),
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "MatrixD")]
          (3, m, n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(), e1.borrow().clone(),e2.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](m0),Value::[<Matrix $kind:camel>](m1),Value::[<Matrix $kind:camel>](m2)) => {
                    let e0 = m0.get_copyable_matrix();
                    let e1 = m1.get_copyable_matrix();
                    let e2 = m2.get_copyable_matrix();
                    Ok(Box::new(HorizontalConcatenateThreeArgs{e0,e1,e2,out:new_ref(out)}))
                  }   
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "Matrix4")]
          (4, 4, 4) => {
            let mut out = Matrix4::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2), Value::MutableReference(e3)] => {
                match (e0.borrow().clone(), e1.borrow().clone(),e2.borrow().clone(), e3.borrow().clone()) {
                  // v4v4v4v4
                  (Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e2)),Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e3)))=>Ok(Box::new(HorizontalConcatenateV4V4V4V4{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),e3:e3.clone(),out:new_ref(out)})),
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "MatrixD")]
          (4, m, n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2), Value::MutableReference(e3)] => {
                match (e0.borrow().clone(), e1.borrow().clone(),e2.borrow().clone(),e3.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](m0),Value::[<Matrix $kind:camel>](m1),Value::[<Matrix $kind:camel>](m2),Value::[<Matrix $kind:camel>](m3)) => {
                    let e0 = m0.get_copyable_matrix();
                    let e1 = m1.get_copyable_matrix();
                    let e2 = m2.get_copyable_matrix();
                    let e3 = m3.get_copyable_matrix();
                    Ok(Box::new(HorizontalConcatenateFourArgs{e0,e1,e2,e3,out:new_ref(out)}))
                  }   
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          #[cfg(feature = "MatrixD")]
          (l, m, n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            let mut args = vec![];
            for arg in arguments {
              match arg {
                Value::MutableReference(e0) => {
                  match e0.borrow().clone() {
                    Value::[<Matrix $kind:camel>](m0) => {
                      let e0 = m0.get_copyable_matrix();
                      args.push(e0);
                    }
                    _ => todo!(),
                  }
                }
                _ => todo!(),
              }
            }
            Ok(Box::new(HorizontalConcatenateNArgs{e0: args, out:new_ref(out)}))
          }
          _ => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind});}
        }
  }}}}}

fn impl_horzcat_fxn(arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  // are they all the same?
  //let same = kinds.iter().all(|x| *x == target_kind);
  let kinds: Vec<ValueKind> = arguments.iter().map(|x| x.kind()).collect::<Vec<ValueKind>>();
  let target_kind = kinds[0].clone();
  if ValueKind::is_compatible(target_kind.clone(), ValueKind::F64)  { impl_horzcat_arms!(F64,arguments,F64::zero())
  } else if ValueKind::is_compatible(target_kind.clone(), ValueKind::F32)  { impl_horzcat_arms!(F32,arguments,F32::zero())
  } else if ValueKind::is_compatible(target_kind.clone(), ValueKind::U8)  { impl_horzcat_arms!(u8,arguments,u8::zero())    
  } else if ValueKind::is_compatible(target_kind.clone(), ValueKind::U16)  { impl_horzcat_arms!(u16,arguments,u16::zero())    
  } else if ValueKind::is_compatible(target_kind.clone(), ValueKind::U32)  { impl_horzcat_arms!(u32,arguments,u32::zero())    
  } else if ValueKind::is_compatible(target_kind.clone(), ValueKind::U64)  { impl_horzcat_arms!(u64,arguments,u64::zero())    
  } else if ValueKind::is_compatible(target_kind.clone(), ValueKind::U128)  { impl_horzcat_arms!(u128,arguments,u128::zero())    
  } else if ValueKind::is_compatible(target_kind.clone(), ValueKind::Bool)  { impl_horzcat_arms!(bool,arguments,false)
  } else {
    todo!();
  }
}

pub struct MaxtrixHorzCat {}
impl NativeFunctionCompiler for MaxtrixHorzCat {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    // First, get the size of the output matrix
    // rows are consistent already so we can just get nrows from the first element
    impl_horzcat_fxn(arguments)
  }
}