#![feature(test)]
#![feature(get_mut_unchecked)]

extern crate test;

use test::Bencher;

extern crate mech_core;

use mech_core::{Core, hash_string, Register, humanize, Table, TableId, Index, Value, ValueMethods, IndexIterator, IndexRepeater, Change, Transaction, Transformation, Block, Store, QuantityMath, Quantity};
use std::hash::Hasher;
use std::time::{Duration, SystemTime};
use std::io;
use std::io::prelude::*;
use std::sync::Arc;


fn init(balls: usize) -> Core {

  let mut core = Core::new(balls * 4 * 4);
  core.load_standard_library();

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
  let table_set = hash_string("table/set");

  let mut block = Block::new(balls * 10 * 10);
  let store = unsafe{&mut *Arc::get_mut_unchecked(&mut block.store)};
  store.strings.insert(time_timer, "time/timer".to_string());
  store.strings.insert(period, "period".to_string());
  store.strings.insert(ticks, "ticks".to_string());
  store.strings.insert(gravity, "gravity".to_string());
  store.strings.insert(balls_id, "balls".to_string());
  store.strings.insert(x_id, "x".to_string());
  store.strings.insert(y_id, "y".to_string());
  store.strings.insert(vx_id, "vx".to_string());
  store.strings.insert(vy_id, "vy".to_string());
  store.strings.insert(table_range, "table/range".to_string());
  store.strings.insert(table_horzcat, "table/horizontal-concatenate".to_string());
  
  let range1 = Transformation::Function{
    name: table_range, 
    arguments: vec![
      (0, TableId::Local(0x01), Index::All, Index::All), 
      (0, TableId::Local(0x02), Index::All, Index::All),
    ],
    out: (TableId::Local(x_id), Index::All, Index::All)
  };
  block.register_transformations(("x = 1:4000".to_string(), vec![
    Transformation::NewTable{table_id: TableId::Local(0x01), rows: 1, columns: 1},
    Transformation::Constant{table_id: TableId::Local(0x01), value: Value::from_u64(1), unit: 0},
    Transformation::NewTable{table_id: TableId::Local(0x02), rows: 1, columns: 1},
    Transformation::Constant{table_id: TableId::Local(0x02), value: Value::from_u64(balls as u64), unit: 0},
    range1.clone(),
    Transformation::NewTable{table_id: TableId::Local(x_id), rows: balls, columns: 1},
  ]));
  
  let range2 = Transformation::Function{
    name: table_range, 
    arguments: vec![
      (0, TableId::Local(0x01), Index::All, Index::All), 
      (0, TableId::Local(0x02), Index::All, Index::All),
    ],
    out: (TableId::Local(y_id), Index::All, Index::All)
  };
  block.register_transformations(("y = 1:4000".to_string(), vec![
    Transformation::NewTable{table_id: TableId::Local(0x01), rows: 1, columns: 1},
    Transformation::Constant{table_id: TableId::Local(0x01), value: Value::from_u64(1), unit: 0},
    Transformation::NewTable{table_id: TableId::Local(0x02), rows: 1, columns: 1},
    Transformation::Constant{table_id: TableId::Local(0x02), value: Value::from_u64(balls as u64), unit: 0},
    range2.clone(),
    Transformation::NewTable{table_id: TableId::Local(y_id), rows: balls, columns: 1},
  ]));
  
  let horzcat = Transformation::Function{
    name: table_horzcat, 
    arguments: vec![
      (0, TableId::Local(x_id), Index::All, Index::All), 
      (0, TableId::Local(y_id), Index::All, Index::All),
      (0, TableId::Local(0x03), Index::All, Index::All),
      (0, TableId::Local(0x04), Index::All, Index::All),
    ],
    out: (TableId::Global(balls_id), Index::All, Index::All)
  };
  block.register_transformations((r#"#ball = [|x   y   vx  vy|
x   y   20  0]"#.to_string(), vec![
    Transformation::NewTable{table_id: TableId::Local(0x03), rows: 1, columns: 1},
    Transformation::Constant{table_id: TableId::Local(0x03), value: Value::from_u64(20), unit: 0},
    Transformation::NewTable{table_id: TableId::Local(0x04), rows: 1, columns: 1},
    Transformation::Constant{table_id: TableId::Local(0x04), value: Value::from_u64(1), unit: 0},
    horzcat.clone(),
    Transformation::NewTable{table_id: TableId::Global(balls_id), rows: balls, columns: 4},
    Transformation::ColumnAlias{table_id: TableId::Global(balls_id), column_ix: 1, column_alias: x_id},
    Transformation::ColumnAlias{table_id: TableId::Global(balls_id), column_ix: 2, column_alias: y_id},
    Transformation::ColumnAlias{table_id: TableId::Global(balls_id), column_ix: 3, column_alias: vx_id},
    Transformation::ColumnAlias{table_id: TableId::Global(balls_id), column_ix: 4, column_alias: vy_id},
  ]));

  block.register_transformations(("#gravity = 9".to_string(), vec![
    Transformation::NewTable{table_id: TableId::Global(gravity), rows: 1, columns: 1},
    Transformation::Constant{table_id: TableId::Global(gravity), value: Value::from_u64(9), unit: 0},
  ]));


  let horzcat2 = Transformation::Function{
    name: table_horzcat, 
    arguments: vec![
      (0, TableId::Local(0x07), Index::All, Index::All),
      (0, TableId::Local(0x08), Index::All, Index::All),
    ],
    out: (TableId::Global(time_timer), Index::All, Index::All)
  };
  block.register_transformations(("#time/timer = [period: 16, ticks: 0]".to_string(), vec![
    Transformation::NewTable{table_id: TableId::Global(time_timer), rows: 1, columns: 2},
    Transformation::ColumnAlias{table_id: TableId::Global(time_timer), column_ix: 1, column_alias: period},
    Transformation::ColumnAlias{table_id: TableId::Global(time_timer), column_ix: 2, column_alias: ticks},
    horzcat2.clone(),
    Transformation::NewTable{table_id: TableId::Local(0x07), rows: 1, columns: 1},
    Transformation::Constant{table_id: TableId::Local(0x07), value: Value::from_u64(16), unit: 0},
    Transformation::NewTable{table_id: TableId::Local(0x08), rows: 1, columns: 1},
    Transformation::Constant{table_id: TableId::Local(0x08), value: Value::from_u64(0), unit: 0},
  ]));

  block.plan.push(range1);
  block.plan.push(range2);
  block.plan.push(horzcat);
  block.plan.push(horzcat2);
  
  block.gen_id();
  core.runtime.register_block(block);


  let mut block = Block::new(balls * 10 * 10);
  let store = unsafe{&mut *Arc::get_mut_unchecked(&mut block.store)};
  store.strings.insert(time_timer, "time/timer".to_string());
  store.strings.insert(ticks, "ticks".to_string());
  store.strings.insert(gravity, "gravity".to_string());
  store.strings.insert(balls_id, "balls".to_string());
  store.strings.insert(x_id, "x".to_string());
  store.strings.insert(y_id, "y".to_string());
  store.strings.insert(vx_id, "vx".to_string());
  store.strings.insert(vy_id, "vy".to_string());
  store.strings.insert(vy_id, "vy".to_string());
  store.strings.insert(math_add, "math/add".to_string());
  store.strings.insert(table_set, "table/set".to_string());

  let whenever = Transformation::Whenever{
    table_id: TableId::Global(time_timer), 
    row: Index::All, 
    column: Index::Alias(ticks), 
    registers: vec![Register{table_id: TableId::Global(time_timer), row: Index::All, column: Index::Alias(ticks)}.hash()]
  };

  block.register_transformations(("~ #time/timer.ticks".to_string(), vec![
    whenever.clone(),
  ]));
  
  let xmove = Transformation::Function{
    name: math_add, 
    arguments: vec![
      (0, TableId::Global(balls_id), Index::All, Index::Alias(x_id)), 
      (0, TableId::Global(balls_id), Index::All, Index::Alias(vx_id))
    ],
    out: (TableId::Local(0x9), Index::All, Index::All)
  };
  let xset = Transformation::Function{
    name: table_set, 
    arguments: vec![
      (0, TableId::Local(0x9), Index::All, Index::All), 
    ],
    out: (TableId::Global(balls_id), Index::All, Index::Alias(x_id))
  };
  block.register_transformations(("#ball.x := #ball.x + #ball.vx".to_string(), vec![
    Transformation::NewTable{table_id: TableId::Local(0x09), rows: 1, columns: 1},
    xset.clone(),
    xmove.clone(),
  ]));


  let ymove = Transformation::Function{
    name: math_add, 
    arguments: vec![
      (0, TableId::Global(balls_id), Index::All, Index::Alias(y_id)), 
      (0, TableId::Global(balls_id), Index::All, Index::Alias(vy_id)),
    ],
    out: (TableId::Local(0x08), Index::All, Index::All)
  };
  let yset = Transformation::Function{
    name: table_set, 
    arguments: vec![
      (0, TableId::Local(0x8), Index::All, Index::All), 
    ],
    out: (TableId::Global(balls_id), Index::All, Index::Alias(y_id))
  };
  block.register_transformations(("#ball.y := #ball.y + #ball.vy".to_string(), vec![
    Transformation::NewTable{table_id: TableId::Local(0x08), rows: 1, columns: 1},
    ymove.clone(),
    yset.clone(),
  ]));


  let vymove = Transformation::Function{
    name: math_add, 
    arguments: vec![
      (0, TableId::Global(balls_id), Index::All, Index::Alias(vy_id)), 
      (0, TableId::Global(gravity), Index::All, Index::All),
    ],
    out: (TableId::Local(0x07), Index::All, Index::All)
  };
  let vyset = Transformation::Function{
    name: table_set, 
    arguments: vec![
      (0, TableId::Local(0x7), Index::All, Index::All), 
    ],
    out: (TableId::Global(balls_id), Index::All, Index::Alias(vy_id))
  };
  block.register_transformations(("#ball.vy := #ball.vy + g".to_string(), vec![
    Transformation::NewTable{table_id: TableId::Local(0x07), rows: 1, columns: 1},
    vymove.clone(),
    vyset.clone(),
  ]));


  


  /*
  block.register_transformations(("#ball.y := #ball.y + #ball.vy".to_string(), vec![

  ]));
  block.register_transformations(("#ball.vy := #ball.vy + #gravity".to_string(), vec![
    Transformation::Function{
      name: math_add, 
      arguments: vec![
        (0, TableId::Global(balls_id), Index::All, Index::Alias(vy_id)), 
        (0, TableId::Global(gravity), Index::All, Index::All),
      ],
      out: (TableId::Global(balls_id), Index::All, Index::Alias(vy_id))
    },
  ]));*/
  block.plan.push(whenever);
  block.plan.push(xmove);
  block.plan.push(xset);
  block.plan.push(ymove);
  block.plan.push(yset);
  block.plan.push(vymove);
  block.plan.push(vyset);
  block.gen_id();
  core.runtime.register_block(block);
  core
}



