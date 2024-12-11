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

struct VerticalConcatenateTwoArgs<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  out: Ref<DMatrix<T>>,
}
impl<T> MechFunction for VerticalConcatenateTwoArgs<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DMatrix<T>>: ToValue
{
  fn solve(&self) {
    let offset = self.e0.copy_into(&self.out,0);
    self.e1.copy_into(&self.out,offset);
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("VerticalConcatenateTwoArgs") }
}
    
struct VerticalConcatenateThreeArgs<T> {
  e0: Box<dyn CopyMat<T>>,
  e1: Box<dyn CopyMat<T>>,
  e2: Box<dyn CopyMat<T>>,
  out: Ref<DMatrix<T>>,
}
impl<T> MechFunction for VerticalConcatenateThreeArgs<T>
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
  fn to_string(&self) -> String { format!("VerticalConcatenateThreeArgs") }
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
  fn to_string(&self) -> String { format!("VerticalConcatenateFourArgs") }
}

struct VerticalConcatenateNArgs<T> {
  e0: Vec<Box<dyn CopyMat<T>>>,
  out: Ref<DMatrix<T>>,
}
impl<T> MechFunction for VerticalConcatenateNArgs<T>
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
  fn to_string(&self) -> String { format!("VerticalConcatenateNArgs") }
}

macro_rules! vertical_concatenate {
  ($name:ident, $vec_size:expr) => {
    paste!{
      #[derive(Debug)]
      struct $name<T> {
        out: Ref<[<Vector $vec_size>]<T>>,
      }

      impl<T> MechFunction for $name<T> 
      where
        T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
        Ref<[<Vector $vec_size>]<T>>: ToValue
      {
        fn solve(&self) {}
        fn out(&self) -> Value { self.out.to_value() }
        fn to_string(&self) -> String { format!("{:?}", self) }
      }
    }
  };}  

#[derive(Debug)]
struct VerticalConcatenateRD<T> {
  out: Ref<DVector<T>>,
}

