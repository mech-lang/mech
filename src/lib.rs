extern crate mech_core;
extern crate mech_utilities;
use mech_core::{Transaction, ValueIterator, ValueMethods};
use mech_core::{Value, Table, Index};
use mech_core::{Quantity, ToQuantity, QuantityMath, make_quantity};

const ROW: u64 = 0x001e3f1182ea4d9d;
const COLUMN: u64 = 0x0071b9e91df03940;
const TABLE: u64 = 0x0064ae06e4bbf825;

#[no_mangle]
pub extern "C" fn stats_average(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {                                        

  // TODO test argument count is 1
  let (in_arg_name, vi) = &arguments[0];

  let mut rows = vi.rows();
  let mut cols = vi.columns();

  match in_arg_name {
    &ROW => {
      unsafe {
        (*out.table).rows = vi.rows();
        (*out.table).columns = 1;
        (*out.table).data.resize(vi.rows(), 0);
      }
      for i in 1..=rows {
        let mut sum: Value = Value::from_u64(0);
        for j in 1..=cols {
          match vi.get(&Index::Index(i),&Index::Index(j)) {
            Some(value) => {
              match sum.add(value) {
                Ok(result) => sum = result,
                _ => (), // TODO Alert user that there was an error
              }
            }
            _ => ()
          }
        }
        unsafe {
          (*out.table).set_unchecked(i, 1, Value::from_f64(sum.as_float().unwrap() / vi.columns() as f64));
        }
      }
    }
    &COLUMN => {
      unsafe {
        (*out.table).rows = 1;
        (*out.table).columns = cols;
        (*out.table).data.resize(cols, 0);
      }
      for (i,m) in (1..=cols).zip(vi.column_iter.clone()) {
        let mut sum: Value = Value::from_u64(0);
        for (j,k) in (1..=rows).zip(vi.row_iter.clone()) {
          match vi.get(&k,&m) {
            Some(value) => {
              match sum.add(value) {
                Ok(result) => sum = result,
                _ => (), // TODO Alert user that there was an error
              }
            }
            _ => ()
          }
        }
        unsafe {
          (*out.table).set_unchecked(1, i, Value::from_f64(sum.as_float().unwrap() / vi.rows() as f64));
        }
      }      
    }
    &TABLE => {
      unsafe {
        (*out.table).rows = 1;
        (*out.table).columns = 1;
        (*out.table).data.resize(1, 0);
      }
      let mut sum: Value = Value::from_u64(0);
      for (i,m) in (1..=cols).zip(vi.column_iter.clone()) {
        for (j,k) in (1..=rows).zip(vi.row_iter.clone()) {
          match vi.get(&k,&m) {
            Some(value) => {
              match sum.add(value) {
                Ok(result) => sum = result,
                _ => (), // TODO Alert user that there was an error
              }
            }
            _ => ()
          }
        }
      }  
      unsafe {
        (*out.table).set_unchecked(1, 1, Value::from_f64(sum.as_float().unwrap() / (vi.rows() * vi.columns()) as f64   ));
      }    
    }
    _ => (), // TODO alert user that argument is unknown
  }
}