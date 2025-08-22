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
    _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::None}),
  }
}

#[cfg(feature = "tuple")]
pub fn tuple(tpl: &Tuple, p: &Interpreter) -> MResult<Value> {
  let mut elements = vec![];
  for el in &tpl.elements {
    let result = expression(el,p)?;
    elements.push(Box::new(result));
  }
  let mech_tuple = MechTuple{elements};
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
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::KindMismatch(k.kind(),key_kind)});
    }
  }
  
  let value_kind = m.values().next().unwrap().kind();
  // verify that all the values are the same kind:
  for v in m.values() {
    if v.kind() != value_kind {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::KindMismatch(v.kind(),value_kind)});
    }
  }
  Ok(Value::Map(MechMap{
    num_elements: m.len(),
    key_kind,
    value_kind,
    map: m
  }))
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
      Some(k) => kind_annotation(&k.kind, p)?.to_value_kind(&p.functions())?,
      None => val.kind(),
    };
    // If the kinds are different, do a conversion.
    kinds.push(knd.clone());
    if knd != val.kind() {
      let fxn = ConvertKind{}.compile(&vec![val.clone(), Value::Kind(knd)]);
      match fxn {
        Ok(convert_fxn) => {
          convert_fxn.solve();
          let converted_result = convert_fxn.out();
          p.add_plan_step(convert_fxn);
          data.insert(name_hash, converted_result);
        },
        Err(e) => {
          return Err(MechError{id: line!(), file: file!().to_string(), tokens: vec![], msg: "".to_string(), kind: MechErrorKind::None});
        }
      }
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

#[cfg(feature = "set")]
pub fn set(m: &Set, p: &Interpreter) -> MResult<Value> { 
  let mut out = IndexSet::new();
  for el in &m.elements {
    let result = expression(el, p)?;
    out.insert(result);
  }

  let set_kind = if out.len() > 0 {
    out.iter().next().unwrap().kind()
  } else {
    ValueKind::Empty
  };
  
  // Make sure all elements have the same kind
  for el in &out {
    if el.kind() != set_kind {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::KindMismatch(el.kind(),set_kind)});
    }
  }

  Ok(Value::Set(MechSet{
    num_elements: out.len(),
    kind: set_kind,
    set: out, 
  }))
}

macro_rules! handle_value_kind {
  ($value_kind:ident, $val:expr, $field_label:expr, $data_map:expr, $converter:ident) => {{
    let mut vals = Vec::new();
    for x in $val.as_vec().iter() {
      match x.$converter() {
        Some(u) => vals.push(u.to_value()),
        None => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::WrongTableColumnKind});}
      }
    }
    let id = $field_label.as_u64().unwrap().borrow().clone();
    $data_map.insert(id, ($value_kind.clone(), Value::to_matrixd(vals.clone(), vals.len(), 1)));
  }};}

