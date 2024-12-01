use crate::stdlib::*;

// Horizontal Concatenate -----------------------------------------------------

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

#[macro_export]
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

#[derive(Debug)]
struct HorizontalConcatenateSR2<T> {
  el: Ref<RowVector2<T>>,
  out: Ref<RowVector3<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSR2<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let el_ptr = (*(self.el.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[1] = el_ptr[0].clone();
      out_ptr[2] = el_ptr[1].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateR2S<T> {
  el: Ref<RowVector2<T>>,
  out: Ref<RowVector3<T>>,
}
impl<T> MechFunction for HorizontalConcatenateR2S<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let el_ptr = (*(self.el.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = el_ptr[0].clone();
      out_ptr[1] = el_ptr[1].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateM1R2<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<RowVector2<T>>,
  out: Ref<RowVector3<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1R2<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e1_ptr[1].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateR2M1<T> {
  e0: Ref<RowVector2<T>>,
  e1: Ref<Matrix1<T>>,
  out: Ref<RowVector3<T>>,
}
impl<T> MechFunction for HorizontalConcatenateR2M1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e0_ptr[1].clone();
      out_ptr[2] = e1_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateM1<T> {
  out: Ref<Matrix1<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<Matrix1<T>>: ToValue
{
  fn solve(&self) { }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateSM1<T> {
  e0: Ref<Matrix1<T>>,
  out: Ref<RowVector2<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSM1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector2<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[1] = e0_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateM1S<T> {
  e0: Ref<Matrix1<T>>,
  out: Ref<RowVector2<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1S<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector2<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

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

macro_rules! horzcat_m1m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
  };}

macro_rules! horzcat_m1sm1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[2] = $e1[0].clone();
  };}

macro_rules! horzcat_sm1m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e1[0].clone();
  };}

macro_rules! horzcat_r2r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };}

macro_rules! horzcat_m1r3 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e1[0].clone();
    $out[2] = $e1[1].clone();
    $out[3] = $e1[2].clone();
  };}

macro_rules! horzcat_r3m1 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[1] = $e0[1].clone();
    $out[2] = $e0[2].clone();
    $out[3] = $e1[0].clone();
  };}

macro_rules! horzcat_sm1r2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[1] = $e0[0].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };}

macro_rules! horzcat_m1sr2 {
  ($out:expr, $e0:expr, $e1:expr) => {
    $out[0] = $e0[0].clone();
    $out[2] = $e1[0].clone();
    $out[3] = $e1[1].clone();
  };}
  


horzcat_two_args!(HorizontalConcatenateM1M1,Matrix1,Matrix1,RowVector2,horzcat_m1m1);
horzcat_two_args!(HorizontalConcatenateR2R2,RowVector2,RowVector2,RowVector4,horzcat_r2r2);
horzcat_two_args!(HorizontalConcatenateM1R3,Matrix1,RowVector3,RowVector4,horzcat_m1r3);
horzcat_two_args!(HorizontalConcatenateR3M1,RowVector3,Matrix1,RowVector4,horzcat_r3m1);
horzcat_two_args!(HorizontalConcatenateM1M1S,Matrix1,Matrix1,RowVector3,horzcat_m1m1);
horzcat_two_args!(HorizontalConcatenateM1SM1,Matrix1,Matrix1,RowVector3,horzcat_m1sm1);
horzcat_two_args!(HorizontalConcatenateSM1M1,Matrix1,Matrix1,RowVector3,horzcat_sm1m1);
horzcat_two_args!(HorizontalConcatenateSM1R2,Matrix1,RowVector2,RowVector4,horzcat_sm1r2);
horzcat_two_args!(HorizontalConcatenateM1SR2,Matrix1,RowVector2,RowVector4,horzcat_m1sr2);

