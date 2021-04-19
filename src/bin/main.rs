#![feature(get_mut_unchecked)]

extern crate mech_core;

use mech_core::{Register, Table, TableId, Database, TableIterator, IndexRepeater, TableIndex, ValueIterator, Value, ValueMethods, Store, IndexIterator, ConstantIterator};
use std::sync::Arc;
use std::cell::RefCell;

extern crate seahash;

fn main() {
  let store = Arc::new(Store::new(10000));
  let database = Arc::new(RefCell::new(Database::new(10000)));
  let mut table = Table::new(1234,2,3,store.clone());
  table.set(&TableIndex::Index(1),&TableIndex::Index(1),Value::from_u64(1));
  table.set(&TableIndex::Index(1),&TableIndex::Index(2),Value::from_u64(2));
  table.set(&TableIndex::Index(1),&TableIndex::Index(3),Value::from_u64(3));
  table.set(&TableIndex::Index(2),&TableIndex::Index(1),Value::from_u64(4));
  table.set(&TableIndex::Index(2),&TableIndex::Index(2),Value::from_u64(5));
  table.set(&TableIndex::Index(2),&TableIndex::Index(3),Value::from_u64(6));


  let store = Arc::new(Store::new(10000));
  let mut table2 = Table::new(5678,1,2,store.clone());
  table2.set(&TableIndex::Index(1),&TableIndex::Index(1),Value::from_u64(1));
  table2.set(&TableIndex::Index(1),&TableIndex::Index(2),Value::from_u64(3));


  let table_id = TableId::Local(0);
  database.borrow_mut().tables.insert(1234, table);
  database.borrow_mut().tables.insert(5678, table2);
  let mut table_ptr = database.borrow_mut().tables.get_mut(&1234).unwrap() as *mut Table;
  let mut table_ptr2 = database.borrow_mut().tables.get_mut(&5678).unwrap() as *mut Table;

  let row_index = TableIndex::All;
  let column_index = TableIndex::All;
  let row_iter = IndexIterator::Range(1..=2);
  let column_iter = IndexIterator::Table(TableIterator::new(table_ptr2));


  /*let row_iter = unsafe { match row_index {
    TableIndex::Index(ix) => IndexIterator::Constant(ConstantIterator::new(TableIndex::Index(*ix))),
    TableIndex::All => {
      match (*table).rows {
        0 => IndexIterator::None,
        r => IndexIterator::Range(1..=r),
      }
    },
    TableIndex::Table(table_id) => {
      let row_table = match table_id {
        TableId::Global(id) => db.tables.get_mut(&id).unwrap() as *mut Table,
        TableId::Local(id) => self.tables.get_mut(&id).unwrap() as *mut Table,
      };
      IndexIterator::Table(TableIterator::new(row_table))
    }
    TableIndex::Alias(alias) => IndexIterator::Alias(AliasIterator::new(*alias, table_id, db.store.clone())),
    _ => IndexIterator::Range(1..=(*table).rows),
  }};

  let column_iter = unsafe { match column_index {
    TableIndex::Index(ix) => IndexIterator::Constant(ConstantIterator::new(TableIndex::Index(*ix))),
    TableIndex::All => {
      match (*table).columns {
        0 => IndexIterator::None,
        c => IndexIterator::Range(1..=c),
      }
    }
    TableIndex::Table(table_id) => {
      let col_table = match table_id {
        TableId::Global(id) => db.tables.get_mut(&id).unwrap() as *mut Table,
        TableId::Local(id) => self.tables.get_mut(&id).unwrap() as *mut Table,
      };
      IndexIterator::Table(TableIterator::new(col_table))
    }
    TableIndex::Alias(alias) => IndexIterator::Alias(AliasIterator::new(*alias, table_id, self.store.clone())),
    TableIndex::None => IndexIterator::None,
    //_ => IndexIterator::Range(1..=(*table).columns),
  }};*/

  /*let to_hash = format!("{:?}{:?}", hash, register.table_id.unwrap());
  println!("{:?}", &to_hash.clone());
  hash = seahash::hash(to_hash.as_bytes());
  println!("{:0x}", hash);

  let to_hash = format!("{:?}{:?}", hash, register.row.unwrap());
  println!("{:?}", &to_hash.clone());
  hash = seahash::hash(to_hash.as_bytes());
  println!("{:0x}", hash);

  let to_hash = format!("{:?}{:?}", hash, register.column.unwrap());
  println!("{:?}", &to_hash.clone());
  hash = seahash::hash(to_hash.as_bytes());
  println!("{:0x}", hash);

  let to_hash = format!("{:?}{:?}{:?}",register.table_id.unwrap(),register.row.unwrap(),register.column.unwrap());
  println!("{:?}", &to_hash.clone());
  hash = seahash::hash(to_hash.as_bytes());
  println!("{:0x}", hash);

  let to_hash = "This is a really long string that I would like to hash, it contains a lot of bytes";
  println!("{:?}", &to_hash.clone());
  hash = seahash::hash(to_hash.as_bytes());
  println!("{:0x}", hash);

  let to_hash = "131176846773215564701311768467732155647";
  println!("{:?}", &to_hash.clone());
  hash = seahash::hash(to_hash.as_bytes());
  println!("{:0x}", hash);*/


  /*
  let balls = 40000;

  print!("Allocating memory...");
  let mut core = Core::new(balls * 4 * 4);
  core.load_standard_library();
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
      (0, TableId::Local(0x01), TableIndex::All, TableIndex::All),
      (0, TableId::Local(0x02), TableIndex::All, TableIndex::All),
    ],
    out: (TableId::Local(x_id), TableIndex::All, TableIndex::All)
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
      (0, TableId::Local(0x01), TableIndex::All, TableIndex::All),
      (0, TableId::Local(0x02), TableIndex::All, TableIndex::All),
    ],
    out: (TableId::Local(y_id), TableIndex::All, TableIndex::All)
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
      (0, TableId::Local(x_id), TableIndex::All, TableIndex::All),
      (0, TableId::Local(y_id), TableIndex::All, TableIndex::All),
      (0, TableId::Local(0x03), TableIndex::All, TableIndex::All),
      (0, TableId::Local(0x04), TableIndex::All, TableIndex::All),
    ],
    out: (TableId::Global(balls_id), TableIndex::All, TableIndex::All)
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
      (0, TableId::Local(0x07), TableIndex::All, TableIndex::All),
      (0, TableId::Local(0x08), TableIndex::All, TableIndex::All),
    ],
    out: (TableId::Global(time_timer), TableIndex::All, TableIndex::All)
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
    row: TableIndex::All,
    column: TableIndex::Alias(ticks),
    registers: vec![Register{table_id: TableId::Global(time_timer), row: TableIndex::All, column: TableIndex::Alias(ticks)}.hash()]
  };

  block.register_transformations(("~ #time/timer.ticks".to_string(), vec![
    whenever.clone(),
  ]));

  let xmove = Transformation::Function{
    name: math_add,
    arguments: vec![
      (0, TableId::Global(balls_id), TableIndex::All, TableIndex::Alias(x_id)),
      (0, TableId::Global(balls_id), TableIndex::All, TableIndex::Alias(vx_id))
    ],
    out: (TableId::Local(0x9), TableIndex::All, TableIndex::All)
  };
  let xset = Transformation::Function{
    name: table_set,
    arguments: vec![
      (0, TableId::Local(0x9), TableIndex::All, TableIndex::All),
    ],
    out: (TableId::Global(balls_id), TableIndex::All, TableIndex::Alias(x_id))
  };
  block.register_transformations(("#ball.x := #ball.x + #ball.vx".to_string(), vec![
    Transformation::NewTable{table_id: TableId::Local(0x09), rows: 1, columns: 1},
    xset.clone(),
    xmove.clone(),
  ]));


  let ymove = Transformation::Function{
    name: math_add,
    arguments: vec![
      (0, TableId::Global(balls_id), TableIndex::All, TableIndex::Alias(y_id)),
      (0, TableId::Global(balls_id), TableIndex::All, TableIndex::Alias(vy_id)),
    ],
    out: (TableId::Local(0x08), TableIndex::All, TableIndex::All)
  };
  let yset = Transformation::Function{
    name: table_set,
    arguments: vec![
      (0, TableId::Local(0x8), TableIndex::All, TableIndex::All),
    ],
    out: (TableId::Global(balls_id), TableIndex::All, TableIndex::Alias(y_id))
  };
  block.register_transformations(("#ball.y := #ball.y + #ball.vy".to_string(), vec![
    Transformation::NewTable{table_id: TableId::Local(0x08), rows: 1, columns: 1},
    ymove.clone(),
    yset.clone(),
  ]));


  let vymove = Transformation::Function{
    name: math_add,
    arguments: vec![
      (0, TableId::Global(balls_id), TableIndex::All, TableIndex::Alias(vy_id)),
      (0, TableId::Global(gravity), TableIndex::All, TableIndex::All),
    ],
    out: (TableId::Local(0x07), TableIndex::All, TableIndex::All)
  };
  let vyset = Transformation::Function{
    name: table_set,
    arguments: vec![
      (0, TableId::Local(0x7), TableIndex::All, TableIndex::All),
    ],
    out: (TableId::Global(balls_id), TableIndex::All, TableIndex::Alias(vy_id))
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
        (0, TableId::Global(balls_id), TableIndex::All, TableIndex::Alias(vy_id)),
        (0, TableId::Global(gravity), TableIndex::All, TableIndex::All),
      ],
      out: (TableId::Global(balls_id), TableIndex::All, TableIndex::Alias(vy_id))
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


  print!("Running computation...");
  io::stdout().flush().unwrap();
  let rounds = 10.0;
  let start_ns = time::precise_time_ns();
  for j in 0..rounds as usize {
    let txn = Transaction{
      changes: vec![
        Change::Set{table_id: time_timer, values: vec![(TableIndex::Index(1), TableIndex::Alias(ticks), Value::from_u64(j as u64))]}
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

*/
}
