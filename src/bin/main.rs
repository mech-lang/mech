extern crate mech_syntax;
extern crate mech_core;

use mech_syntax::compiler::{Compiler, Node, Element};
use mech_syntax::formatter::Formatter;
use mech_core::Block;
use mech_core::{Change, Transaction};
use mech_core::{Value, Index};
use mech_core::Hasher;
use mech_core::Core;
use mech_core::make_quantity;
use std::time::{Duration, SystemTime};

fn compile_test(input: String, test: Value) {
  let mut compiler = Compiler::new();
  let mut core = Core::new(10, 10);
  compiler.compile_string(input);
  core.register_blocks(compiler.blocks);
  core.step();
  let table = Hasher::hash_str("test");
  let row = Index::Index(1);
  let column = Index::Index(1);
  let actual = core.index(table, &row, &column);
  match actual {
    Some(value) => {
      assert_eq!(*value, test);
    },
    None => assert_eq!(0,1),
  }
  
}


fn main() {
  let input = String::from(r#"# Pong Game Client (Client)

Start a timer
  #time/timer = [period: 5000 tick: 0 hours: 0 minutes: 0 seconds: 0]

Draw the game
  #app/main = [root: "drawing" direction: _ contains: [#game]]

controller slider
  #paddle-control = [type: "slider" class: _ contains: _ parameters:  [min: 0 max: 300 value: 0]]

game area
  #game = [|type     class contains    parameters|
            #paddle-control
            "canvas" _     [#elements] [width: 400 height: 400]]

define the controller
  #pong/paddle = #paddle-control{1,4}{1,3}

draw the game area
  pos = #pong/paddle
  start = pos
  end = pos + 100
  #elements = [|shape    parameters|
                "circle" [cx: #pong/ball.x cy: #pong/ball.y radius: 10 fill: "000000"]
                "line"   [x1: start y1: 350 x2: end y2: 350 stroke: "000000"]]

Stub the ball
  #pong/ball = [x: 0 y: 0]"#);
  
  //let value = Value::Number(make_quantity(780000,-4,0));
  //compile_test(input.clone(), value);

  let mut compiler = Compiler::new();
  let mut formatter = Formatter::new();
  let mut core = Core::new(1_000, 250);
  let programs = compiler.compile_string(input.clone());

  println!("{:?}", programs);
 

  core.register_blocks(compiler.blocks.clone());
  //println!("{:?}", compiler.parse_tree);
  println!("{:?}", compiler.unparsed);
  println!("{:?}", compiler.syntax_tree);
  //println!("{:?}", core.runtime);
  core.step();
  println!("{:?}", core);
  println!("{:?}", core.runtime);
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