impl<T> MechFunction for VerticalConcatenateRD<T> 
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
{
  fn solve(&self) {}
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct VerticalConcatenateRD2<T> {
  e0: Matrix<T>,
  e1: Matrix<T>,
  out: Ref<DVector<T>>,
}

impl<T> MechFunction for VerticalConcatenateRD2<T> 
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
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
struct VerticalConcatenateRD3<T> {
  e0: Matrix<T>,
  e1: Matrix<T>,
  e2: Matrix<T>,
  out: Ref<DVector<T>>,
}

impl<T> MechFunction for VerticalConcatenateRD3<T> 
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
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
struct VerticalConcatenateRD4<T> {
  e0: Matrix<T>,
  e1: Matrix<T>,
  e2: Matrix<T>,
  e3: Matrix<T>,
  out: Ref<DVector<T>>,
}

impl<T> MechFunction for VerticalConcatenateRD4<T> 
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
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
struct VerticalConcatenateRDN<T> {
  scalar: Vec<(Ref<T>,usize)>,
  matrix: Vec<(Matrix<T>,usize)>,
  out: Ref<DVector<T>>,
}

impl<T> MechFunction for VerticalConcatenateRDN<T> 
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
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
struct VerticalConcatenateS1<T> {
  out: Ref<Matrix1<T>>,
}

impl<T> MechFunction for VerticalConcatenateS1<T> 
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<Matrix1<T>>: ToValue
{
  fn solve(&self) {}
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

vertical_concatenate!(VerticalConcatenateS2,2);
vertical_concatenate!(VerticalConcatenateS3,3);
vertical_concatenate!(VerticalConcatenateS4,4);
vertical_concatenate!(VerticalConcatenateR2,2);
vertical_concatenate!(VerticalConcatenateR3,3);
vertical_concatenate!(VerticalConcatenateR4,4);

#[derive(Debug)]
struct VerticalConcatenateSD<T> {
  out: Ref<DVector<T>>,
}
impl<T> MechFunction for VerticalConcatenateSD<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<DVector<T>>: ToValue
{
  fn solve(&self) { }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

macro_rules! vertcat_single {
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

vertcat_single!(VerticalConcatenateM1,Matrix1);
vertcat_single!(VerticalConcatenateM2,Matrix2);
vertcat_single!(VerticalConcatenateM3,Matrix3);
vertcat_single!(VerticalConcatenateM4,Matrix4);
vertcat_single!(VerticalConcatenateM2x3,Matrix2x3);
vertcat_single!(VerticalConcatenateM3x2,Matrix3x2);
vertcat_single!(VerticalConcatenateMD,DMatrix);
vertcat_single!(VerticalConcatenateV2,Vector2);
vertcat_single!(VerticalConcatenateV3,Vector3);
vertcat_single!(VerticalConcatenateV4,Vector4);
vertcat_single!(VerticalConcatenateVD,DVector);

macro_rules! vertcat_sr2 {
  ($out:expr, $e0:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e0[1].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateSR2, Vector2, Vector3, vertcat_sr2);

macro_rules! vertcat_r2s {
  ($out:expr, $e0:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateR2S, Vector2, Vector3, vertcat_r2s);

macro_rules! vertcat_sm1 {
  ($out:expr, $e0:expr) => {
    $out[1] = $e0[0].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateSM1, Matrix1, Vector2, vertcat_sm1);

macro_rules! vertcat_m1s {
  ($out:expr, $e0:expr) => {
    $out[0] = $e0[0].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateM1S, Matrix1, Vector2, vertcat_m1s);

macro_rules! vertcat_sssm1 {
  ($out:expr, $e0:expr) => {
    $out[3] = $e0[0].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateSSSM1, Matrix1, Vector4, vertcat_sssm1);

macro_rules! vertcat_ssm1s {
  ($out:expr, $e0:expr) => {
    $out[2] = $e0[0].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateSSM1S, Matrix1, Vector4, vertcat_ssm1s);

macro_rules! vertcat_sm1ss {
  ($out:expr, $e0:expr) => {
    $out[1] = $e0[0].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateSM1SS, Matrix1, Vector4, vertcat_sm1ss);

macro_rules! vertcat_m1sss {
  ($out:expr, $e0:expr) => {
    $out[0] = $e0[0].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateM1SSS, Matrix1, Vector4, vertcat_m1sss);

macro_rules! vertcat_sr3 {
  ($out:expr, $e0:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e0[1].clone();
    $out[3] = $e0[2].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateSR3, Vector3, Vector4, vertcat_sr3);

macro_rules! vertcat_r3s {
  ($out:expr, $e0:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateR3S, Vector3, Vector4, vertcat_r3s);

macro_rules! vertcat_ssm1 {
  ($out:expr, $e0:expr) => {
    $out[2] = $e0[0].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateSSM1, Matrix1, Vector3, vertcat_ssm1);

macro_rules! vertcat_sm1s {
  ($out:expr, $e0:expr) => {
    $out[1] = $e0[0].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateSM1S, Matrix1, Vector3, vertcat_sm1s);

macro_rules! vertcat_m1ss {
  ($out:expr, $e0:expr) => {
    $out[0] = $e0[0].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateM1SS, Matrix1, Vector3, vertcat_m1ss);

macro_rules! vertcat_ssr2 {
  ($out:expr, $e0:expr) => {
    $out[2] = $e0[0].clone();
    $out[3] = $e0[1].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateSSR2, Vector2, Vector4, vertcat_ssr2);

macro_rules! vertcat_sr2s {
  ($out:expr, $e0:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e0[1].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateSR2S, Vector2, Vector4, vertcat_sr2s);

macro_rules! vertcat_r2ss {
  ($out:expr, $e0:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
  };
}
vertcat_one_arg!(VerticalConcatenateR2SS, Vector2, Vector4, vertcat_r2ss);

macro_rules! vertcat_m1m1s {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
  };}
vertcat_two_args!(VerticalConcatenateM1M1S,Matrix1,Matrix1,Vector3,vertcat_m1m1s);

macro_rules! vertcat_m1m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
  };}
vertcat_two_args!(VerticalConcatenateM1M1,Matrix1,Matrix1,Vector2,vertcat_m1m1);

macro_rules! vertcat_m1sm1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[2] = $e1[0].clone();
  };}
vertcat_two_args!(VerticalConcatenateM1SM1,Matrix1,Matrix1,Vector3,vertcat_m1sm1);

macro_rules! vertcat_sm1m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e1[0].clone();
  };}
vertcat_two_args!(VerticalConcatenateSM1M1,Matrix1,Matrix1,Vector3,vertcat_sm1m1);

macro_rules! vertcat_r2r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };}
vertcat_two_args!(VerticalConcatenateR2R2,Vector2,Vector2,Vector4,vertcat_r2r2);

macro_rules! vertcat_m1r3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
    $out[3] = $e1[2].clone();
  };}
vertcat_two_args!(VerticalConcatenateM1R3,Matrix1,Vector3,Vector4,vertcat_m1r3);

macro_rules! vertcat_r3m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e1[0].clone();
  };}
vertcat_two_args!(VerticalConcatenateR3M1,Vector3,Matrix1,Vector4,vertcat_r3m1);

macro_rules! vertcat_sm1r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };}
vertcat_two_args!(VerticalConcatenateSM1R2,Matrix1,Vector2,Vector4,vertcat_sm1r2);

macro_rules! vertcat_m1sr2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };}
vertcat_two_args!(VerticalConcatenateM1SR2,Matrix1,Vector2,Vector4,vertcat_m1sr2);
  
macro_rules! vertcat_sm1sm1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[1] = $e0[0].clone();
    $out[3] = $e1[0].clone();
  };} 
