use mech_syntax::parser;
use mech_syntax::ast::Ast;
use mech_syntax::compiler::Compiler;
use mech_core::*;

use std::rc::Rc;

use std::fs;
fn main() -> Result<(),MechError> {
    // ----------------------------------------------------------------
    let s = fs::read_to_string("test.mec").unwrap();
    match parser::parse(&s) {
        Ok(tree) => { 
          println!("----------- SYNTAX TREE ---------");
          println!("{:#?}", tree);
          let mut ast = Ast::new();
          ast.build_syntax_tree(&tree);
          println!("----------- AST ---------");
          println!("{:#?}", ast.syntax_tree);
          let mut compiler = Compiler::new();
          let sections = compiler.compile_sections(&vec![ast.syntax_tree.clone()]).unwrap();
          // let mut core = Core::new();
          // core.load_sections(sections);
          // println!("{:#?}", core.blocks);
          // println!("{:?}", core);
        },
        Err(err) => if let MechErrorKind::ParserError(node, report) = err.kind {
          println!("----- TREE -----");
          println!("{:?}", node);
          println!("----- MESSAGE -----");
          parser::print_err_report(&s, &report);
        } else {
          panic!("Unexpected error type");
        },
    }
    return Ok(());
    // ----------------------------------------------------------------

let input = r#"
[a<f32>] = foo(x<f32>)
  y = 3
  z = x * 2
  a = z + y * 3
y = foo(x: 10)
#test = y"#;
  let input = String::from(input);

  let mut ast = Ast::new();
  let mut compiler = Compiler::new();
  let mut core = Core::new();
  let parse_tree = parser::parse(&input)?;
  println!("{:#?}", parse_tree);

  ast.build_syntax_tree(&parse_tree);

  println!("{:?}", ast.syntax_tree);

  let sections = compiler.compile_sections(&vec![ast.syntax_tree.clone()]).unwrap();
  //println!("{:?}",sections);
  core.load_sections(sections);
  println!("{:#?}", core.blocks);
  println!("{:?}", core);

/*
  let mut code = r#"
Next
  #mech/tables = [|name<string>|
                  "other-a"
                  "other-b"
                  "other-c"]
block
  #q = 123
"#;
  
  let mut compiler = Compiler::new();
  let blocks = compiler.compile_str(&code).unwrap();
  core.load_blocks(blocks);
  
  core.schedule_blocks()?;
  //println!("Done");
  
  let ticks = 30;
  // println!("{:#?}", core.get_table("balls").unwrap().borrow());

  let changes = vec![
    Change::Set((hash_str("button"), vec![(TableIndex::Index(1), TableIndex::Index(1), Value::Bool(true))]))
  ];

  //core.process_transaction(&changes)?;

  let changes = vec![
    Change::Set((hash_str("y"), vec![(TableIndex::Index(1), TableIndex::Index(1), Value::Bool(true))]))
  ];

  let changes = vec![
    Change::Set((hash_str("x"), vec![(TableIndex::Index(1), TableIndex::Index(1), Value::Bool(true))]))
  ];*/


  //core.process_transaction(&changes)?;



  /*for i in 1..=ticks {
    let txn = vec![
      Change::Set((hash_str("time/timer"), vec![(TableIndex::Index(1), TableIndex::Index(2), Value::U64(U64::new(i as u64)))])),
    ];
    core.process_transaction(&txn)?;
    println!("{:#?}", core.get_table("balls").unwrap().borrow());
  }*/
  //let txn: Vec<Change> = vec![
    //Change::Set((hash_str("time/timer"), vec![(TableIndex::Index(1), TableIndex::Alias(hash_str("ticks")), Value::U64(U64::new(1)))])),
    //Change::Set((hash_str("time/timer"), vec![(TableIndex::Index(1), TableIndex::Alias(hash_str("ticks")), Value::U64(U64::new(2)))])),
  //];
  //println!("Processing Txn...");
  //core.process_transaction(&txn);
  //println!("Done Txn.");
  //println!("{:#?}", core.blocks);

  //println!("Core:");
  //println!("{:#?}", core);

  
  /*if let Ok(table) = core.get_table("container") {
    println!("Answer:");
    println!("{:#?}", table.borrow());
  }*/
  Ok(())
}