#[cfg(feature = "table")]
pub fn table(t: &Table, p: &Interpreter) -> MResult<Value> { 
  let mut rows = vec![];
  let headings = table_header(&t.header, p)?;
  let mut cols = 0;
  // Interpret the rows
  for row in &t.rows {
    let result = table_row(row, p)?;
    cols = result.len();
    rows.push(result);
  }
  // Provision columns
  let mut data = Vec::new();
  for i in 0..cols {
    data.push(vec![])
  }
  // Populate columns with data from rows
  for row in rows {
    for (ix,el) in row.iter().enumerate() {
      data[ix].push(el.clone());
    }
  }
  // Build the table
  let mut data_map: IndexMap<u64,(ValueKind,Matrix<Value>)> = IndexMap::new();
  for ((id,knd,name),(column)) in headings.iter().zip(data.iter()) {
    let val = Value::to_matrix(column.clone(),column.len(),1);
    match knd {
      #[cfg(feature = "i8")]
      ValueKind::I8   => handle_value_kind!(knd, val, id, data_map, as_i8),
      #[cfg(feature = "i16")]
      ValueKind::I16  => handle_value_kind!(knd, val, id, data_map, as_i16),
      #[cfg(feature = "i32")]
      ValueKind::I32  => handle_value_kind!(knd, val, id, data_map, as_i32),
      #[cfg(feature = "i64")]
      ValueKind::I64  => handle_value_kind!(knd, val, id, data_map, as_i64),
      #[cfg(feature = "i128")]
      ValueKind::I128 => handle_value_kind!(knd, val, id, data_map, as_i128),      
      #[cfg(feature = "u8")]
      ValueKind::U8   => handle_value_kind!(knd, val, id, data_map, as_u8),
      #[cfg(feature = "u16")]
      ValueKind::U16  => handle_value_kind!(knd, val, id, data_map, as_u16),
      #[cfg(feature = "u32")]
      ValueKind::U32  => handle_value_kind!(knd, val, id, data_map, as_u32),
      #[cfg(feature = "u64")]
      ValueKind::U64  => handle_value_kind!(knd, val, id, data_map, as_u64),
      #[cfg(feature = "u128")]
      ValueKind::U128 => handle_value_kind!(knd, val, id, data_map, as_u128),
      #[cfg(feature = "f32")]
      ValueKind::F32  => handle_value_kind!(knd, val, id, data_map, as_f32),
      #[cfg(feature = "f64")]
      ValueKind::F64  => handle_value_kind!(knd, val, id, data_map, as_f64),
      #[cfg(feature = "string")]
      ValueKind::String  => handle_value_kind!(knd, val, id, data_map, as_string),
      #[cfg(feature = "complex")]
      ValueKind::ComplexNumber  => handle_value_kind!(knd, val, id, data_map, as_complexnumber),
      #[cfg(feature = "rational")]
      ValueKind::RationalNumber  => handle_value_kind!(knd, val, id, data_map, as_rationalnumber),
      #[cfg(feature = "bool")]
      ValueKind::Bool => {
        let vals: Vec<Value> = val.as_vec().iter().map(|x| x.as_bool().unwrap().to_value()).collect::<Vec<Value>>();
        let id = id.as_u64().unwrap().borrow().clone();
        data_map.insert(id.clone(),(knd.clone(),Value::to_matrix(vals.clone(),vals.len(),1)));
      },
      _ => todo!(),
    };
  }
  let names: HashMap<u64,String> = headings.iter().map(|(id,_,name)| (id.as_u64().unwrap().borrow().clone(), name.to_string())).collect();
  let tbl = MechTable::new(t.rows.len(), cols, data_map.clone(), names);
  Ok(Value::Table(Ref::new(tbl)))
}

pub fn table_header(fields: &Vec<Field>, p: &Interpreter) -> MResult<Vec<(Value,ValueKind,Identifier)>> {
  let mut headings: Vec<(Value,ValueKind,Identifier)> = Vec::new();
  for f in fields {
    let id = f.name.hash();
    let kind = match &f.kind {
      Some(k) => kind_annotation(&k.kind, p)?,
      None => Kind::Any,
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

#[cfg(feature = "matrix")]
pub fn matrix(m: &Mat, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  let mut shape = vec![0, 0];
  let mut col: Vec<Value> = Vec::new();
  let mut kind = ValueKind::Empty;
  for row in &m.rows {
    let result = matrix_row(row, p)?;
    if shape == vec![0,0] {
      shape = result.shape();
      kind = result.kind();
      col.push(result);
    } else if shape[1] == result.shape()[1] {
      col.push(result);
    } else {
      return Err(MechError{file: file!().to_string(), tokens: row.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::DimensionMismatch(vec![])});
    }
  }
  if col.is_empty() {
    return Ok(Value::MatrixValue(Matrix::DMatrix(Ref::new(DMatrix::from_vec(0, 0, vec![])))));
  } else if col.len() == 1 {
    return Ok(col[0].clone());
  }
  let new_fxn = MaxtrixVertCat{}.compile(&col)?;
  new_fxn.solve();
  let out = new_fxn.out();
  let mut plan_brrw = plan.borrow_mut();
  plan_brrw.push(new_fxn);
  Ok(out)
}

#[cfg(feature = "matrix")]
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
      return Err(MechError{file: file!().to_string(), tokens: r.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::DimensionMismatch(vec![])});
    }
  }
  let new_fxn = MaxtrixHorzCat{}.compile(&row)?;
  new_fxn.solve();
  let out = new_fxn.out();
  let mut plan_brrw = plan.borrow_mut();
  plan_brrw.push(new_fxn);
  Ok(out)
}

pub fn matrix_column(r: &MatrixColumn, p: &Interpreter) -> MResult<Value> { 
  expression(&r.element, p)
}