#[bench]
fn balls_1_000(b:&mut Bencher){

  let mut core = init(1000);
  let time_timer = hash_string("time/timer");
  let ticks = hash_string("ticks");
  let txn = Transaction{
  changes: vec![
    Change::Set{table_id: time_timer, values: vec![(Index::Index(1), Index::Alias(ticks), Value::from_u64(1 as u64))]}
  ]};
  b.iter(|| {
    core.process_transaction(&txn);
  });

}

#[bench]
fn balls_10_000(b:&mut Bencher){

  let mut core = init(10000);
  let time_timer = hash_string("time/timer");
  let ticks = hash_string("ticks");
  let txn = Transaction{
  changes: vec![
    Change::Set{table_id: time_timer, values: vec![(Index::Index(1), Index::Alias(ticks), Value::from_u64(1 as u64))]}
  ]};
  b.iter(|| {
    core.process_transaction(&txn);
  });

}


#[bench]
fn balls_100_000(b:&mut Bencher){

  let mut core = init(100000);
  let time_timer = hash_string("time/timer");
  let ticks = hash_string("ticks");
  let txn = Transaction{
  changes: vec![
    Change::Set{table_id: time_timer, values: vec![(Index::Index(1), Index::Alias(ticks), Value::from_u64(1 as u64))]}
  ]};
  b.iter(|| {
    core.process_transaction(&txn);
  });

}


