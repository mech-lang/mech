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
            // s1
            // m1
            todo!()
          }
          (1,2) => {
            // r2
            todo!()
          }
          (1,3) => {
            // r3
            todo!()
          }
          (1,4) => {
            // r4
            todo!()
          }
          (1,n) => {
            // rd
            todo!()
          }
          (2,2) => {
            // s1s1
            // s1m1
            // m1s1
            // m1m1
            todo!()
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
            // s1 r3
            // r3 s1
            // r2 r2
            // m1 r3
            // r3 m1
            todo!()
          } 
          (2,n) => {
            todo!()
          }
          (3,3) => {  
            // s1 s1 s1
            // s1 s1 m1
            // s1 m1 s1
            // s1 m1 m1
            // m1 s1 s1
            // m1 s1 m1
            // m1 m1 s1
            // m1 m1 m1
            todo!()
          }
          (3,4) => {
            // s1 s1 r2
            // s1 m1 r2
            // s1 r2 s1
            // s1 r2 m1
            // m1 s1 r2
            // m1 m1 r2
            // m1 r2 s1
            // m1 r2 m1
            // r2 s1 s1
            // r2 s1 m1
            // r2 m1 s1
            // r2 m1 m1
            todo!()
          }
          (3,n) => {
            todo!()
          }
          (4,4) => {
            // s1 s1 s1 m1
            // s1 s1 m1 s1
            // s1 s1 m1 m1
            // s1 m1 s1 s1
            // s1 m1 s1 m1
            // s1 m1 m1 s1
            // s1 m1 m1 m1
            // m1 s1 s1 s1
            // m1 s1 s1 m1
            // m1 s1 m1 s1
            // m1 s1 m1 m1
            // m1 m1 s1 s1
            // m1 m1 s1 m1
            // m1 m1 m1 s1
            // m1 m1 m1 m1
            todo!()
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