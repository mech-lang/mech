#[macro_use]
use crate::stdlib::*;
use crate::stdlib::access::*;

// Table Access ---------------------------------------------------------------

macro_rules! impl_col_access_fxn {
  ($fxn_name:ident, $vector_size:ident, $out_type:ty) => {
    #[derive(Debug)]
    struct $fxn_name {
      source: Matrix<Value>,
      out: Ref<$vector_size<$out_type>>,
    }
    impl MechFunctionImpl for $fxn_name {
      fn solve(&self) {
        let out_ptr = self.out.as_mut_ptr();
        unsafe { 
          for i in 1..=self.source.shape()[0] {
            paste! {
              (&mut (*out_ptr))[i-1] = self.source.index1d(i).[<as_ $out_type:lower>]().unwrap().borrow().clone();
            }
          }
        }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for $fxn_name {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let mut registers = [0, 0];
        registers[0] = compile_register_brrw!(self.out, ctx);
        registers[1] = compile_register!(self.source, ctx);
        ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Access));
        ctx.emit_unop(
          hash_str(stringify!($fxn_name)),
          registers[0],
          registers[1],
        );
        Ok(registers[0])
      }
    }
  }
}

macro_rules! impl_col_access_fxn_shapes {
  ($type:ident) => {
    paste!{
      #[cfg(feature = "matrix1")]
      impl_col_access_fxn!([<TableAccessCol $type:camel M1>], Matrix1, [<$type>]);
      #[cfg(feature = "vector2")]
      impl_col_access_fxn!([<TableAccessCol $type:camel V2>], Vector2, [<$type>]);
      #[cfg(feature = "vector3")]
      impl_col_access_fxn!([<TableAccessCol $type:camel V3>], Vector3, [<$type>]);
      #[cfg(feature = "vector4")]
      impl_col_access_fxn!([<TableAccessCol $type:camel V4>], Vector4, [<$type>]);
      #[cfg(feature = "vectord")]
      impl_col_access_fxn!([<TableAccessCol $type:camel VD>], DVector, [<$type>]);
    }
  }
}

#[cfg(all(feature = "bool", feature = "matrix"))]
impl_col_access_fxn_shapes!(bool);
#[cfg(all(feature = "i8", feature = "matrix"))]
impl_col_access_fxn_shapes!(i8);
#[cfg(all(feature = "i16", feature = "matrix"))]
impl_col_access_fxn_shapes!(i16);
#[cfg(all(feature = "i32", feature = "matrix"))]
impl_col_access_fxn_shapes!(i32);
#[cfg(all(feature = "i64", feature = "matrix"))]
impl_col_access_fxn_shapes!(i64);
#[cfg(all(feature = "i128", feature = "matrix"))]
impl_col_access_fxn_shapes!(i128);
#[cfg(all(feature = "u8", feature = "matrix"))]
impl_col_access_fxn_shapes!(u8);
#[cfg(all(feature = "u16", feature = "matrix"))]
impl_col_access_fxn_shapes!(u16);
#[cfg(all(feature = "u32", feature = "matrix"))]
impl_col_access_fxn_shapes!(u32);
#[cfg(all(feature = "u64", feature = "matrix"))]
impl_col_access_fxn_shapes!(u64);
#[cfg(all(feature = "u128", feature = "matrix"))]
impl_col_access_fxn_shapes!(u128);
#[cfg(all(feature = "f32", feature = "matrix"))]
impl_col_access_fxn_shapes!(F32);
#[cfg(all(feature = "f64", feature = "matrix"))]
impl_col_access_fxn_shapes!(F64);
#[cfg(all(feature = "string", feature = "matrix"))]
impl_col_access_fxn_shapes!(String);
#[cfg(all(feature = "complex", feature = "matrix"))]
impl_col_access_fxn_shapes!(ComplexNumber);
#[cfg(all(feature = "rational", feature = "matrix"))]
impl_col_access_fxn_shapes!(R64);

