extern crate mech_core;
extern crate mech_utilities;
use mech_core::{Interner, Transaction};
use mech_core::{Value, Table};
use mech_utilities::Watcher;
use mech_core::{Quantity, ToQuantity, QuantityMath, make_quantity};

#[no_mangle]
pub extern "C" fn stats_average(input: Vec<(String, Table)>) -> Table {
  let (argument, table_ref) = &input[0];
  let out = if argument == "row" {
    let mut out = Table::new(0,table_ref.rows,1);
    for i in 0..table_ref.rows as usize {
      let mut value = 0.0;
      for j in 0..table_ref.columns as usize {
        value = value + &table_ref.data[j][i].as_float().unwrap();
      }
      out.data[0][i] = Value::from_quantity((value / table_ref.rows as f64).to_quantity());
    }
    out
  } else if argument == "column" {
    let mut out = Table::new(0,1,table_ref.columns);
    for i in 0..table_ref.columns as usize {
      let mut value = 0.0;
      for j in 0..table_ref.rows as usize {
        value = value + &table_ref.data[i][j].as_float().unwrap();
      }
      out.data[i][0] = Value::from_quantity((value / table_ref.rows as f64).to_quantity());
    }
    out
  } else if argument == "table" {
    let mut out = Table::new(0,1,1);
    let mut value = 0.0;
    for i in 0..table_ref.columns as usize {
      for j in 0..table_ref.rows as usize {
        value = value + &table_ref.data[i][j].as_float().unwrap();
      }
      out.data[0][0] = Value::from_quantity((value / (table_ref.rows * table_ref.columns) as f64).to_quantity());
    }
    out
  } else {
    Table::new(0,1, 1)
  };
  out 
}