vertcat_two_args!(VerticalConcatenateSM1SM1,Matrix1,Matrix1,Vector4,vertcat_sm1sm1);

macro_rules! vertcat_m1r2s {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
  };} 
vertcat_two_args!(VerticalConcatenateM1R2S,Matrix1,Vector2,Vector4,vertcat_m1r2s);

macro_rules! vertcat_r2m1s {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
  };} 
vertcat_two_args!(VerticalConcatenateR2M1S,Vector2,Matrix1,Vector4,vertcat_r2m1s);

macro_rules! vertcat_r2sm1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[3] = $e1[0].clone();
  };
}
vertcat_two_args!(VerticalConcatenateR2SM1, Vector2, Matrix1, Vector4, vertcat_r2sm1);

macro_rules! vertcat_sr2m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e0[1].clone();
    $out[3] = $e1[0].clone();
  };
}
vertcat_two_args!(VerticalConcatenateSR2M1, Vector2, Matrix1, Vector4, vertcat_sr2m1);

macro_rules! vertcat_ssm1m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[2] = $e0[0].clone();
    $out[3] = $e1[0].clone();
  };
}
vertcat_two_args!(VerticalConcatenateSSM1M1, Matrix1, Matrix1, Vector4, vertcat_ssm1m1);

macro_rules! vertcat_m1m1ss {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
  };
}
vertcat_two_args!(VerticalConcatenateM1M1SS, Matrix1, Matrix1, Vector4, vertcat_m1m1ss);

macro_rules! vertcat_sm1m1s {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e1[0].clone();
  };
}
vertcat_two_args!(VerticalConcatenateSM1M1S, Matrix1, Matrix1, Vector4, vertcat_sm1m1s);

macro_rules! vertcat_m1ssm1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[3] = $e1[0].clone();
  };
}
vertcat_two_args!(VerticalConcatenateM1SSM1, Matrix1, Matrix1, Vector4, vertcat_m1ssm1);

macro_rules! vertcat_m1sm1s {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[2] = $e1[0].clone();
  };
}
vertcat_two_args!(VerticalConcatenateM1SM1S, Matrix1, Matrix1, Vector4, vertcat_m1sm1s);

macro_rules! vertcat_m1r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
  };
}
vertcat_two_args!(VerticalConcatenateM1R2, Matrix1, Vector2, Vector3, vertcat_m1r2);

macro_rules! vertcat_r2m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
  };
}
vertcat_two_args!(VerticalConcatenateR2M1, Vector2, Matrix1, Vector3, vertcat_r2m1);

macro_rules! vertcat_m1m1m1 {
  ($out:expr, $e0:expr,$e1:expr,$e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e2[0].clone();
  };
}
vertcat_three_args!(VerticalConcatenateM1M1M1,Matrix1,Matrix1,Matrix1,Vector3, vertcat_m1m1m1);

macro_rules! vertcat_m1m1r2 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e2[0].clone();
    $out[3] = $e2[1].clone();
  };
}
vertcat_three_args!(VerticalConcatenateM1M1R2, Matrix1, Matrix1, Vector2, Vector4, vertcat_m1m1r2);

macro_rules! vertcat_m1r2m1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
    $out[3] = $e2[0].clone();
  };
}
vertcat_three_args!(VerticalConcatenateM1R2M1, Matrix1, Vector2, Matrix1, Vector4, vertcat_m1r2m1);

macro_rules! vertcat_r2m1m1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e2[0].clone();
  };
}
vertcat_three_args!(VerticalConcatenateR2M1M1, Vector2, Matrix1, Matrix1, Vector4, vertcat_r2m1m1);

macro_rules! vertcat_sm1m1m1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e2[0].clone();
  };
}
vertcat_three_args!(VerticalConcatenateSM1M1M1, Matrix1, Matrix1, Matrix1, Vector4, vertcat_sm1m1m1);

macro_rules! vertcat_m1sm1m1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e2[0].clone();
  };
}
vertcat_three_args!(VerticalConcatenateM1SM1M1, Matrix1, Matrix1, Matrix1, Vector4, vertcat_m1sm1m1);

macro_rules! vertcat_m1m1sm1 {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[3] = $e2[0].clone();
  };
}
vertcat_three_args!(VerticalConcatenateM1M1SM1, Matrix1, Matrix1, Matrix1, Vector4, vertcat_m1m1sm1);

