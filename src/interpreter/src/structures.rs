use crate::*;
use std::collections::HashMap;

// Structures
// ----------------------------------------------------------------------------

pub fn structure(strct: &Structure, p: &Interpreter) -> MResult<Value> {
  match strct {
    Structure::Empty => Ok(Value::Empty),
    #[cfg(feature = "record")]
    Structure::Record(x) => record(&x, p),
    #[cfg(feature = "matrix")]
    Structure::Matrix(x) => matrix(&x, p),
    #[cfg(feature = "table")]
    Structure::Table(x) => table(&x, p),
    #[cfg(feature = "tuple")]
    Structure::Tuple(x) => tuple(&x, p),
    #[cfg(feature = "tuple_struct")]
    Structure::TupleStruct(x) => todo!(),
    #[cfg(feature = "set")]
    Structure::Set(x) => set(&x, p),
    #[cfg(feature = "map")]
    Structure::Map(x) => map(&x, p),
    x => Err(MechError2::new(FeatureNotEnabledError, Some(format!("Feature not enabled for `{:?}`", stringify!(x)))).with_compiler_loc()),
  }
}

#[cfg(feature = "tuple")]
pub fn tuple(tpl: &Tuple, p: &Interpreter) -> MResult<Value> {
  let mut elements = vec![];
  for el in &tpl.elements {
    let result = expression(el,p)?;
    elements.push(Box::new(result));
  }
  let mech_tuple = Ref::new(MechTuple{elements});
  Ok(Value::Tuple(mech_tuple))
}

#[cfg(feature = "map")]
pub fn map(mp: &Map, p: &Interpreter) -> MResult<Value> {
  let mut m = IndexMap::new();
  for b in &mp.elements {
    let key = expression(&b.key, p)?;
    let val = expression(&b.value, p)?;
    m.insert(key,val);
  }
  
  let key_kind = m.keys().next().unwrap().kind();
  // verify that all the keys are the same kind:
  for k in m.keys() {
    if k.kind() != key_kind {
      return Err(MechError2::new(
        MapKeyKindMismatchError{expected_kind: key_kind.clone(), actual_kind: k.kind().clone()},
        None
      ).with_compiler_loc());
    }
  }
  
  let value_kind = m.values().next().unwrap().kind();
  // verify that all the values are the same kind:
  for v in m.values() {
    if v.kind() != value_kind {
      return Err(MechError2::new(
        MapValueKindMismatchError{expected_kind: value_kind.clone(), actual_kind: v.kind().clone()},
        None
      ).with_compiler_loc());
    }
  }
  Ok(Value::Map(Ref::new(MechMap{
    num_elements: m.len(),
    key_kind,
    value_kind,
    map: m
  })))
}

#[cfg(feature = "record")]
pub fn record(rcrd: &Record, p: &Interpreter) -> MResult<Value> {
  let mut data: IndexMap<u64,Value> = IndexMap::new();
  let cols: usize = rcrd.bindings.len();
  let mut kinds: Vec<ValueKind> = Vec::new();
  let mut field_names: HashMap<u64,String> = HashMap::new();
  for b in &rcrd.bindings {
    let name_hash = b.name.hash();
    let name_str = b.name.to_string();
    let val = expression(&b.value, p)?;
    let knd: ValueKind = match &b.kind {
      Some(k) => kind_annotation(&k.kind, p)?.to_value_kind(&p.state.borrow().kinds)?,
      None => val.kind(),
    };
    // If the kinds are different, do a conversion.
    kinds.push(knd.clone());
    #[cfg(feature = "convert")]
    if knd != val.kind() {
      let fxn = ConvertKind{}.compile(&vec![val.clone(), Value::Kind(knd.clone())]);
      match fxn {
        Ok(convert_fxn) => {
          convert_fxn.solve();
          let converted_result = convert_fxn.out();
          p.state.borrow_mut().add_plan_step(convert_fxn);
          data.insert(name_hash, converted_result);
        },
        Err(e) => {
          return Err(MechError2::new(
            TableColumnKindMismatchError {
              column_id: name_hash,
              expected_kind: knd.clone(),
              actual_kind: val.kind().clone(),
            },
            None
          ).with_compiler_loc());
        }
      }
    } else {
      data.insert(name_hash, val);
    }
    #[cfg(not(feature = "convert"))]
    if knd != val.kind() {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::KindMismatch(val.kind(),knd)});
    } else {
      data.insert(name_hash, val);
    }
    field_names.insert(name_hash, name_str);
  }
  Ok(Value::Record(Ref::new(MechRecord{
    cols,
    kinds,
    data,
    field_names,
  })))
}

