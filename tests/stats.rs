extern crate mech_core;
extern crate mech_utilities;
extern crate mech_stats;
use mech_core::{Value, Table, Index};
use mech_stats::{stats_average};
use mech_core::{Quantity, ToQuantity, QuantityMath, make_quantity};


/*
#[test]
fn average_test() {

  let mut table = Table::new(0,3,2);
  
  table.set_cell(&Index::Index(1), &Index::Index(1), Value::from_u64(1));
  table.set_cell(&Index::Index(1), &Index::Index(2), Value::from_u64(2));
  table.set_cell(&Index::Index(2), &Index::Index(1), Value::from_u64(3));
  table.set_cell(&Index::Index(2), &Index::Index(2), Value::from_u64(4));
  table.set_cell(&Index::Index(3), &Index::Index(1), Value::from_u64(5));
  table.set_cell(&Index::Index(3), &Index::Index(2), Value::from_u64(6));


  let result = stats_average(vec![("row".to_string(),table.clone())]);
  assert_eq!(result.index(&Index::Index(1), &Index::Index(1)), 
             Some(&Value::from_quantity(((1.0 + 2.0) / 2.0).to_quantity())));
  assert_eq!(result.index(&Index::Index(2), &Index::Index(1)), 
             Some(&Value::from_quantity(((3.0 + 4.0) / 2.0).to_quantity())));
  assert_eq!(result.index(&Index::Index(3), &Index::Index(1)), 
             Some(&Value::from_quantity(((5.0 + 6.0) / 2.0).to_quantity())));


  let result = stats_average(vec![("column".to_string(),table.clone())]);
  assert_eq!(result.index(&Index::Index(1), &Index::Index(1)), 
             Some(&Value::from_quantity(((1.0 + 3.0 + 5.0) / 3.0).to_quantity())));
  assert_eq!(result.index(&Index::Index(1), &Index::Index(2)), 
             Some(&Value::from_quantity(((2.0 + 4.0 + 6.0) / 3.0).to_quantity())));

  let result = stats_average(vec![("table".to_string(),table)]);
  assert_eq!(result.index(&Index::Index(1), &Index::Index(1)), 
            Some(&Value::from_quantity(((1.0 + 2.0 + 3.0 + 4.0 + 5.0 + 6.0) / 6.0).to_quantity())));

}*/