macro_rules! vertcat_m1m1m1s {
  ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e2[0].clone();
  };
}
vertcat_three_args!(VerticalConcatenateM1M1M1S, Matrix1, Matrix1, Matrix1, Vector4, vertcat_m1m1m1s);

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
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
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
  fn to_string(&self) -> String { format!("{:?}", self) }
}

macro_rules! vertcat_v2v2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };
}
vertcat_two_args!(VerticalConcatenateV2V2, RowVector2, RowVector2, Matrix2, vertcat_v2v2);

macro_rules! vertcat_v3v3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e1[0].clone();
    $out[4] = $e1[1].clone();
    $out[5] = $e1[2].clone();
  };
}
vertcat_two_args!(VerticalConcatenateV3V3, RowVector3, RowVector3, Matrix2x3, vertcat_v3v3);

macro_rules! vertcat_v2m2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
    $out[4] = $e1[2].clone();
    $out[5] = $e1[3].clone();
  };
}
vertcat_two_args!(VerticalConcatenateV2M2, RowVector2, Matrix2, Matrix3x2, vertcat_v2m2);

macro_rules! vertcat_m2v2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e0[3].clone();
    $out[4] = $e1[0].clone();
    $out[5] = $e1[1].clone();
  };
}
vertcat_two_args!(VerticalConcatenateM2V2, Matrix2, RowVector2, Matrix3x2, vertcat_m2v2);