// Set
// ----------------------------------------------------------------------------

// Define a MechFunction that creaates a Set from a list of Values
#[cfg(feature = "set")]
#[derive(Debug)]
pub struct ValueSet {
  pub out: Ref<MechSet>,
}
#[cfg(feature = "set")]
#[cfg(feature = "functions")]
impl MechFunctionImpl for ValueSet {
  fn solve(&self) {}
  fn out(&self) -> Value { Value::Set(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "set")]
#[cfg(feature = "functions")]
impl MechFunctionFactory for ValueSet {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Nullary(out) => {
        let out: Ref<MechSet> = unsafe{ out.as_unchecked().clone() };
        Ok(Box::new(ValueSet {out}))
      },
      _ => Err(MechError2::new(
          IncorrectNumberOfArguments { expected: 0, found: args.len() },
          None
        ).with_compiler_loc()
      ),
    }
  }
}
#[cfg(feature = "set")]
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for ValueSet {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    compile_nullop!("set/define", self.out, ctx, FeatureFlag::Builtin(FeatureKind::Set));
  }
}
#[cfg(feature = "set")]
#[cfg(feature = "functions")]
register_descriptor!{
  FunctionDescriptor {
    name: "set/define",
    ptr: ValueSet::new,
  }
}

#[cfg(feature = "set")]
pub struct SetDefine {}
#[cfg(feature = "set")]
#[cfg(feature = "functions")]
impl NativeFunctionCompiler for SetDefine {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    Ok(Box::new(ValueSet {
      out: Ref::new(MechSet::from_vec(arguments.clone())),
    }))
  }
}
#[cfg(feature = "set")]
#[cfg(feature = "functions")]
register_descriptor!{
  FunctionCompilerDescriptor {
    name: "set/define",
    ptr: &SetDefine{},
  }
}

#[cfg(feature = "set")]
pub fn set(m: &Set, p: &Interpreter) -> MResult<Value> { 
  let mut elements = Vec::new();
  for el in &m.elements {
    let result = expression(el, p)?;
    elements.push(result.clone());
  }
  let element_kind = if elements.len() > 0 {
    elements[0].kind()
  } else {
    ValueKind::Empty
  };
  // Make sure all elements have the same kind
  for el in &elements {
    if el.kind() != element_kind {
      return Err(MechError2::new(
        SetKindMismatchError{expected_kind: element_kind.clone(), actual_kind: el.kind().clone()},
        None
      ).with_compiler_loc());
    }
  }
  #[cfg(feature = "functions")]
  {
    let new_fxn = SetDefine {}.compile(&elements)?;
    new_fxn.solve();
    let out = new_fxn.out();
    let plan = p.plan();
    let mut plan_brrw = plan.borrow_mut();
    plan_brrw.push(new_fxn);
    Ok(out)
  }
  #[cfg(not(feature = "functions"))]
  {
    Ok(Value::Set(Ref::new(MechSet::from_vec(elements))))
  }
}

// Table
// ----------------------------------------------------------------------------

macro_rules! handle_value_kind {
  ($value_kind:ident, $val:expr, $field_label:expr, $data_map:expr, $converter:ident) => {{
    let mut vals = Vec::new();
    let id = $field_label; // <- FIXED: it's already a u64
    for x in $val.as_vec().iter() {
      match x.$converter() {
        Ok(u) => vals.push(u.to_value()),
        Err(_) => {
          return Err(MechError2::new(
            TableColumnKindMismatchError { 
              column_id: id, 
              expected_kind: $value_kind.clone(), 
              actual_kind: x.kind() 
            },
            None
          ).with_compiler_loc());
        }
      }
    }
    $data_map.insert(id, ($value_kind.clone(), Value::to_matrixd(vals.clone(), vals.len(), 1)));
  }};
}


