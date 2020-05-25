extern crate mech_core;

use mech_core::{Core, Table, TableId, Index, Value, Change, Transaction, Transformation, Block, Store, QuantityMath, Quantity};
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

  let period = hash_string("period");
  let ticks = hash_string("ticks");
  let time_timer = hash_string("time/timer");
  let gravity = hash_string("gravity");
  let balls_id = hash_string("balls");
  let x_id = hash_string("x");
  let y_id = hash_string("y");
  let vx_id = hash_string("vx");
  let vy_id = hash_string("vy");
  let math_add = hash_string("math/add");
  let table_range = hash_string("table/range");
  let table_horzcat = hash_string("table/horizontal-concatenate");


/*
  x = 1:4000
  y = 1:4000
  #ball = [|x   y   vx  vy|
            x   y   20  0]
  #gravity = 1
  #time/timer = [period: 1, ticks: 0]
*/

  let mut block = Block::new(balls * 10 * 10);
  block.identifiers.insert(time_timer, "time/timer");
  block.identifiers.insert(period, "period");
  block.identifiers.insert(ticks, "ticks");
  block.identifiers.insert(gravity, "gravity");
  block.identifiers.insert(balls_id, "balls");
  block.identifiers.insert(x_id, "x");
  block.identifiers.insert(y_id, "y");
  block.identifiers.insert(vx_id, "vx");
  block.identifiers.insert(vy_id, "vy");
  block.identifiers.insert(table_range, "table/range");
  block.identifiers.insert(table_horzcat, "table/horizontal-concatenate");
  block.register_transformation(Transformation::NewTable{table_id: TableId::Local(0x01), rows: 1, columns: 1});
  block.register_transformation(Transformation::Constant{table_id: TableId::Local(0x01), value: Value::from_u64(0)});
  block.register_transformation(Transformation::NewTable{table_id: TableId::Local(0x02), rows: 1, columns: 1});
  block.register_transformation(Transformation::Constant{table_id: TableId::Local(0x02), value: Value::from_u64(balls as u64)});
  block.register_transformation(Transformation::NewTable{table_id: TableId::Local(0x03), rows: 1, columns: 1});
  block.register_transformation(Transformation::Constant{table_id: TableId::Local(0x03), value: Value::from_u64(20)});
  block.register_transformation(Transformation::NewTable{table_id: TableId::Local(0x04), rows: 1, columns: 1});
  block.register_transformation(Transformation::Constant{table_id: TableId::Local(0x04), value: Value::from_u64(0)});
  block.register_transformation(Transformation::NewTable{table_id: TableId::Local(x_id), rows: balls, columns: 1});
  block.register_transformation(Transformation::NewTable{table_id: TableId::Local(y_id), rows: balls, columns: 1});
  block.register_transformation(Transformation::NewTable{table_id: TableId::Global(balls_id), rows: balls, columns: 4});
  block.register_transformation(Transformation::ColumnAlias{table_id: TableId::Global(balls_id), column_ix: 1, column_alias: x_id});
  block.register_transformation(Transformation::ColumnAlias{table_id: TableId::Global(balls_id), column_ix: 2, column_alias: y_id});
  block.register_transformation(Transformation::ColumnAlias{table_id: TableId::Global(balls_id), column_ix: 3, column_alias: vx_id});
  block.register_transformation(Transformation::ColumnAlias{table_id: TableId::Global(balls_id), column_ix: 4, column_alias: vy_id});
  block.register_transformation(Transformation::NewTable{table_id: TableId::Global(gravity), rows: 1, columns: 1});
  block.register_transformation(Transformation::Set{table_id: TableId::Global(gravity), row: Index::Index(1), column: Index::Index(1), value: Value::from_u64(9)});
  block.register_transformation(Transformation::NewTable{table_id: TableId::Global(time_timer), rows: 1, columns: 2});
  block.register_transformation(Transformation::ColumnAlias{table_id: TableId::Global(time_timer), column_ix: 1, column_alias: period});
  block.register_transformation(Transformation::ColumnAlias{table_id: TableId::Global(time_timer), column_ix: 2, column_alias: ticks});
  block.register_transformation(Transformation::Set{table_id: TableId::Global(time_timer), row: Index::Index(1), column: Index::Index(1), value: Value::from_u64(16)});
  block.register_transformation(Transformation::Function{
    name: table_range, 
    arguments: vec![
      (TableId::Local(0x01), Index::All, Index::All), 
      (TableId::Local(0x02), Index::All, Index::All),
    ],
    out: (TableId::Local(x_id), Index::All, Index::All)
  });
  block.register_transformation(Transformation::Function{
    name: table_range, 
    arguments: vec![
      (TableId::Local(0x01), Index::All, Index::All), 
      (TableId::Local(0x02), Index::All, Index::All),
    ],
    out: (TableId::Local(y_id), Index::All, Index::All)
  });
  block.register_transformation(Transformation::Function{
    name: table_horzcat, 
    arguments: vec![
      (TableId::Local(x_id), Index::All, Index::All), 
      (TableId::Local(y_id), Index::All, Index::All),
      (TableId::Local(0x03), Index::All, Index::All),
      (TableId::Local(0x04), Index::All, Index::All),
    ],
    out: (TableId::Global(balls_id), Index::All, Index::All)
  });
  block.gen_id();
  core.runtime.register_block(block);

/*
  ~ #time/timer.ticks
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity
*/


  let mut block = Block::new(balls * 10 * 10);
  block.identifiers.insert(time_timer, "time/timer");
  block.identifiers.insert(ticks, "ticks");
  block.identifiers.insert(gravity, "gravity");
  block.identifiers.insert(balls_id, "balls");
  block.identifiers.insert(x_id, "x");
  block.identifiers.insert(y_id, "y");
  block.identifiers.insert(vx_id, "vx");
  block.identifiers.insert(vy_id, "vy");
  block.identifiers.insert(vy_id, "vy");
  block.identifiers.insert(math_add, "math/add");
  block.register_transformation(Transformation::Whenever{table_id: time_timer, row: Index::All, column: Index::Alias(ticks)});
  block.register_transformation(Transformation::Function{
    name: math_add, 
    arguments: vec![
      (TableId::Global(balls_id), Index::All, Index::Alias(x_id)), 
      (TableId::Global(balls_id), Index::All, Index::Alias(vx_id))
    ],
    out: (TableId::Global(balls_id), Index::All, Index::Alias(x_id))
  });
  block.register_transformation(Transformation::Function{
    name: math_add, 
    arguments: vec![
      (TableId::Global(balls_id), Index::All, Index::Alias(y_id)), 
      (TableId::Global(balls_id), Index::All, Index::Alias(vy_id)),
    ],
    out: (TableId::Global(balls_id), Index::All, Index::Alias(y_id))
  });
  block.register_transformation(Transformation::Function{
    name: math_add, 
    arguments: vec![
      (TableId::Global(balls_id), Index::All, Index::Alias(vy_id)), 
      (TableId::Global(gravity), Index::All, Index::All),
    ],
    out: (TableId::Global(balls_id), Index::All, Index::Alias(vy_id))
  });
  block.gen_id();

  core.runtime.register_block(block);

  print!("Running computation...");
  io::stdout().flush().unwrap();
  let rounds = 1000.0;
  let start_ns = time::precise_time_ns(); 
  for j in 0..rounds as usize {
    let txn = Transaction{
      changes: vec![
        Change::Set{table_id: time_timer, values: vec![(Index::Index(1), Index::Index(2), Value::from_u64(j as u64))]}
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

