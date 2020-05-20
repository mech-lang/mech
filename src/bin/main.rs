extern crate mech_core;


use mech_core::{Core, Index, Value, Change, Transaction, Transformation, Block};

use std::time::{Duration, SystemTime};
use std::io;
use std::io::prelude::*;

fn main() {


  let balls = 100_000;

  print!("Allocating memory...");
  let mut core = Core::new(balls * 4 * 4);
  println!("Done!");

  let mut txn = Transaction{
    changes: vec![
      Change::NewTable{table_id: 123, rows: balls, columns: 4},
    ]
  };
  let mut values = vec![];
  for i in 1..balls+1 {
    let mut v = vec![
      (Index::Index(i), Index::Index(1), Value::from_u64(i as u64)),
      (Index::Index(i), Index::Index(2), Value::from_u64(i as u64)),
      (Index::Index(i), Index::Index(3), Value::from_u64(20)),
      (Index::Index(i), Index::Index(4), Value::from_u64(0)),
    ];
    values.append(&mut v);
  }
  txn.changes.push(Change::Set{table_id: 123, values});
  core.process_transaction(txn);

  let mut txn = Transaction{
    changes: vec![
      Change::NewTable{table_id: 456, rows: 1, columns: 1},
      Change::Set{table_id: 456, values: vec![(Index::Index(1), Index::Index(1), Value::from_u64(9))]},
      Change::NewTable{table_id: 789, rows: 1, columns: 2},
      Change::Set{table_id: 789, values: vec![
        (Index::Index(1), Index::Index(1), Value::from_u64(10)),
        (Index::Index(1), Index::Index(2), Value::from_u64(0))
      ]},
    ]
  };

  core.process_transaction(txn);
  
  let mut block = Block::new(1000);
  block.register_transformation(Transformation::Whenever{table_id: 789, row: Index::All, column: Index::Index(2)});
  block.register_transformation(Transformation::Function{
    name: 0x13166E07A8EF9CC3, 
    lhs_table: 123, lhs_column: Index::Index(1), 
    rhs_table: 123, rhs_column: Index::Index(3),
    output_table: 123, output_column: Index::Index(1)
  });
  block.register_transformation(Transformation::Function{
    name: 0x13166E07A8EF9CC3, 
    lhs_table: 123, lhs_column: Index::Index(2), 
    rhs_table: 123, rhs_column: Index::Index(4),
    output_table: 123, output_column: Index::Index(2)
  });
  block.register_transformation(Transformation::Function{
    name: 0x13166E07A8EF9CC3, 
    lhs_table: 123, lhs_column: Index::Index(3), 
    rhs_table: 123, rhs_column: Index::Index(4),
    output_table: 123, output_column: Index::Index(4)
  });


  //println!("{:?}", core.database);
  //println!("{:?}", block);

  core.runtime.register_block(block);


 
  // Hand compile this...
  /*
  ~ #time/timer.ticks
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity"#);*
  */


 
  print!("Running computation...");
  io::stdout().flush().unwrap();
  let rounds = 100.0;
  let start_ns = time::precise_time_ns(); 
  for j in 0..rounds as usize {
    let txn = Transaction{
      changes: vec![
        Change::Set{table_id: 789, values: vec![(Index::Index(1), Index::Index(2), Value::from_u64(j as u64))]}
      ]
    };
    /*let mut values = vec![];
    for i in 1..balls+1 {
      let mut v = vec![
        (Index::Index(i), Index::Index(1), Value::from_u64(i as u64)),
        (Index::Index(i), Index::Index(2), Value::from_u64(i as u64)),
        (Index::Index(i), Index::Index(3), Value::from_u64(20)),
        (Index::Index(i), Index::Index(4), Value::from_u64(0)),
      ];
      values.append(&mut v);
      /*match core.database.tables.get(&123) {
        Some(table) => {
          // Set the value
          let mut t = table.borrow_mut();
          t.set(Index::Index(i), Index::Index(1), Value::from_u64(j as u64));
          t.set(Index::Index(i), Index::Index(2), Value::from_u64(j as u64));
          t.set(Index::Index(i), Index::Index(4), Value::from_u64(j as u64));
          // Mark the table as updated
          //self.changed_this_round.insert(Register{table_id, row: Index::All, column}.hash());
        },
        None => {
          // TODO Throw an error here and roll back all changes
        }
      }*/
      //table.borrow_mut().set(row, column, value);
      /*
      let v3;
      {
        let s = store.borrow();
        let v1 = &s.data[table.get(i,1).unwrap()];
        let v2 = &s.data[table.get(i,3).unwrap()];
        v3 = v1.as_quantity().unwrap().add(v2.as_quantity().unwrap()).unwrap();
      }
      let v3 = Value::from_quantity(v3);
      table.set(i,1,v3);
    
      let v3;
      {
        let s = store.borrow();
        let v1 = &s.data[table.get(i,2).unwrap()];
        let v2 = &s.data[table.get(i,4).unwrap()];
        v3 = v1.as_quantity().unwrap().add(v2.as_quantity().unwrap()).unwrap();
      }
      let v3 = Value::from_quantity(v3);
      table.set(i,2,v3);
    
      let v3;
      {
        let s = store.borrow();
        let v1 = &s.data[table.get(i,4).unwrap()];
        let v2 = &s.data[gravity.get(1,1).unwrap()];
        v3 = v1.as_quantity().unwrap().add(v2.as_quantity().unwrap()).unwrap();
      }
      let v3 = Value::from_quantity(v3);
      table.set(i,4,v3);*/
    }
    /*let mut txn = Transaction{
      changes: vec![
        Change::Set{table_id: 123, values},
      ]
    };*/*/
    core.process_transaction(txn.clone());
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f64 / 1000000.0;   
  let per_iteration_time = time / rounds;
  println!("Done!");
  println!("{:?}s total", time / 1000.0);  
  println!("{:?}ms per iteration", per_iteration_time);  

  println!("{:?}", core);
  

}

