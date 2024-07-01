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
use std::io;
use colored::*;
use std::io::{Write, BufReader, BufWriter, stdout};
use crossterm::{
  ExecutableCommand, QueueableCommand,
  terminal, cursor, style::Print,
};
use clap::{arg, command, value_parser, ArgAction, Command};
use std::path::PathBuf;
use tabled::{
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};

fn main() -> Result<(), MechError> {
  let version = "0.2.0";
  let text_logo = r#"
  ┌─────────┐ ┌──────┐ ┌─┐ ┌──┐ ┌─┐   ┌─┐
  └───┐ ┌───┘ └──────┘ │ │ └┐ │ │ │   │ │
  ┌─┐ │ │ ┌─┐ ┌──────┐ │ │  └─┘ │ └─┐ │ │
  │ │ │ │ │ │ │ ┌────┘ │ │  ┌─┐ │ ┌─┘ │ │
  │ │ └─┘ │ │ │ └────┐ │ └──┘ │ │ │   │ │
  └─┘     └─┘ └──────┘ └──────┘ └─┘   └─┘"#.truecolor(246,192,78);



  let matches = command!() // requires `cargo` feature
                .arg(arg!([mech_paths] "Optional Mech source target")
                  .required(false)
                  .value_parser(value_parser!(PathBuf)))
                .arg(
                    arg!(
                        -c --config <FILE> "Sets a custom config file"
                    )
                    .required(false)
                    .value_parser(value_parser!(PathBuf)),
                )
                .arg(arg!(
                    -d --debug ... "Turn debugging information on"
                ))
                .get_matches();

  if let Some(mech_paths) = matches.get_one::<PathBuf>("mech_paths") {
    let s = fs::read_to_string(&mech_paths).unwrap();
    match parser2::parse(&s) {
      Ok(tree) => { 
        let mut intrp = Interpreter::new();
        let result = intrp.interpret(&tree);
        
        let tree_hash = hash_str(&format!("{:#?}", tree));
        let syntax_tree_str = format!("Tree Hash: {:?}\n{:#?}", tree_hash, tree);

        let mut interpreter_str = format!("Symbols: {:#?}\n", intrp.symbols); 
        interpreter_str = format!("{}Plan:\n", interpreter_str); 
        for (ix,fxn) in intrp.plan.borrow().iter().enumerate() {
          interpreter_str = format!("{}  {}. {}\n", interpreter_str, ix + 1, fxn.to_string());
        }
        interpreter_str = format!("{}Fxns:\n", interpreter_str); 
        for (id,fxn) in intrp.functions.borrow().functions.iter() {
          println!("{:?}", fxn);
        }
        for fxn in intrp.plan.borrow().iter() {
          fxn.solve();
        }
        let result_str = format!("{:#?}", result);

        let data = vec!["🌳 Syntax Tree", &syntax_tree_str, 
                        "💻 Interpreter", &interpreter_str, 
                        "🌟 Result",      &result_str];
        let mut table = tabled::Table::new(data);
        table
            .with(Style::modern())
            .with(Panel::header(format!("Runtime Debug Info")))
            .with(Alignment::left());
        println!("{table}");
      },
      Err(err) => {
        if let MechErrorKind::ParserError(tree, report, _) = err.kind {
          parser::print_err_report(&s, &report);
        } else {
          panic!("Unexpected error type");
        }
      }
    }
  } else {
    #[cfg(windows)]
    control::set_virtual_terminal(true).unwrap();
    let mut stdo = stdout();
    stdo.execute(terminal::Clear(terminal::ClearType::All));
    stdo.execute(cursor::MoveTo(0,0));
    stdo.execute(Print(text_logo));
    stdo.execute(cursor::MoveToNextLine(1));
    println!(" {}",  "╔═══════════════════════════════════════╗".bright_black());
    println!(" {}                 {}                {}", "║".bright_black(), format!("v{}",version).truecolor(246,192,78), "║".bright_black());
    println!(" {}           {}           {}", "║".bright_black(), "www.mech-lang.org", "║".bright_black());
    println!(" {}\n",  "╚═══════════════════════════════════════╝".bright_black());


    let mut intrp = Interpreter::new();
    'REPL: loop {
      io::stdout().flush().unwrap();
      // Print a prompt 
      // 4, 8, 15, 16, 23, 42
      print!("{}", ">: ".truecolor(246,192,78));
      io::stdout().flush().unwrap();
      let mut input = String::new();
      io::stdin().read_line(&mut input).unwrap();
      match parser2::parse(&input) {
        Ok(tree) => { 
          let result = intrp.interpret(&tree);
          println!("{:?}", result);
        }
        Err(err) => {
          if let MechErrorKind::ParserError(tree, report, _) = err.kind {
            parser::print_err_report(&input, &report);
          } else {
            panic!("Unexpected error type");
          }
        }
      }

    }
  }
  Ok(())
}
