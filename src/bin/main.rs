use mech_syntax::parser;
use mech_syntax::ast::Ast;
use mech_syntax::compiler::Compiler;
use mech_core::*;

use std::cell::RefCell;
use std::rc::Rc;

fn main() -> Result<(),MechError> {

let input = r#"
controller
  #player/controller = [left-y: 0.5]

sphero
  #sphero/foo = [|id<string> speed<f32> heading<f32>|]

sphero
  #sphero/foo += [id: "SB-C714"]

sphero controls
  #sphero/foo.speed := #player/controller.left-y"#;

  let input = String::from(input);

  let mut ast = Ast::new();
  let mut compiler = Compiler::new();
  let mut core = Core::new();
    let parse_tree = parser::parse(&input)?;
println!("{:#?}", parse_tree);

  ast.build_syntax_tree(&parse_tree);

  println!("{:?}", ast.syntax_tree);

  let blocks = compiler.compile_blocks(&vec![ast.syntax_tree.clone()]).unwrap();
  
  core.load_blocks(blocks);
  println!("{:?}", core);
  core.schedule_blocks()?;
  println!("Done");
  /*
  let ticks = 30;
  // println!("{:#?}", core.get_table("balls").unwrap().borrow());

  let changes = vec![
    Change::Set((hash_str("time/timer"), vec![(TableIndex::Index(1), TableIndex::Index(2), Value::U64(U64::new(1)))])),
    Change::Set((hash_str("time/timer"), vec![(TableIndex::Index(1), TableIndex::Index(2), Value::U64(U64::new(2)))])),
  ];

  core.process_transaction(&changes)?;*/




  /*for i in 1..=ticks {
    let txn = vec![
      Change::Set((hash_str("time/timer"), vec![(TableIndex::Index(1), TableIndex::Index(2), Value::U64(U64::new(i as u64)))])),
    ];
    core.process_transaction(&txn)?;
    println!("{:#?}", core.get_table("balls").unwrap().borrow());
  }
  let txn: Vec<Change> = vec![
    Change::Set((hash_str("time/timer"), vec![(TableIndex::Index(1), TableIndex::Alias(hash_str("ticks")), Value::U64(U64::new(1)))])),
    Change::Set((hash_str("time/timer"), vec![(TableIndex::Index(1), TableIndex::Alias(hash_str("ticks")), Value::U64(U64::new(2)))])),
  ];
  println!("Processing Txn...");
  core.process_transaction(&txn);
  println!("Done Txn.");*/
  println!("{:#?}", core.blocks);

  //println!("Core:");
  println!("{:#?}", core);

  
  /*if let Ok(table) = core.get_table("b") {
    println!("Answer:");
    println!("{:#?}", table.borrow());
  }*/
  Ok(())
}