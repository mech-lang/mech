use crate::stdlib::*;

// Horizontal Concatenate -----------------------------------------------------

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
struct HorizontalConcatenateR2<T> {
  el: Ref<RowVector2<T>>,
  ix: usize,
  out: Ref<RowVector3<T>>,
}
impl<T> MechFunction for HorizontalConcatenateR2<T>
where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<RowVector3<T>>: ToValue
{
  fn solve(&self) { 
    unsafe {
      let el_ptr = (*(self.el.as_ptr())).clone();
      let mut out_ptr = (&mut *(self.out.as_ptr()));
      let ix = self.ix;
      out_ptr[0 + ix] = el_ptr[0].clone();
      out_ptr[1 + ix] = el_ptr[1].clone();
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

fn impl_horzcat_fxn(arguments: &Vec<Value>, rows: usize, columns: usize) -> Result<Box<dyn MechFunction>, MechError> {
  println!("{:?} {:?} {:?}", arguments, rows, columns);
  let nargs = arguments.len();
  let kinds: Vec<ValueKind> = arguments.iter().map(|x| x.kind()).collect::<Vec<ValueKind>>();
  let no_refs = !kinds.iter().any(|x| *x == ValueKind::Reference);

  
  
  let target_kind = kinds[0].clone();
  // are they all the same?
  //let same = kinds.iter().all(|x| *x == target_kind);
  
  if no_refs {
    let mat: Vec<F64> = arguments.iter().flat_map(|v| v.as_vecf64().unwrap()).collect::<Vec<F64>>();
    match &mat[..] {
      [e0, e1]         => Ok(Box::new(HorizontalConcatenateS2{out:new_ref(RowVector2::from_vec(mat))})),
      [e0, e1, e2]     => Ok(Box::new(HorizontalConcatenateS3{out:new_ref(RowVector3::from_vec(mat))})),
      [e0, e1, e2, e3] => Ok(Box::new(HorizontalConcatenateS4{out:new_ref(RowVector4::from_vec(mat))})),
      _ => Ok(Box::new(HorizontalConcatenateSD{out:new_ref(RowDVector::from_vec(mat))})),
    }      
  } else {
    match (nargs,columns) {
      //(1,1) => {}
      //(1,2) => {}
      //(1,3) => {}
      //(1,4) => {}
      //(1,n) => {}
      //(2,2) => {}
      (2,3) => {
        let mut out = RowVector3::from_element(F64::zero());
        match &arguments[..] {
          //sr2
          [Value::F64(e0), Value::MutableReference(ref_val)] => {
            match *ref_val.borrow() {
              Value::MatrixF64(Matrix::RowVector2(ref e1)) => {
                out[0] = e0.borrow().clone();
                Ok(Box::new(HorizontalConcatenateR2{el: e1.clone(), ix: 1, out: new_ref(out)}))
              }
              _ => todo!(),
            }
          }
          //r2s
          [Value::MutableReference(ref_val),Value::F64(e0)] => {
              match *ref_val.borrow() {
                Value::MatrixF64(Matrix::RowVector2(ref e1)) => {
                  out[2] = e0.borrow().clone();
                  Ok(Box::new(HorizontalConcatenateR2{el: e1.clone(), ix: 0, out: new_ref(out)}))
                }
                _ => todo!(),
              }
            }
            _ => todo!(),
          }
          //m1r2
          //r2m1
      }
      //(2,4) => {}
      //(2,n) => {}
      //(3,3) => {}
      //(3,4) => {}
      //(3,n) => {}
      //(4,4) => {}
      //(4,n) => {}
      //(m,n) => todo!()
      _ => Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
    }
  }
}

pub struct MaxtrixHorzCat {}
impl NativeFunctionCompiler for MaxtrixHorzCat {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    // First, get the size of the output matrix
    // rows are consistent already so we can just get nrows from the first element
    let rows = arguments[0].shape()[0];
    let columns:usize = arguments.iter().fold(0, |acc, x| acc + x.shape()[1]);
    impl_horzcat_fxn(arguments,rows,columns)
  }
}