#[cfg(feature = "table")]
fn handle_column_kind(
    kind: ValueKind,
    id: u64,
    val: Matrix<Value>,
    data_map: &mut IndexMap<u64,(ValueKind,Matrix<Value>)>
) -> MResult<()> 
{
  match kind {
    #[cfg(feature = "i8")]
    ValueKind::I8   => handle_value_kind!(kind, val, id, data_map, as_i8),
    #[cfg(feature = "i16")]
    ValueKind::I16  => handle_value_kind!(kind, val, id, data_map, as_i16),
    #[cfg(feature = "i32")]
    ValueKind::I32  => handle_value_kind!(kind, val, id, data_map, as_i32),
    #[cfg(feature = "i64")]
    ValueKind::I64  => handle_value_kind!(kind, val, id, data_map, as_i64),
    #[cfg(feature = "i128")]
    ValueKind::I128 => handle_value_kind!(kind, val, id, data_map, as_i128),

    #[cfg(feature = "u8")]
    ValueKind::U8   => handle_value_kind!(kind, val, id, data_map, as_u8),
    #[cfg(feature = "u16")]
    ValueKind::U16  => handle_value_kind!(kind, val, id, data_map, as_u16),
    #[cfg(feature = "u32")]
    ValueKind::U32  => handle_value_kind!(kind, val, id, data_map, as_u32),
    #[cfg(feature = "u64")]
    ValueKind::U64  => handle_value_kind!(kind, val, id, data_map, as_u64),
    #[cfg(feature = "u128")]
    ValueKind::U128 => handle_value_kind!(kind, val, id, data_map, as_u128),

    #[cfg(feature = "f32")]
    ValueKind::F32  => handle_value_kind!(kind, val, id, data_map, as_f32),
    #[cfg(feature = "f64")]
    ValueKind::F64  => handle_value_kind!(kind, val, id, data_map, as_f64),

    #[cfg(feature = "string")]
    ValueKind::String => handle_value_kind!(kind, val, id, data_map, as_string),

    #[cfg(feature = "complex")]
    ValueKind::C64 => handle_value_kind!(kind, val, id, data_map, as_c64),

    #[cfg(feature = "rational")]
    ValueKind::R64 => handle_value_kind!(kind, val, id, data_map, as_r64),

    #[cfg(feature = "bool")]
    ValueKind::Bool => {
      let vals: Vec<Value> = val.as_vec()
          .iter()
          .map(|x| x.as_bool().unwrap().to_value())
          .collect();
      data_map.insert(id, (ValueKind::Bool, Value::to_matrix(vals.clone(), vals.len(), 1)));
    }

    x => {
      println!("Unsupported kind in table column: {:?}", x);
      todo!()
    }
  }

  Ok(())
}

#[cfg(feature = "table")]
pub fn table(t: &Table, p: &Interpreter) -> MResult<Value> { 
  let mut rows = vec![];
  let headings = table_header(&t.header, p)?;
  let mut cols = 0;

  // Interpret rows
  for row in &t.rows {
    let result = table_row(row, p)?;
    cols = result.len();
    rows.push(result);
  }

  // Allocate columns
  let mut data = vec![Vec::<Value>::new(); cols];

  // Populate columns
  for row in rows {
    for (ix, el) in row.into_iter().enumerate() {
      data[ix].push(el);
    }
  }

  // Build table
  let mut data_map: IndexMap<u64,(ValueKind,Matrix<Value>)> = IndexMap::new();

  for ((id, knd, _name), column) in headings.iter().zip(data.iter()) {
    let id_u64 = id.as_u64().unwrap().borrow().clone();

    // Infer kind if None
    let actual_kind = match knd {
      ValueKind::None => {
        match column.first() {
          Some(v) => v.kind(),
          None => ValueKind::String, // default for empty column
        }
      }
      _ => knd.clone(),
    };

    // Convert column to matrix
    let val = Value::to_matrix(column.clone(), column.len(), 1);

    // Dispatch conversion
    handle_column_kind(actual_kind, id_u64, val, &mut data_map)?;
  }

  // Assign names
  let names: HashMap<u64, String> = headings.iter()
      .map(|(id, _, name)| (id.as_u64().unwrap().borrow().clone(), name.to_string()))
      .collect();

  let tbl = MechTable::new(t.rows.len(), cols, data_map.clone(), names);
  Ok(Value::Table(Ref::new(tbl)))
}