macro_rules! impl_access_column_table_match_arms {
  ($arg:expr, $($lhs_type:ident, $($default:expr, $type_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        (Value::Table(tbl),Value::Id(k)) => {
          let tbl_brrw = tbl.borrow();
          match (tbl_brrw.get(&k),tbl_brrw.rows()) {
            $(
              $(
                #[cfg(all(feature = $type_string, feature = "matrix1"))]
                (Some((ValueKind::$lhs_type,value)),1) => Ok(Box::new([<TableAccessCol $lhs_type M1>]{source: value.clone(), out: Ref::new(Matrix1::from_element($default)) })),
                #[cfg(all(feature = $type_string, feature = "vector2"))]
                (Some((ValueKind::$lhs_type,value)),2) => Ok(Box::new([<TableAccessCol $lhs_type V2>]{source: value.clone(), out: Ref::new(Vector2::from_element($default)) })),
                #[cfg(all(feature = $type_string, feature = "vector3"))]
                (Some((ValueKind::$lhs_type,value)),3) => Ok(Box::new([<TableAccessCol $lhs_type V3>]{source: value.clone(), out: Ref::new(Vector3::from_element($default)) })),
                #[cfg(all(feature = $type_string, feature = "vector4"))]
                (Some((ValueKind::$lhs_type,value)),4) => Ok(Box::new([<TableAccessCol $lhs_type V4>]{source: value.clone(), out: Ref::new(Vector4::from_element($default)) })),
                #[cfg(all(feature = $type_string, feature = "vectord"))]
                (Some((ValueKind::$lhs_type,value)),n) => Ok(Box::new([<TableAccessCol $lhs_type VD>]{source: value.clone(), out: Ref::new(DVector::from_element(n,$default)) })),
              )+
            )+
            x => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "no shape".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
          }
        }
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:#?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_access_column_table_fxn(source: Value, key: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_access_column_table_match_arms!(
    (source,key),
    Bool,bool::default(),"bool";
    I8,i8::default(),"i8";
    I16,i16::default(),"i16";
    I32,i32::default(),"i32";
    I64,i64::default(),"i64";
    I128,i128::default(),"i128";
    U8,u8::default(),"u8";
    U16,u16::default(),"u16";
    U32,u32::default(),"u32";
    U64,u64::default(),"u64";
    U128,u128::default(),"u128";
    F32,F32::default(),"f32";
    F64,F64::default(),"f64";
    String,String::default(),"string";
    ComplexNumber,ComplexNumber::default(),"complex";
    R64,R64::default(),"rational";
  )
}

pub struct TableAccessColumn {}
impl NativeFunctionCompiler for TableAccessColumn {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let tbl = arguments[0].clone();
    let key = arguments[1].clone();
    match impl_access_column_table_fxn(tbl.clone(), key.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (tbl,&key) {
          (Value::MutableReference(tbl),_) => { impl_access_column_table_fxn(tbl.borrow().clone(), key.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}
  
// Table Access Swizzle -------------------------------------------------------

#[derive(Debug)]
pub struct TableAccessSwizzle {
  pub out: Value,
}

impl MechFunctionImpl for TableAccessSwizzle {
  fn solve(&self) {
    ()
  }
  fn out(&self) -> Value { self.out.clone() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for TableAccessSwizzle {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0];
    registers[0] = compile_register!(self.out, ctx);
    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Swizzle));
    ctx.emit_nullop(
      hash_str("TableAccessSwizzle"),
      registers[0],
    );
    Ok(registers[0])
  }
}

// Table Access Scalar -------------------------------------------------------

#[derive(Debug)]
pub struct TableAccessScalarF {
  pub source: Ref<MechTable>,
  pub ix: Ref<usize>,
  pub out: Ref<MechRecord>,
}

impl MechFunctionImpl for TableAccessScalarF {
  fn solve(&self) {
    let table = self.source.borrow();
    let mut record = self.out.borrow_mut();
    let row_ix = *self.ix.borrow();
    for (key, (kind, matrix)) in table.data.iter() {
      let value = matrix.index1d(row_ix);
      record.data.insert(*key, value.clone());
    }
  }
  fn out(&self) -> Value { Value::Record(self.out.clone()) }
  fn to_string(&self) -> String {format!("{:#?}", self)}
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for TableAccessScalarF {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0,0,0];
    
    registers[0] = compile_register_brrw!(self.out,  ctx);
    registers[1] = compile_register_brrw!(self.source, ctx);
    registers[2] = compile_register_brrw!(self.ix, ctx);

    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Table));
    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Access));

    ctx.emit_binop(
      hash_str(stringify!("TableAccessScalarF")),
      registers[0],
      registers[1],
      registers[2],
    );

    return Ok(registers[0])
  }
}

pub struct TableAccessScalar{}

impl NativeFunctionCompiler for TableAccessScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let tbl = arguments[0].clone();
    let ix = arguments[1].clone();
    match (tbl, ix) {
      #[cfg(feature = "table")]
      (Value::Table(source), Value::Index(ix)) => {
        let record = match source.borrow().get_record(*ix.borrow()) {
          Some(record) => record,
          None => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::None}),
        };
        Ok(Box::new(TableAccessScalarF{source: source.clone(), ix: ix.clone(), out: Ref::new(record) }))
      }
      (Value::MutableReference(src_ref), Value::Index(ix)) => {
        let src_ref_brrw = src_ref.borrow();
        match &*src_ref_brrw {
          #[cfg(feature = "table")]
          Value::Table(source) => {
            let record = match source.borrow().get_record(*ix.borrow()) {
              Some(record) => record,
              None => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::None}),
            };
            Ok(Box::new(TableAccessScalarF{source: source.clone(), ix: ix.clone(), out: Ref::new(record) }))
          }
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
        }
      }
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
    }
  }
}

// Table Access Range -------------------------------------------------------

#[derive(Debug)]
pub struct TableAccessRangeIndex {
  pub source: Ref<MechTable>,
  pub ix: Ref<DVector<usize>>,
  pub out: Ref<MechTable>,
}

