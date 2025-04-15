use crate::*;
use std::collections::HashMap;

// Structures
// ----------------------------------------------------------------------------

pub fn structure(strct: &Structure, p: &Interpreter) -> MResult<Value> {
  match strct {
    Structure::Empty => Ok(Value::Empty),
    Structure::Record(x) => record(&x, p),
    Structure::Matrix(x) => matrix(&x, p),
    Structure::Table(x) => table(&x, p),
    Structure::Tuple(x) => tuple(&x, p),
    Structure::TupleStruct(x) => todo!(),
    Structure::Set(x) => set(&x, p),
    Structure::Map(x) => map(&x, p),
  }
}

pub fn tuple(tpl: &Tuple, p: &Interpreter) -> MResult<Value> {
  let mut elements = vec![];
  for el in &tpl.elements {
    let result = expression(el,p)?;
    elements.push(Box::new(result));
  }
  let mech_tuple = MechTuple{elements};
  Ok(Value::Tuple(mech_tuple))
}

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

pub fn record(rcrd: &Record, p: &Interpreter) -> MResult<Value> {
  let mut data: IndexMap<u64,Value> = IndexMap::new();
  let cols: usize = rcrd.bindings.len();
  let mut kinds: Vec<ValueKind> = Vec::new();

  for b in &rcrd.bindings {
    let name = b.name.hash();
    let val = expression(&b.value, p)?;
    let knd: ValueKind = match &b.kind {
      Some(k) => kind_annotation(&k.kind, p)?.to_value_kind(p.functions())?,
      None => val.kind(),
    };
    kinds.push(knd);
    data.insert(name, val);
  }
  Ok(Value::Record(MechRecord{
    cols,
    kinds,
    data,
  }))
}

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
    $data_map.insert($field_label.clone(), ($value_kind.clone(), Value::to_matrix(vals.clone(), vals.len(), 1)));
  }};}

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
  let mut data_map = IndexMap::new();
  for ((id,knd,name),(column)) in headings.iter().zip(data.iter()) {
    let val = Value::to_matrix(column.clone(),column.len(),1);
    match knd {
      ValueKind::I8   => handle_value_kind!(knd, val, id, data_map, as_i8),
      ValueKind::I16  => handle_value_kind!(knd, val, id, data_map, as_i16),
      ValueKind::I32  => handle_value_kind!(knd, val, id, data_map, as_i32),
      ValueKind::I64  => handle_value_kind!(knd, val, id, data_map, as_i64),
      ValueKind::I128 => handle_value_kind!(knd, val, id, data_map, as_i128),      
      ValueKind::U8   => handle_value_kind!(knd, val, id, data_map, as_u8),
      ValueKind::U16  => handle_value_kind!(knd, val, id, data_map, as_u16),
      ValueKind::U32  => handle_value_kind!(knd, val, id, data_map, as_u32),
      ValueKind::U64  => handle_value_kind!(knd, val, id, data_map, as_u64),
      ValueKind::U128 => handle_value_kind!(knd, val, id, data_map, as_u128),
      ValueKind::F32  => handle_value_kind!(knd, val, id, data_map, as_f32),
      ValueKind::F64  => handle_value_kind!(knd, val, id, data_map, as_f64),
      ValueKind::Bool => {
        let vals: Vec<Value> = val.as_vec().iter().map(|x| x.as_bool().unwrap().to_value()).collect::<Vec<Value>>();
        data_map.insert(id.clone(),(knd.clone(),Value::to_matrix(vals.clone(),vals.len(),1)));
      },
      _ => todo!(),
    };
  }
  let names: HashMap<Value,String> = headings.iter().map(|(id,_,name)| (id.clone(), name.to_string())).collect();
  let tbl = MechTable::new(t.rows.len(), cols, data_map.clone(), names);
  Ok(Value::Table(tbl))
}

pub fn table_header(fields: &Vec<Field>, p: &Interpreter) -> MResult<Vec<(Value,ValueKind,Identifier)>> {
  let mut headings: Vec<(Value,ValueKind,Identifier)> = Vec::new();
  for f in fields {
    let id = f.name.hash();
    let kind = match &f.kind {
      Some(k) => kind_annotation(&k.kind, p)?,
      None => Kind::Any,
    };
    headings.push((Value::Id(id),kind.to_value_kind(p.functions())?,f.name.clone()));
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
    return Ok(Value::MatrixF64(Matrix::<F64>::DMatrix(new_ref(DMatrix::from_vec(0, 0, vec![])))));
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