macro_rules! vertcat_m3x2v3 {
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
vertcat_two_args!(VerticalConcatenateM2x3V3, Matrix2x3, RowVector3, Matrix3, vertcat_m3x2v3);

macro_rules! vertcat_v3m3x2 {
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
vertcat_two_args!(VerticalConcatenateV3M2x3, RowVector3, Matrix2x3, Matrix3, vertcat_v3m3x2);

macro_rules! vertcat_v4md {
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
vertcat_two_args!(VerticalConcatenateV4MD, RowVector4, DMatrix, Matrix4, vertcat_v4md);

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
vertcat_two_args!(VerticalConcatenateMDV4, DMatrix, RowVector4, Matrix4, vertcat_mdv4);

macro_rules! vertcat_mdmd {
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
vertcat_two_args!(VerticalConcatenateMDMD, DMatrix, DMatrix, Matrix4, vertcat_mdmd);

macro_rules! vertcat_mdmdmd {
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

vertcat_three_args!(VerticalConcatenateV2V2V2, RowVector2, RowVector2, RowVector2, Matrix2x3, vertcat_mdmdmd);
vertcat_three_args!(VerticalConcatenateV3V3V3, RowVector3, RowVector3, RowVector3, Matrix3, vertcat_mdmdmd);
vertcat_three_args!(VerticalConcatenateV4V4MD, RowVector4, RowVector4, DMatrix, Matrix4, vertcat_mdmdmd);
vertcat_three_args!(VerticalConcatenateV4MDV4, RowVector4, DMatrix, RowVector4, Matrix4, vertcat_mdmdmd);
vertcat_three_args!(VerticalConcatenateMDV4V4, DMatrix, RowVector4, RowVector4, Matrix4, vertcat_mdmdmd);


macro_rules! vertcat_mdmdmdmd {
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

vertcat_four_args!(VerticalConcatenateV4V4V4V4, RowVector4, RowVector4, RowVector4, RowVector4, Matrix4, vertcat_mdmdmdmd);

macro_rules! impl_vertcat_arms {
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
          [e0]             => {return Ok(Box::new(VerticalConcatenateS1{out:new_ref(Matrix1::from_vec(mat))}));}
          [e0, e1]         => {return Ok(Box::new(VerticalConcatenateS2{out:new_ref(Vector2::from_vec(mat))}));}
          [e0, e1, e2]     => {return Ok(Box::new(VerticalConcatenateS3{out:new_ref(Vector3::from_vec(mat))}));}
          [e0, e1, e2, e3] => {return Ok(Box::new(VerticalConcatenateS4{out:new_ref(Vector4::from_vec(mat))}));}
          _ => {return Ok(Box::new(VerticalConcatenateSD{out:new_ref(DVector::from_vec(mat))}));}
        }      
      } else {
        match (nargs,rows,columns) {
          (1,1,1) => {
            let mut out = Matrix1::from_element($default);
            match &arguments[..] {
              // m1
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)) => {
                    return Ok(Box::new(VerticalConcatenateM1{out: e0.clone()}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (1,1,2) => {
            let mut out = Vector2::from_element($default);
            match &arguments[..] {
              // r2
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)) => {
                    return Ok(Box::new(VerticalConcatenateR2{out: e0.clone()}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (1,1,3) => {
            let mut out = Vector3::from_element($default);
            match &arguments[..] {
              // r3
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e0)) => {
                    return Ok(Box::new(VerticalConcatenateR3{out: e0.clone()}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (1,1,4) => {
            let mut out = Vector4::from_element($default);
            match &arguments[..] {
              // r4
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e0)) => {
                    return Ok(Box::new(VerticalConcatenateR4{out: e0.clone()}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (1,1,n) => {
            let mut out = Vector4::from_element($default);
            match &arguments[..] {
              // rd
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::DVector(ref e0)) => {
                    return Ok(Box::new(VerticalConcatenateRD{out: e0.clone()}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (2,1,2) => {
            let mut out = Vector2::from_element($default);
            match &arguments[..] {
              // s1m1
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1)] => {
                match *e1.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateSM1{e0: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              // m1s1
              [Value::MutableReference(e0), Value::[<$kind:camel>](e1)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)) => {
                    out[1] = e1.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateM1S{e0: e0.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }              
              // m1m1
              [Value::MutableReference(e0), Value::MutableReference(e1)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))) => {
                    return Ok(Box::new(VerticalConcatenateM1M1{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }      
              _ => todo!(),
            }
          }
          (2,1,3) => {
            let mut out = Vector3::from_element($default);
            match &arguments[..] {
              //sr2
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1)] => {
                match *e1.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1)) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateSR2{e0: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              //r2s
              [Value::MutableReference(e0),Value::[<$kind:camel>](e1)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)) => {
                    out[2] = e1.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateR2S{e0: e0.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              [Value::MutableReference(e0),Value::MutableReference(e1)] => {
                match (&*e0.borrow(),&*e1.borrow()) {
                  //m1r2
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1))) => {
                    return Ok(Box::new(VerticalConcatenateM1R2{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  //r2m1
                  (Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))) => {
                    return Ok(Box::new(VerticalConcatenateR2M1{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (2,1,4) => {
            let mut out = Vector4::from_element($default);
            match &arguments[..] {
              // s r3
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1)] => {
                match *e1.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e1)) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateSR3{e0: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              // r3 s
              [Value::MutableReference(e0),Value::[<$kind:camel>](e1)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e0)) => {
                    out[3] = e1.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateR3S{e0: e0.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              [Value::MutableReference(e0),Value::MutableReference(e1)] => {
                match (&*e0.borrow(),&*e1.borrow()) {
                  // m1 r3
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e1))) => {
                    return Ok(Box::new(VerticalConcatenateM1R3{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  // r3 m1
                  (Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))) => {
                    return Ok(Box::new(VerticalConcatenateR3M1{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  // r2 r2
                  (Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1))) => {
                    return Ok(Box::new(VerticalConcatenateR2R2{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          } 
          (2,1,n) => {
            let mut out = DVector::from_element(n,$default);
            match &arguments[..] {
              [Value::MutableReference(e0),Value::MutableReference(e1)] => {
                match (&*e0.borrow(),&*e1.borrow()) {
                  (Value::[<Matrix $kind:camel>](e0),Value::[<Matrix $kind:camel>](e1)) => {
                    return Ok(Box::new(VerticalConcatenateRD2{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (3,1,3) => {  
            let mut out = Vector3::from_element($default);
            match &arguments[..] {
              // s s m1
              [Value::[<$kind:camel>](e0), Value::[<$kind:camel>](e1), Value::MutableReference(e2)] => {
                match *e2.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2)) => {
                    out[0] = e0.borrow().clone();
                    out[1] = e1.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateSSM1{e0: e2.clone(), out: new_ref(out)}));
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
                    return Ok(Box::new(VerticalConcatenateSM1S{e0: e1.clone(), out: new_ref(out)}));
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
                    return Ok(Box::new(VerticalConcatenateM1SS{e0: e0.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              // m1 m1 s
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::[<$kind:camel>](e2)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))) => {
                    out[2] = e2.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateM1M1S{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              // m1 s m1
              [Value::MutableReference(e0), Value::[<$kind:camel>](e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(), e2.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    out[1] = e1.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateM1SM1{e0: e0.clone(), e1: e2.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              // s m1 m1
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e1.borrow().clone(), e2.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateSM1M1{e0: e1.clone(), e1: e2.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }    
              // m1 m1 m1
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(), e1.borrow().clone(), e2.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    return Ok(Box::new(VerticalConcatenateM1M1M1{e0: e1.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }           
              _ => todo!()
            }
          }
          (3,1,4) => {
            let mut out = Vector4::from_element($default);
            match &arguments[..] {
              // s s r2
              [Value::[<$kind:camel>](e0), Value::[<$kind:camel>](e1), Value::MutableReference(e2)] => {
                match *e2.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e2)) => {
                    out[0] = e0.borrow().clone();
                    out[1] = e1.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateSSR2{e0: e2.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              // s r2 s1
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1), Value::[<$kind:camel>](e2)] => {
                match *e1.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1)) => {
                    out[0] = e0.borrow().clone();
                    out[3] = e2.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateSR2S{e0: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }      
              // r2 s s
              [Value::MutableReference(e0), Value::[<$kind:camel>](e1), Value::[<$kind:camel>](e2)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)) => {
                    out[2] = e1.borrow().clone();
                    out[3] = e2.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateR2SS{e0: e0.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }    
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(),e1.borrow().clone(),e2.borrow().clone()) {
                  // m1 m1 r2
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),
                   Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e2))) => {
                    return Ok(Box::new(VerticalConcatenateM1M1R2{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
                  }
                  // m1 r2 m1
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),
                   Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    return Ok(Box::new(VerticalConcatenateM1R2M1{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
                  }
                  // r2 m1 m1
                  (Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),
                   Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    return Ok(Box::new(VerticalConcatenateR2M1M1{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }        
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e1.borrow().clone(),e2.borrow().clone()) {
                  // s m1 r2
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e2))) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateSM1R2{e0: e1.clone(), e1: e2.clone(), out: new_ref(out)}));
                  }
                  // s r2 m1
                  (Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateSR2M1 { e0: e1.clone(), e1: e2.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              [Value::MutableReference(e0), Value::[<$kind:camel>](e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(), e2.borrow().clone()) {
                  // m1 s r2
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e2))) => {
                    out[1] = e1.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateM1SR2 { e0: e0.clone(), e1: e2.clone(), out: new_ref(out) }));
                  }
                  // r2 s m1
                  (Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2))) => {
                    out[2] = e1.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateR2SM1 { e0: e0.clone(), e1: e2.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::[<$kind:camel>](e2)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  // m1 r2 s
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e1))) => {
                    out[3] = e2.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateM1R2S { e0: e0.clone(), e1: e1.clone(), out: new_ref(out) }));
                  }
                  // r2 m1 s
                  (Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))) => {
                    out[3] = e2.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateR2M1S { e0: e0.clone(), e1: e1.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!()
            }
          }
          (3,1,n) => {
            let mut out = DVector::from_element(n,$default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(),e1.borrow().clone(),e2.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](e0),Value::[<Matrix $kind:camel>](e1),Value::[<Matrix $kind:camel>](e2)) => {
                    return Ok(Box::new(VerticalConcatenateRD3{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (4,1,4) => {
            let mut out = Vector4::from_element($default);
            match &arguments[..] {
             // s s s m1
              [Value::[<$kind:camel>](e0), Value::[<$kind:camel>](e1), Value::[<$kind:camel>](e2), Value::MutableReference(e3)] => {
                match (e3.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e3))) => {
                    out[0] = e0.borrow().clone();
                    out[1] = e1.borrow().clone();
                    out[2] = e2.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateSSSM1 { e0: e3.clone(), out: new_ref(out) }));
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
                    return Ok(Box::new(VerticalConcatenateSSM1S { e0: e2.clone(), out: new_ref(out) }));
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
                    return Ok(Box::new(VerticalConcatenateSM1SS { e0: e1.clone(), out: new_ref(out) }));
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
                    return Ok(Box::new(VerticalConcatenateM1SSS { e0: e0.clone(), out: new_ref(out) }));
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
                    return Ok(Box::new(VerticalConcatenateSSM1M1 { e0: e2.clone(), e1: e3.clone(), out: new_ref(out) }));
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
                    return Ok(Box::new(VerticalConcatenateM1M1SS { e0: e0.clone(), e1: e1.clone(), out: new_ref(out) }));
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
                    return Ok(Box::new(VerticalConcatenateSM1M1S { e0: e1.clone(), e1: e2.clone(), out: new_ref(out) }));
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
                    return Ok(Box::new(VerticalConcatenateM1SSM1 { e0: e0.clone(), e1: e3.clone(), out: new_ref(out) }));
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
                    return Ok(Box::new(VerticalConcatenateM1SM1S { e0: e0.clone(), e1: e2.clone(), out: new_ref(out) }));
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
                    return Ok(Box::new(VerticalConcatenateSM1SM1 { e0: e1.clone(), e1: e3.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }

              // s m1 m1 m1
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1), Value::MutableReference(e2), Value::MutableReference(e3)] => {
                match (e1.borrow().clone(), e2.borrow().clone(), e3.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e2)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e3))) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(VerticalConcatenateSM1M1M1 { e0: e1.clone(), e1: e2.clone(), e2: e3.clone(), out: new_ref(out) }));
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
                    return Ok(Box::new(VerticalConcatenateM1SM1M1 { e0: e0.clone(), e1: e2.clone(), e2: e3.clone(), out: new_ref(out) }));
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
                    return Ok(Box::new(VerticalConcatenateM1M1SM1 { e0: e0.clone(), e1: e1.clone(), e2: e3.clone(), out: new_ref(out) }));
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
                    return Ok(Box::new(VerticalConcatenateM1M1M1S { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out) }));
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
                    return Ok(Box::new(VerticalConcatenateM1M1M1M1 { e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: new_ref(out) }));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (4,1,n) => {
            let mut out = DVector::from_element(n,$default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2), Value::MutableReference(e3)] => {
                match (e0.borrow().clone(), e1.borrow().clone(), e2.borrow().clone(), e3.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](e0),Value::[<Matrix $kind:camel>](e1),Value::[<Matrix $kind:camel>](e2),Value::[<Matrix $kind:camel>](e3)) => {
                    return Ok(Box::new(VerticalConcatenateRD4{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), e3: e3.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (m,1,n) => {
            let mut out = DVector::from_element(n,$default);
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
            return Ok(Box::new(VerticalConcatenateRDN{scalar: scalar_args, matrix: matrix_args, out: new_ref(out)}));
          }
          (1,2,1) => {
            // v2
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector2(ref e0)) => {return Ok(Box::new(VerticalConcatenateV2{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (1,2,2) => {
            // m2
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix2(ref e0)) => {return Ok(Box::new(VerticalConcatenateM2{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (1,2,3) => {
            // m2x3
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix2x3(ref e0)) => {return Ok(Box::new(VerticalConcatenateM2x3{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (1,3,1) => {
            // v3
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector3(ref e0)) => {return Ok(Box::new(VerticalConcatenateV3{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (1,3,2) => {
            // m3x2
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix3x2(ref e0)) => {return Ok(Box::new(VerticalConcatenateM3x2{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (1,3,3) => {
            // m3
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix3(ref e0)) => {return Ok(Box::new(VerticalConcatenateM3{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (1,4,1) => {
            // v4
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Vector4(ref e0)) => {return Ok(Box::new(VerticalConcatenateV4{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (1,4,4) => {
            // m4
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix4(ref e0)) => {return Ok(Box::new(VerticalConcatenateM4{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (1,m,n) => {
            // md
            match &arguments[..] {
              [Value::MutableReference(e0)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e0)) => {return Ok(Box::new(VerticalConcatenateMD{out: e0.clone()}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (2, 2, 2) => {
            let mut out = Matrix2::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  // v2v2
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1))) => {return Ok(Box::new(VerticalConcatenateV2V2{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));}
                  _ => todo!(),
                }
              }  
              _ => todo!(),
            }
          }
          (2, 3, 2) => {
            let mut out = Matrix2x3::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  // v3v3
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e1))) => {return Ok(Box::new(VerticalConcatenateV3V3{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));}
                  _ => todo!(),
                }
              }  
              _ => todo!(),
            }
          }
          (2, 3, 2) => {
            let mut out = Matrix3x2::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  // v2m2
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix2(ref e1))) => {return Ok(Box::new(VerticalConcatenateV2M2{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));}
                  // m2v2
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix2(ref e0)), Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1))) => {return Ok(Box::new(VerticalConcatenateM2V2{e0: e0.clone(),e1: e1.clone(),out: new_ref(out),}));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (2, 3, 3) => {
            let mut out = Matrix3::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  // v3m3x2
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e0)), Value::[<Matrix $kind:camel>](Matrix::Matrix2x3(ref e1))) => {return Ok(Box::new(VerticalConcatenateV3M2x3 { e0: e0.clone(), e1: e1.clone(), out: new_ref(out) }));}
                  // m3x2v3
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix2x3(ref e0)), Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e1))) => {return Ok(Box::new(VerticalConcatenateM2x3V3 { e0: e0.clone(), e1: e1.clone(), out: new_ref(out) }));}
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (2, 4, 4) => {
            let mut out = Matrix4::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  // v4md
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e0)),Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e1)))=>Ok(Box::new(VerticalConcatenateV4MD{e0:e0.clone(),e1:e1.clone(),out:new_ref(out)})),
                  // mdv4
                  (Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e1)))=>Ok(Box::new(VerticalConcatenateMDV4{e0:e0.clone(),e1:e1.clone(),out:new_ref(out)})),
                  // mdmd
                  (Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e0)),Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e1)))=>Ok(Box::new(VerticalConcatenateMDMD{e0:e0.clone(),e1:e1.clone(),out:new_ref(out)})),
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (2, m, n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](m0),Value::[<Matrix $kind:camel>](m1)) => {
                    let e0 = m0.get_copyable_matrix();
                    let e1 = m1.get_copyable_matrix();
                    Ok(Box::new(VerticalConcatenateTwoArgs{e0,e1,out:new_ref(out)}))
                  }   
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (3, 2, 3) => {
            let mut out = Matrix2x3::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(), e1.borrow().clone(),e2.borrow().clone()) {
                  // v2v2v2
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e2)))=>Ok(Box::new(VerticalConcatenateV2V2V2{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:new_ref(out)})),
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (3, 3, 3) => {
            let mut out = Matrix3::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(), e1.borrow().clone(),e2.borrow().clone()) {
                  // v3v3v3
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector3(ref e2)))=>Ok(Box::new(VerticalConcatenateV3V3V3{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:new_ref(out)})),
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (3, 4, 4) => {
            let mut out = Matrix4::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(), e1.borrow().clone(),e2.borrow().clone()) {
                  // v4v4md
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e1)),Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e2)))=>Ok(Box::new(VerticalConcatenateV4V4MD{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:new_ref(out)})),
                  // v4mdv4
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e0)),Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e2)))=>Ok(Box::new(VerticalConcatenateV4MDV4{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:new_ref(out)})),
                  // mdv4v4
                  (Value::[<Matrix $kind:camel>](Matrix::DMatrix(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e2)))=>Ok(Box::new(VerticalConcatenateMDV4V4{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),out:new_ref(out)})),
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (3, m, n) => {
            let mut out = DMatrix::from_element(m,n,$default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(), e1.borrow().clone(),e2.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](m0),Value::[<Matrix $kind:camel>](m1),Value::[<Matrix $kind:camel>](m2)) => {
                    let e0 = m0.get_copyable_matrix();
                    let e1 = m1.get_copyable_matrix();
                    let e2 = m2.get_copyable_matrix();
                    Ok(Box::new(VerticalConcatenateThreeArgs{e0,e1,e2,out:new_ref(out)}))
                  }   
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (4, 4, 4) => {
            let mut out = Matrix4::from_element($default);
            match &arguments[..] {
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2), Value::MutableReference(e3)] => {
                match (e0.borrow().clone(), e1.borrow().clone(),e2.borrow().clone(), e3.borrow().clone()) {
                  // v4v4v4v4
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e0)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e1)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e2)),Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e3)))=>Ok(Box::new(VerticalConcatenateV4V4V4V4{e0:e0.clone(),e1:e1.clone(),e2:e2.clone(),e3:e3.clone(),out:new_ref(out)})),
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
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
                    Ok(Box::new(VerticalConcatenateFourArgs{e0,e1,e2,e3,out:new_ref(out)}))
                  }   
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
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
            Ok(Box::new(VerticalConcatenateNArgs{e0: args, out:new_ref(out)}))
          }
          _ => {return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind});}
        }
  }}}}}

fn impl_vertcat_fxn(arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  // are they all the same?
  //let same = kinds.iter().all(|x| *x == target_kind);
  let kinds: Vec<ValueKind> = arguments.iter().map(|x| x.kind()).collect::<Vec<ValueKind>>();
  let target_kind = kinds[0].clone();
  if ValueKind::is_compatible(target_kind.clone(), ValueKind::F64)  { impl_vertcat_arms!(F64,arguments,F64::zero())
  } else if ValueKind::is_compatible(target_kind.clone(), ValueKind::F32)  { impl_vertcat_arms!(F32,arguments,F32::zero())
  } else if ValueKind::is_compatible(target_kind.clone(), ValueKind::U8)  { impl_vertcat_arms!(u8,arguments,u8::zero())    
  } else if ValueKind::is_compatible(target_kind.clone(), ValueKind::U16)  { impl_vertcat_arms!(u16,arguments,u16::zero())    
  } else if ValueKind::is_compatible(target_kind.clone(), ValueKind::U32)  { impl_vertcat_arms!(u32,arguments,u32::zero())    
  } else if ValueKind::is_compatible(target_kind.clone(), ValueKind::U64)  { impl_vertcat_arms!(u64,arguments,u64::zero())    
  } else if ValueKind::is_compatible(target_kind.clone(), ValueKind::U128)  { impl_vertcat_arms!(u128,arguments,u128::zero())    
  } else if ValueKind::is_compatible(target_kind.clone(), ValueKind::Bool)  { impl_vertcat_arms!(bool,arguments,false)
  } else {
    todo!();
  }
}

pub struct MaxtrixVertCat {}
impl NativeFunctionCompiler for MaxtrixVertCat {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    // First, get the size of the output matrix
    // rows are consistent already so we can just get nrows from the first element
    impl_vertcat_fxn(arguments)
  }
}