impl MechFunctionImpl for TableAccessRangeIndex {
  fn solve(&self) {
    let table = self.source.borrow();
    let mut out_table = self.out.borrow_mut();
    let ix_brrw = self.ix.borrow();

    for (key, (_kind, matrix)) in table.data.iter() {
      let (_out_kind, out_matrix) = out_table.data.get_mut(key).unwrap();
      for (out_i, i) in ix_brrw.iter().enumerate() {
        let value = matrix.index1d(*i);
        out_matrix.set_index1d(out_i, value.clone());
      }
    }
  }
  fn out(&self) -> Value { Value::Table(self.out.clone()) }
  fn to_string(&self) -> String {format!("{:#?}", self)}
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for TableAccessRangeIndex {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0,0,0];
    
    registers[0] = compile_register_brrw!(self.out,  ctx);
    registers[1] = compile_register_brrw!(self.source, ctx);
    registers[2] = compile_register_brrw!(self.ix, ctx);

    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Table));
    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::SubscriptRange));

    ctx.emit_binop(
      hash_str(stringify!("TableAccessRangeIndex")),
      registers[0],
      registers[1],
      registers[2],
    );

    return Ok(registers[0])
  }
}

#[derive(Debug)]
pub struct TableAccessRangeBool {
  pub source: Ref<MechTable>,
  pub ix: Ref<DVector<bool>>,
  pub out: Ref<MechTable>,
}

impl MechFunctionImpl for TableAccessRangeBool {
  fn solve(&self) {
    let table = self.source.borrow();
    let ix_brrw = self.ix.borrow();
    let true_count = ix_brrw.iter().filter(|&&b| b).count();

    let mut out_table = self.out.borrow_mut();

    for (key, (_kind, matrix)) in table.data.iter() {
      let (_out_kind, out_matrix) = out_table.data.get_mut(key).unwrap();

      // Resize output to match number of true entries
      out_matrix.resize_vertically(true_count, Value::Empty);

      // Fill with contiguous values
      let mut push_index = 0;
      for (i, flag) in ix_brrw.iter().enumerate() {
        if *flag {
          let value = matrix.index1d(i + 1); // 1-based indexing; use `i` if 0-based
          out_matrix.set_index1d(push_index, value.clone());
          push_index += 1;
        }
      }
    }
    out_table.rows = true_count;
  }
  fn out(&self) -> Value { Value::Table(self.out.clone()) }
  fn to_string(&self) -> String {format!("{:#?}", self)}
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for TableAccessRangeBool {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0,0,0];
    
    registers[0] = compile_register_brrw!(self.out,  ctx);
    registers[1] = compile_register_brrw!(self.source, ctx);
    registers[2] = compile_register_brrw!(self.ix, ctx);

    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Table));
    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::LogicalIndexing));

    ctx.emit_binop(
      hash_str(stringify!("TableAccessRangeBool")),
      registers[0],
      registers[1],
      registers[2],
    );

    return Ok(registers[0])
  }
}

pub struct TableAccessRange{}

impl NativeFunctionCompiler for TableAccessRange {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let tbl = arguments[0].clone();
    match (tbl, ixes.as_slice()) {
      #[cfg(all(feature = "table", feature = "matrix"))]
      (Value::Table(source), [Value::MatrixIndex(Matrix::DVector(ix))])  => {
        let out_table = source.borrow().empty_table(ix.borrow().len());
        Ok(Box::new(TableAccessRangeIndex{source: source.clone(), ix: ix.clone(), out: Ref::new(out_table) }))
      }
      #[cfg(all(feature = "matrix", feature = "table", feature = "logical_indexing"))]
      (Value::Table(source), [Value::MatrixBool(Matrix::DVector(ix))])  => {
        let out_table = source.borrow().empty_table(ix.borrow().len());
        Ok(Box::new(TableAccessRangeBool{source: source.clone(), ix: ix.clone(), out: Ref::new(out_table) }))
      }
      #[cfg(all(feature = "table", feature = "matrix"))]
      (Value::MutableReference(src_ref), [Value::MatrixIndex(Matrix::DVector(ix))]) => {
        let src_ref_brrw = src_ref.borrow();
        match &*src_ref_brrw {
          Value::Table(source) => {
            let out_table = source.borrow().empty_table(ix.borrow().len());
            Ok(Box::new(TableAccessRangeIndex{source: source.clone(), ix: ix.clone(), out: Ref::new(out_table) }))
          }
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
        }
      }
      #[cfg(all(feature = "matrix", feature = "table", feature = "logical_indexing"))]
      (Value::MutableReference(src_ref), [Value::MatrixBool(Matrix::DVector(ix))]) => {
        let src_ref_brrw = src_ref.borrow();
        match &*src_ref_brrw {
          Value::Table(source) => {
            let out_table = source.borrow().empty_table(ix.borrow().len());
            Ok(Box::new(TableAccessRangeBool{source: source.clone(), ix: ix.clone(), out: Ref::new(out_table) }))
          }
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
        }
      }
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
    }
  }
}