use mech_core::*;
use mech_syntax::parser;
use mech_syntax::*;
use std::cell::RefCell;
use std::rc::Rc;

//use mech_syntax::analyzer::*;
use mech_core::interpreter::*;
use std::time::Instant;
use std::fs;
extern crate nalgebra as na;
use na::{Vector3, DVector, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix6, Matrix2};


fn main() -> Result<(),MechError> {

    // ----------------------------------------------------------------

    let s = fs::read_to_string("../../test.mec").unwrap();

    match parser::parse(&s) {
        Ok(tree) => { 
          println!("----------- SYNTAX TREE ---------");
          println!("{:?}",hash_str(&format!("{:#?}", tree)));
          println!("{:#?}", tree);
          //let result = analyze(&tree);
          //println!("A: {:#?}", result);
          let mut intrp = Interpreter::new();
          let result = intrp.interpret(&tree)?;
          println!("{}", result.pretty_print());
          println!("{:#?}", intrp.symbols); 
          println!("Plan: ");
          for fxn in intrp.plan.borrow().iter() {
            println!("  - {}", fxn.to_string());
          }

          let now = Instant::now();
          for _ in 0..1e6 as usize {
            for fxn in intrp.plan.borrow().iter() {
              fxn.solve();
            }
          }
          let elapsed_time = now.elapsed();
          let cycle_duration = elapsed_time.as_nanos() as f64;
          println!("{:0.2?} ns", cycle_duration / 1000000.0);


          let tree_string = hash_str(&format!("{:#?}", tree));
          println!("{:?}", tree_string);



          //let mut ast = Ast::new();
          //ast.build_syntax_tree(&tree);
          //println!("----------- AST ---------");
          //println!("{:#?}", ast.syntax_tree);
          /*let mut compiler = Compiler::new(); 
          let sections = compiler.compile_sections(&vec![ast.syntax_tree.clone()]).unwrap();
           let mut core = Core::new();
          core.load_sections(sections); //
          let changes = vec![ //active-tab-ix
            Change::Set((hash_str("active-tab-ix"), vec![(TableIndex::Index(1), TableIndex::Index(1), Value::U64(U64::new(2)))]))
          ];
          core.process_transaction(&changes)?;
          println!("{:#?}", core.blocks);
          println!("{:?}", core);*/ 
        },
        Err(err) => {
          println!("{:?}", err);          
          if let MechErrorKind::ParserError(report, _) = err.kind {
            println!("----- MESSAGE -----");
            parser::print_err_report(&s, &report);
          } else {
            panic!("Unexpected error type");
          }
        }
    }
    return Ok(());
    // ----------------------------------------------------------------

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
