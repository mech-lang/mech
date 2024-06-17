#![feature(hash_extract_if)]
#![allow(warnings)]
use mech_syntax::parser;
use mech_syntax::ast::Ast;
use mech_syntax::compiler::Compiler;
use mech_core::*;
use mech_syntax::parser2;
//use mech_syntax::analyzer::*;
use mech_syntax::interpreter::*;
use std::time::Instant;
use std::fs;
use std::env;

#[tokio::main]
async fn main() -> Result<(), MechError> {
  
    let args: Vec<_> = env::args().collect();
    let s = fs::read_to_string(&args[1]).unwrap();
    match parser2::parse(&s) {
        Ok(tree) => { 
          println!("----------- SYNTAX TREE ---------");
          println!("{:#?}", tree);
          let mut intrp = Interpreter::new();
          let result = intrp.interpret(&tree);
          println!("R: {:#?}", result);
          println!("{:#?}", intrp.symbols); 
          for fxn in intrp.plan.borrow().iter() {
            println!("{:?}", fxn.to_string());
          }
          for fxn in intrp.plan.borrow().iter() {
            fxn.solve();
          }
          let tree_string = hash_str(&format!("{:#?}", tree));
          println!("{:?}", tree_string);
        },
        Err(err) => {
          println!("{:?}", err);          
          if let MechErrorKind::ParserError(node, report, _) = err.kind {
            println!("----- TREE -----");
            println!("{:?}", node);
            println!("----- MESSAGE -----");
            parser::print_err_report(&s, &report);
          } else {
            panic!("Unexpected error type");
          }
        }
    }

  Ok(())
}