#[cfg(feature = "kind_annotation")]
pub fn table_header(fields: &Vec<Field>, p: &Interpreter) -> MResult<Vec<(Value,ValueKind,Identifier)>> {
  let mut headings: Vec<(Value,ValueKind,Identifier)> = Vec::new();
  for f in fields {
    let id = f.name.hash();
    let kind = match &f.kind {
      Some(k) => kind_annotation(&k.kind, p)?,
      None => Kind::None,
    };
    headings.push((Value::Id(id),kind.to_value_kind(&p.state.borrow().kinds)?,f.name.clone()));
  }
  Ok(headings)
}

pub fn table_row(r: &TableRow, p: &Interpreter) -> MResult<Vec<Value>> {
  let mut row: Vec<Value> = Vec::new();
  for col in &r.columns {
    let result = table_column(col, p)?;
    row.push(result);
  }
  Ok(row)
}

pub fn table_column(r: &TableColumn, p: &Interpreter) -> MResult<Value> { 
  expression(&r.element, p)
}

// Matrix
// ----------------------------------------------------------------------------

#[cfg(feature = "matrix")]
pub fn matrix(m: &Mat, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  let mut shape = vec![0, 0];
  let mut col: Vec<Value> = Vec::new();
  let mut kind = ValueKind::Empty;
  #[cfg(feature = "matrix_horzcat")]
  {
    for row in &m.rows {
      let result = matrix_row(row, p)?;
      if shape == vec![0,0] {
        shape = result.shape();
        kind = result.kind();
        col.push(result);
      } else if shape[1] == result.shape()[1] {
        col.push(result);
      } else {
        return Err(MechError2::new(
            DimensionMismatch { dims: vec![shape[1], result.shape()[1]] },
            None
          ).with_compiler_loc()
        );
      }
    }
    if col.is_empty() {
      return Ok(Value::MatrixValue(Matrix::from_vec(vec![], 0, 0)));
    } else if col.len() == 1 {
      return Ok(col[0].clone());
    }
  }
  #[cfg(feature = "matrix_vertcat")]
  {
    let new_fxn = MatrixVertCat{}.compile(&col)?;
    new_fxn.solve();
    let out = new_fxn.out();
    let mut plan_brrw = plan.borrow_mut();
    plan_brrw.push(new_fxn);
    return Ok(out);
  }
  return Err(MechError2::new(
    FeatureNotEnabledError,
    Some("matrix/vertcat feature not enabled".to_string())).with_compiler_loc()
  );
}

#[cfg(feature = "matrix_horzcat")]
pub fn matrix_row(r: &MatrixRow, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  let mut row: Vec<Value> = Vec::new();
  let mut shape = vec![0, 0];
  let mut kind = ValueKind::Empty;
  for col in &r.columns {
    let result = matrix_column(col, p)?;
    if shape == vec![0,0] {
      shape = result.shape();
      kind = result.kind();
      row.push(result);
    } else if shape[0] == result.shape()[0] {
      row.push(result);
    } else {
      return Err(MechError2::new(
          DimensionMismatch { dims: vec![shape[0], result.shape()[0]] },
          None
        ).with_compiler_loc()
      );
    }
  }
  let new_fxn = MatrixHorzCat{}.compile(&row)?;
  new_fxn.solve();
  let out = new_fxn.out();
  let mut plan_brrw = plan.borrow_mut();
  plan_brrw.push(new_fxn);
  Ok(out)
}

pub fn matrix_column(r: &MatrixColumn, p: &Interpreter) -> MResult<Value> { 
  expression(&r.element, p)
}
