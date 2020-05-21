extern crate mech_core;


use mech_core::{Core, TableId, Index, Value, Change, Transaction, Transformation, Block};
use std::hash::Hasher;
extern crate ahash;
use ahash::AHasher;
use std::time::{Duration, SystemTime};
use std::io;
use std::io::prelude::*;

fn hash_string(input: &str) -> u64 {
  let mut hasher = AHasher::new_with_keys(329458495230, 245372983457);
  hasher.write(input.to_string().as_bytes());
  hasher.finish()
}

fn main() {

  let balls = 4000;

  print!("Allocating memory...");
  let mut core = Core::new(balls * 4 * 4);
  println!("Done!");

  let mut block = Block::new(6);
  block.register_transformation(Transformation::NewTable{table_id: TableId::Local(0x01), rows: 1, columns: 1});
  block.register_transformation(Transformation::Constant{table_id: TableId::Local(0x01), value: Value::from_u64(0)});
  block.register_transformation(Transformation::NewTable{table_id: TableId::Local(0x02), rows: 1, columns: 1});
  block.register_transformation(Transformation::Constant{table_id: TableId::Local(0x02), value: Value::from_u64(balls as u64)});
  block.register_transformation(Transformation::NewTable{table_id: TableId::Local(0x78), rows: balls, columns: 1});
  block.register_transformation(Transformation::NewTable{table_id: TableId::Local(0x79), rows: balls, columns: 1});
  block.register_transformation(Transformation::NewTable{table_id: TableId::Global(0x123), rows: balls, columns: 4});
  block.register_transformation(Transformation::NewTable{table_id: TableId::Global(0x456), rows: 1, columns: 1});
  block.register_transformation(Transformation::Set{table_id: TableId::Global(0x456), row: Index::Index(1), column: Index::Index(1), value: Value::from_u64(9)});
  block.register_transformation(Transformation::NewTable{table_id: TableId::Global(0x789), rows: 1, columns: 2});
  block.register_transformation(Transformation::Set{table_id: TableId::Global(0x789), row: Index::Index(1), column: Index::Index(1), value: Value::from_u64(16)});
  block.register_transformation(Transformation::Function{
    name: 0x285a4efbfcdc2ef4, 
    arguments: vec![
      (TableId::Local(0x01), Index::All, Index::All), 
      (TableId::Local(0x02), Index::All, Index::All),
    ],
    out: (TableId::Local(0x78), Index::All, Index::All)
  });
  block.register_transformation(Transformation::Function{
    name: 0x285a4efbfcdc2ef4, 
    arguments: vec![
      (TableId::Local(0x01), Index::All, Index::All), 
      (TableId::Local(0x02), Index::All, Index::All),
    ],
    out: (TableId::Local(0x79), Index::All, Index::All)
  });
  block.gen_id();
  core.runtime.register_block(block);

/*
  x = 1:4000
  y = 1:4000
  #ball = [|x   y   vx  vy|
            x   y   20  0]
  #gravity = 1
  #time/timer = [period: 1, ticks: 0]
*/
/*
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
      (Index::Index(i), Index::Index(4), Value::from_u64(1)),
    ];
    values.append(&mut v);
  }
  txn.changes.push(Change::Set{table_id: 123, values});
  core.process_transaction(&txn);

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

  core.process_transaction(&txn);*/

/*
  ~ #time/timer.ticks
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity
*/

  let mut block = Block::new(8500);
  block.register_transformation(Transformation::Whenever{table_id: 789, row: Index::All, column: Index::Index(2)});
  block.register_transformation(Transformation::Function{
    name: 0x13166E07A8EF9CC3, 
    arguments: vec![
      (TableId::Global(123), Index::All, Index::Index(1)), 
      (TableId::Global(123), Index::All, Index::Index(3))
    ],
    out: (TableId::Global(123), Index::All, Index::Index(1))
  });
  block.register_transformation(Transformation::Function{
    name: 0x13166E07A8EF9CC3, 
    arguments: vec![
      (TableId::Global(123), Index::All, Index::Index(2)), 
      (TableId::Global(123), Index::All, Index::Index(4)),
    ],
    out: (TableId::Global(123), Index::All, Index::Index(2))
  });
  block.register_transformation(Transformation::Function{
    name: 0x13166E07A8EF9CC3, 
    arguments: vec![
      (TableId::Global(123), Index::All, Index::Index(4)), 
      (TableId::Global(456), Index::All, Index::All),
    ],
    out: (TableId::Global(123), Index::All, Index::Index(4))
  });
  block.gen_id();

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
  let rounds = 0.0;
  let start_ns = time::precise_time_ns(); 
  for j in 0..rounds as usize {
    let txn = Transaction{
      changes: vec![
        Change::Set{table_id: 789, values: vec![(Index::Index(1), Index::Index(2), Value::from_u64(j as u64))]}
      ]
    };
    core.process_transaction(&txn);
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f64 / 1000000.0;   
  let per_iteration_time = time / rounds;
  println!("Done!");
  println!("{:0.4?}s total", time / 1000.0);  
  println!("{:0.4?}ms per iteration", per_iteration_time);  

  println!("{:?}", core);
  
  

}