#[derive(Debug)]
struct HorizontalConcatenateSR3<T> {
  e0: Ref<RowVector3<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSR3<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[1] = e0_ptr[0].clone();
      out_ptr[2] = e0_ptr[1].clone();
      out_ptr[3] = e0_ptr[2].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateR3S<T> {
  e0: Ref<RowVector3<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateR3S<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e0_ptr[1].clone();
      out_ptr[2] = e0_ptr[2].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateSSM1<T> {
  e0: Ref<Matrix1<T>>,
  out: Ref<RowVector3<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSSM1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[2] = e0_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateSM1S<T> {
  e0: Ref<Matrix1<T>>,
  out: Ref<RowVector3<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSM1S<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[1] = e0_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateM1SS<T> {
  e0: Ref<Matrix1<T>>,
  out: Ref<RowVector3<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1SS<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateM1M1M1<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<Matrix1<T>>,
  out: Ref<RowVector3<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1M1M1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e1.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateSSR2<T> {
  e0: Ref<RowVector2<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSSR2<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[2] = e0_ptr[0].clone();
      out_ptr[3] = e0_ptr[1].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateSR2S<T> {
  e0: Ref<RowVector2<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSR2S<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[1] = e0_ptr[0].clone();
      out_ptr[2] = e0_ptr[1].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateR2SS<T> {
  e0: Ref<RowVector2<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateR2SS<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e0_ptr[1].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateM1M1R2<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<RowVector2<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1M1R2<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_ptr[0].clone();
      out_ptr[3] = e2_ptr[1].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateM1R2M1<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<RowVector2<T>>,
  e2: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1R2M1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e1_ptr[1].clone();
      out_ptr[3] = e2_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateR2M1M1<T> {
  e0: Ref<RowVector2<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateR2M1M1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e0_ptr[1].clone();
      out_ptr[2] = e1_ptr[0].clone();
      out_ptr[3] = e2_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateM1R2S<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<RowVector2<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1R2S<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e1_ptr[1].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateR2M1S<T> {
  e0: Ref<RowVector2<T>>,
  e1: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateR2M1S<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e0_ptr[1].clone();
      out_ptr[2] = e1_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateR2SM1<T> {
  e0: Ref<RowVector2<T>>,
  e1: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateR2SM1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e0_ptr[1].clone();
      out_ptr[3] = e1_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateSR2M1<T> {
  e0: Ref<RowVector2<T>>,
  e1: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSR2M1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[1] = e0_ptr[0].clone();
      out_ptr[2] = e0_ptr[1].clone();
      out_ptr[3] = e1_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateSSSM1<T> {
  e0: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSSSM1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[3] = e0_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateSSM1S<T> {
  e0: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSSM1S<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[2] = e0_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateSM1SS<T> {
  e0: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSM1SS<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[1] = e0_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateM1SSS<T> {
  e0: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1SSS<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateSSM1M1<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSSM1M1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[2] = e0_ptr[0].clone();
      out_ptr[3] = e1_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateM1M1SS<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1M1SS<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateSM1M1S<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSM1M1S<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[1] = e0_ptr[0].clone();
      out_ptr[2] = e1_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateM1SSM1<T> {
  e0: Ref<Matrix1<T>>,
  e3: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1SSM1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e3_ptr = (*(self.e3.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[3] = e3_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateM1SM1S<T> {
  e0: Ref<Matrix1<T>>,
  e2: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1SM1S<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[2] = e2_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateSM1SM1<T> {
  e1: Ref<Matrix1<T>>,
  e3: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSM1SM1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e3_ptr = (*(self.e3.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[3] = e3_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateSM1M1M1<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateSM1M1M1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[1] = e0_ptr[0].clone();
      out_ptr[2] = e1_ptr[0].clone();
      out_ptr[3] = e2_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateM1SM1M1<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1SM1M1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[2] = e1_ptr[0].clone();
      out_ptr[3] = e2_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateM1M1SM1<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1M1SM1<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[3] = e2_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct HorizontalConcatenateM1M1M1S<T> {
  e0: Ref<Matrix1<T>>,
  e1: Ref<Matrix1<T>>,
  e2: Ref<Matrix1<T>>,
  out: Ref<RowVector4<T>>,
}
impl<T> MechFunction for HorizontalConcatenateM1M1M1S<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector4<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let e0_ptr = (*(self.e0.as_ptr())).clone();
      let e1_ptr = (*(self.e1.as_ptr())).clone();
      let e2_ptr = (*(self.e2.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      out_ptr[0] = e0_ptr[0].clone();
      out_ptr[1] = e1_ptr[0].clone();
      out_ptr[2] = e2_ptr[0].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

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


macro_rules! impl_horzcat_3args {
  ($fxn_name:ident, $m1:ident,$m2:ident,$m3:ident) => {
    paste!{
      #[derive(Debug)]
      struct $fxn_name<T> {
        e0: Ref<$m1<T>>,
        e1: Ref<$m2<T>>,
        e2: Ref<$m3<T>>,
        out: Ref<RowDVector<T>>,
      }
      impl<T> MechFunction for $fxn_name<T>
      where
        T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
        Ref<RowDVector<T>>: ToValue
      {
        fn solve(&self) { 
          unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
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
    }};}

impl_horzcat_3args!(HorizontalConcatenateM4M4M4,RowVector4,RowVector4,RowVector4);
impl_horzcat_3args!(HorizontalConcatenateM2M2M2,RowVector2,RowVector2,RowVector2);

macro_rules! impl_horzcat_arms {
  ($kind:ident, $args:expr, $default:expr) => {
    paste!{
    {
      let arguments = $args;   
      let rows = arguments[0].shape()[0];
      let columns:usize = arguments.iter().fold(0, |acc, x| acc + x.shape()[1]);
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
        match (nargs,columns) {
          (1,1) => {
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
          (1,2) => {
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
          (1,3) => {
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
          (1,4) => {
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
          (1,n) => {
            // rd
            todo!()
          }
          (2,2) => {
            let mut out = RowVector2::from_element($default);
            match &arguments[..] {
              // s1m1
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1)] => {
                match *e1.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1)) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSM1{e0: e1.clone(), out: new_ref(out)}));
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
                  _ => todo!(),
                }
              }              
              // m1m1
              [Value::MutableReference(e0), Value::MutableReference(e1)] => {
                match (e0.borrow().clone(), e1.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e0)),Value::[<Matrix $kind:camel>](Matrix::Matrix1(ref e1))) => {
                    return Ok(Box::new(HorizontalConcatenateM1M1{e0: e0.clone(), e1: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }      
              _ => todo!(),
            }
          }
          (2,3) => {
            let mut out = RowVector3::from_element($default);
            match &arguments[..] {
              //sr2
              [Value::[<$kind:camel>](e0), Value::MutableReference(e1)] => {
                match *e1.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1)) => {
                    out[0] = e0.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateSR2{el: e1.clone(), out: new_ref(out)}));
                  }
                  _ => todo!(),
                }
              }
              //r2s
              [Value::MutableReference(e0),Value::[<$kind:camel>](e1)] => {
                match *e0.borrow() {
                  Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)) => {
                    out[2] = e1.borrow().clone();
                    return Ok(Box::new(HorizontalConcatenateR2S{el: e0.clone(), out: new_ref(out)}));
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
          (2,4) => {
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
          (2,n) => {
            todo!()
          }
          (3,3) => {  
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
          (3,4) => {
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
          (3,n) => {
            let mut out = RowDVector::from_element(n,$default);
            match &arguments[..] {
              // m4 m4 m4
              [Value::MutableReference(e0), Value::MutableReference(e1), Value::MutableReference(e2)] => {
                match (e0.borrow().clone(),e1.borrow().clone(),e2.borrow().clone()) {
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e0)),
                   Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e1)),
                   Value::[<Matrix $kind:camel>](Matrix::RowVector4(ref e2))) => {
                    return Ok(Box::new(HorizontalConcatenateM4M4M4{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
                  }
                  (Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e0)),
                   Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e1)),
                   Value::[<Matrix $kind:camel>](Matrix::RowVector2(ref e2))) => {
                   return Ok(Box::new(HorizontalConcatenateM2M2M2{e0: e0.clone(), e1: e1.clone(), e2: e2.clone(), out: new_ref(out)}));
                 }
                  _ => todo!(),
                }
              }
              _ => todo!(),
            }
          }
          (4,4) => {
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
                    return Ok(Box::new(HorizontalConcatenateM1SSM1 { e0: e0.clone(), e3: e3.clone(), out: new_ref(out) }));
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
                    return Ok(Box::new(HorizontalConcatenateM1SM1S { e0: e0.clone(), e2: e2.clone(), out: new_ref(out) }));
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
                    return Ok(Box::new(HorizontalConcatenateSM1SM1 { e1: e1.clone(), e3: e3.clone(), out: new_ref(out) }));
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
          (4,n) => {
            todo!()
          }
          //(m,n) => todo!()
          _ => {return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind});}
        }
      //}
  }}}}}

fn impl_horzcat_fxn(arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
  println!("{:?}", arguments);

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