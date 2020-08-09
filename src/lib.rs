extern crate mech_core;
extern crate mech_utilities;
extern crate libm;
use mech_core::{Transaction};
use mech_core::{Value, ValueMethods, IndexIterator, Table, ValueIterator};
use mech_core::{Quantity, ToQuantity, QuantityMath, make_quantity};
use libm::{sin, cos, fmod, round, floor};

static PI: f64 = 3.141592653589793238462643383279502884197169399375105820974944592307816406286;

const ANGLE: u64 = 0x001e3f1182ea4d9d;

#[no_mangle]
pub extern "C" fn math_sin(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) { 
  let (in_arg_name, vi) = &arguments[0];
  let mut rows = vi.rows();
  let mut cols = match vi.column_iter {
    IndexIterator::Constant{..} => 1,
    _ => vi.columns(),
  };
  match in_arg_name {
    &ANGLE => {
      unsafe {
        (*out.table).rows = 1;
        (*out.table).columns = 1;
        (*out.table).data.resize(1, 0);
      }
      let mut flag: bool = false;
      for (i,m) in (1..=cols).zip(vi.column_iter.clone()) {
        for (j,k) in (1..=rows).zip(vi.row_iter.clone()) {
          let value = unsafe{(*vi.table).get(&k,&m).unwrap()};
          match value.as_quantity() {
            Some(x) => {
              let result = match fmod(x.as_float().unwrap(), 360.0) {
                0.0 => 0.0,
                90.0 => 1.0,
                180.0 => 0.0,
                270.0 => -1.0,
                _ => sin(x.as_float().unwrap() * PI / 180.0),
              };
              unsafe {
                (*out.table).set_unchecked(i, j, Value::from_f64(result));
              }
            },
            _ => (), // TODO Alert user that there was an error
          }
        }
      }  
    }
    _ => (), // Unknown argument name
  }
}

#[no_mangle]
pub extern "C" fn math_cos(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) { 
  /*let (argument, table_ref) = &input[0];
  let mut out = Table::new(0,table_ref.rows,table_ref.columns);
  if argument == "radians" {

  } else if argument == "degrees" {
    for i in 0..table_ref.columns as usize {
      for j in 0..table_ref.rows as usize {
        let x = &table_ref.data[i][j];
        let result = match fmod(x.as_float().unwrap(), 360.0) {
          0.0 => 1.0,
          90.0 => 0.0,
          180.0 => -1.0,
          270.0 => 0.0,
          _ => cos(x.as_float().unwrap() * PI / 180.0),
        };
        out.data[i][j] = Value::from_quantity(result.to_quantity());
      }
    }
  }
  out*/ 
}


#[no_mangle]
pub extern "C" fn math_round(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) { 
  /*let (argument, table_ref) = &input[0];
  let mut out = Table::new(0,table_ref.rows,table_ref.columns);
  for i in 0..table_ref.columns as usize {
    for j in 0..table_ref.rows as usize {
      let x = &table_ref.data[i][j];
      let result = match x {
        Value::Number(n) => Value::from_quantity(round(n.to_float()).to_quantity()),
        _ => Value::Empty
      };
      out.data[i][j] = result;
    }
  }
  out*/
}

#[no_mangle]
pub extern "C" fn math_floor(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) { 
  /*let (argument, table_ref) = &input[0];
  let mut out = Table::new(0,table_ref.rows,table_ref.columns);
  for i in 0..table_ref.columns as usize {
    for j in 0..table_ref.rows as usize {
      let x = &table_ref.data[i][j];
      let result = match x {
        Value::Number(n) => Value::from_quantity(floor(n.to_float()).to_quantity()),
        _ => Value::Empty
      };
      out.data[i][j] = result;
    }
  }
  out*/
}