extern crate mech_syntax;
extern crate mech_core;

use mech_syntax::compiler::{Compiler, Node, Element};
use mech_syntax::formatter::Formatter;
use mech_core::Block;
use mech_core::{Change, Transaction};
use mech_core::{Value, Index};
use mech_core::hash_string;
use mech_core::Core;
use mech_core::{make_quantity, Quantity, ValueMethods, ToQuantity, QuantityMath};
use std::time::{Duration, SystemTime};
use std::mem;

use std::rc::Rc;

fn main() {

  /*let input = String::from(r#"# Bouncing Balls

Define the environment
  x = 1:4000
  #gravity = 1
  #ball = [|x   y   vx  vy|
            x   x   20  3 ]

Update the block positions on each tick of the timer
  ~ #time/timer.ticks
  #ball.x := #ball.x + #ball.vx
    #ball.y := #ball.y + #ball.vy
    #ball.vy := #ball.vy + #gravity"#);*/

// Some primitives
  let input = String::from(r#"
block
  #test = #i

block
  #i = [x: 2]

block
  #i.x{#i <= 6} := #i.x + 1"#);

  //let value = Value::Number(make_quantity(780000,-4,0));
  //compile_test(input.clone(), value);

  let mut compiler = Compiler::new();
  let mut formatter = Formatter::new();
  let mut core = Core::new(1_000_000, 10);
  core.load_standard_library();
  let programs = compiler.compile_string(input.clone());

  //println!("{:?}", programs);
  //println!("{:?}", compiler.blocks);
  //println!("{:?}", compiler.parse_tree);
  //println!("{:?}", compiler.syntax_tree);
  //core.runtime.register_block(compiler.blocks);
  core.runtime.register_block(compiler.blocks[0].clone());
  core.runtime.register_block(compiler.blocks[1].clone());
  core.runtime.register_block(compiler.blocks[2].clone());
  //core.runtime.register_block(compiler.blocks[3].clone());
  //core.step();
  //let y = core.get_table_by_name("y").unwrap();
  //println!("{:?}",y);
  //core.runtime.register_block(compiler.blocks[4].clone());
  //let y = core.get_table_by_name("y").unwrap();
  //println!("{:?}",y);
  //core.step();
  println!("{:?}", core);
  //println!("{:?}", compiler.parse_tree);
  //println!("{:?}", compiler.unparsed);
  //println!("{:?}", compiler.syntax_tree);

  

  /*
  let x: u64 = 37678279552074374;
  println!("{:064b}", x);
  println!("{:064b}", hash_string("slider"));

  let change = Change::Set{
    table_id: 37678279552074374, values: vec![ 
      (Index::Index(1),
       Index::Alias(0xcb672312fe42b4),
       Value::from_i64(75)),
    ]
  };

  let txn = Transaction{changes: vec![change]};

  core.process_transaction(&txn);
*//*
  let x = core.get_table(hash_string("x")).unwrap();
  let y = core.get_table(hash_string("y")).unwrap();

  println!("{:?}", x);
  
  println!("{:?}", x.transaction_boundaries);
  println!("{:?}", x.history);

  println!("{:?}", y);
  println!("{:?}", y.transaction_boundaries);
  println!("{:?}", y.history);*/
  //core.step(100000);
  
  /*
  let changes = vec![
    Rc::new(Change::NewTable{id: 0xd2d75008, rows: 0, columns: 2}),
    Rc::new(Change::RenameColumn{table: 0xd2d75008, column_ix: 1, column_alias: 0x6972c9df}),
    Rc::new(Change::RenameColumn{table: 0xd2d75008, column_ix: 2, column_alias: 0x6b6369e7}),
  ];

  let txn = Transaction::from_changeset(changes);

  core.process_transaction(&txn);

  let txn = Transaction::from_change(Rc::new(Change::Set{
    table: 0xd2d75008, 
    column: Index::Alias(0x6972c9df), 
    values: vec![(Index::Index(1), Rc::new(Value::from_u64(16)))]
  }));

  core.process_transaction(&txn);

  extern crate time;

  let mut counter = 0;
  let start_ns = time::precise_time_ns();
  let rounds = 2000.0;
  for i in 0..rounds as u64 {
    let txn = Transaction::from_change(Rc::new(Change::Set{
      table: 0xd2d75008, 
      column: Index::Alias(0x6b6369e7), 
      values: vec![(Index::Index(1), Rc::new(Value::from_u64(counter)))],
    }));
    core.process_transaction(&txn);
    counter = counter + 1;
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f64 / 1000000.0;   
  let per_iteration_time = time / rounds;
  
  */
/*
  println!("{:?}", core);
  println!("{:?}", core.runtime);

  println!("{:?}", std::mem::size_of::<Value>());

  let mut v: Vec<Vec<Value>> = vec![];

  let qq = 4000;

  for i in 0..4 {
    let mut q = vec![];
    for j in 0..qq as usize {
      q.push(Value::from_u64(j as u64));
    }
    v.push(q);
  }
  
  let gravity = Value::from_u64(1);

  let rounds = 1000.0;

  let start_ns = time::precise_time_ns();
  for i in 0..rounds as usize {
    for j in 0..qq {
      v[0][j] = Value::from_quantity(v[0][j].as_quantity().unwrap().add(v[2][j].as_quantity().unwrap()).unwrap());
      v[1][j] = Value::from_quantity(v[1][j].as_quantity().unwrap().add(v[3][j].as_quantity().unwrap()).unwrap());
      v[3][j] = Value::from_quantity(v[3][j].as_quantity().unwrap().add(gravity.as_quantity().unwrap()).unwrap());
    }
  }
  
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f64 / 1000000.0;   
  let per_iteration_time = time / rounds;
  println!("{:?}s total", time / 1000.0);  
  println!("{:?}ms per iteration", per_iteration_time);  */

  //println!("{:?}s total", time / 1000.0);  
  //println!("{:?}ms per iteration", per_iteration_time);  

  /*
  let rows = 10000;
  let columns = 4;
  let mut vec1: Vec<Vec<Rc<Value>>> = vec![vec![]];
  vec1.resize(columns, vec![]);
  for i in 0..columns {
    for j in 0..rows {
      vec1[i].resize(rows, Rc::new(Value::Empty));
    }
  }

  let value = Rc::new(Value::from_string("Hello world".to_string()));
  let start_ns = time::precise_time_ns();
  for i in 0..columns {
    for j in 0..rows {
      vec1[i][j] = value.clone();
    }
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f64;   
  let per_iteration_time = time;
  println!("{:?}ns per iteration", per_iteration_time);  


  let mut vec2: Vec<Rc<Value>> = Vec::with_capacity(rows*columns);
  vec2.resize((rows*columns), Rc::new(Value::Empty));

  let value = Rc::new(Value::from_string("Hello world".to_string()));
  let start_ns = time::precise_time_ns();
  for i in 0..(rows*columns) {
    vec2[i] = value.clone();
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f64;   
  let per_iteration_time = time;
  println!("{:?}ns per iteration", per_iteration_time);  

*/

/*
  let value = Value::from_string("Hello world".to_string());
  let rounds = 10000.0;

  
  let rows = 10000;
  let columns = 10;
  let mut table = vec![vec![Value::Empty; rows as usize]; columns as usize];


  let start_ns = time::precise_time_ns();
  for i in 0..columns as usize {
    for j in 0..rows as usize {
      table[i][j] = value.clone();
    }
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f64;   
  let per_iteration_time = time / (rows * columns) as f64;
  println!("{:?}ns per iteration", per_iteration_time);  

  use std::rc::Rc;
  let value = Rc::new(Value::from_string("Hello world".to_string()));
  let rows = 10000;
  let columns = 10;
  let mut table = vec![vec![Rc::new(Value::Empty); rows as usize]; columns as usize];


  let start_ns = time::precise_time_ns();
  for i in 0..columns as usize {
    for j in 0..rows as usize {
      table[i][j] = value.clone();
    }
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f64;   
  let per_iteration_time = time / (rows * columns) as f64;
  println!("{:?}ns per iteration", per_iteration_time);  */


  /*let block_ast = match &programs[0].sections[0].elements[1] {
  Element::Block((id, node)) => node,
    _ => &Node::Null,
  };
  formatter.format(&block_ast);*/
  
  
  //let now = SystemTime::now();
  /*let change = Change::Set{table: 0x132537277, 
                            row: Index::Index(1), 
                            column: Index::Index(3),
                            value: Value::from_u64(42),
                          };
  let txn = Transaction::from_change(change.clone());

  core.process_transaction(&txn);*/
  //println!("{:?}", core);
  //println!("{:?}", core.runtime);
  /*
  match now.elapsed() {
    Ok(elapsed) => {
      // it prints '2'
      let time: f32 = elapsed.as_millis() as f32;
      println!("{}ms", time / n as f32);
    }
    Err(e) => {
      // an error occurred!
      println!("Error: {:?}", e);
    }
  }*/
  //println!("{:?}", core);

}

/*
This program doesn't execute correctly.
block
  #i = [x: 2]
  #h = [53; 100; 85]
  #p = [|x   y|
         400 500 
         0   0
         0   0
         0   0]
  #angle = [10; 20; 30]
 
block
  #i.x{#i.x <= 6} := #i.x + 1

block 
  ~ #i.x
  i = #i
  i2 = i / 2
  ir = math/round(column: i2)
  a = #angle{i2,:}
  #p.x{i} := #p.x{i - 1} + #h{i2,:} * math/sin(degrees: a)
  #p.y{i} := #p.y{i - 1} - #h{i2,:} * math/cos(degrees: a)

  */
  