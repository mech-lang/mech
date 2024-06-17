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
    match args.get(1) {
      Some(filename) => {
        let s = fs::read_to_string(&filename).unwrap();
        match parser2::parse(&s) {
          Ok(tree) => { 
            println!("\n-------------------------------- 🌳 Syntax Tree --------------------------------\n");
            let tree_hash = hash_str(&format!("{:#?}", tree));
            println!("Tree Hash: {:?}", tree_hash);
            println!("{:#?}", tree);
            let mut intrp = Interpreter::new();
            let result = intrp.interpret(&tree);
            println!("\n-------------------------------- 💻 Interpreter --------------------------------\n");
            println!("Symbols: {:#?}", intrp.symbols); 
            println!("Plan:"); 
            for (ix,fxn) in intrp.plan.borrow().iter().enumerate() {
              println!("  {}. {}", ix + 1, fxn.to_string());
            }
            for fxn in intrp.plan.borrow().iter() {
              fxn.solve();
            }
            println!("\n---------------------------------- 🌟 Result ----------------------------------\n");
            println!("Result:  {:#?}", result);
          },
          Err(err) => {
            if let MechErrorKind::ParserError(tree, report, _) = err.kind {
              parser::print_err_report(&s, &report);
            } else {
              panic!("Unexpected error type");
            }
          }
        }
      }
      None => println!("Missing path to .mec file."),
    }
  Ok(())
}
