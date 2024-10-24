#![feature(hash_extract_if)]
#![allow(warnings)]
use mech::format_parse_tree;
use mech_core::*;
use mech_syntax::parser;
//use mech_syntax::analyzer::*;
use mech_core::interpreter::*;
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
use clap::{arg, command, value_parser, Arg, ArgAction, Command};
use std::path::PathBuf;
use tabled::{
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};
use serde_json;
use std::panic;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), MechError> {
  panic::set_hook(Box::new(|panic_info| {
    if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
      println!("Mech Language Error: {}", s);
      // Check for underflow error message
      if s.contains("underflow") {
          println!("Underflow error occurred!");
      }
    } else {
        println!("Mech Language Error: Unknown panic");
    }
  }));
  
  let text_logo = r#"
  ┌─────────┐ ┌──────┐ ┌─┐ ┌──┐ ┌─┐   ┌─┐
  └───┐ ┌───┘ └──────┘ │ │ └┐ │ │ │   │ │
  ┌─┐ │ │ ┌─┐ ┌──────┐ │ │  └─┘ │ └─┐ │ │
  │ │ │ │ │ │ │ ┌────┘ │ │  ┌─┐ │ ┌─┘ │ │
  │ │ └─┘ │ │ │ └────┐ │ └──┘ │ │ │   │ │
  └─┘     └─┘ └──────┘ └──────┘ └─┘   └─┘"#.truecolor(246,192,78);

  let about = format!("{}", text_logo);

  let matches = Command::new("Mech")
    .version(VERSION)
    .author("Corey Montella corey@mech-lang.org")
    .about(about)
    .arg(Arg::new("mech_paths")
        .help("Source .mec and .blx files")
        .required(false)
        .action(ArgAction::Append))
    .arg(Arg::new("debug")
        .short('d')
        .long("debug")
        .help("Print debug info")
        .action(ArgAction::SetTrue))
    .arg(Arg::new("repl")
        .short('r')
        .long("repl")
        .help("Start REPL")
        .action(ArgAction::SetTrue))
    .get_matches();

  let mut intrp = Interpreter::new();
  if let Some(mech_paths) = matches.get_one::<String>("mech_paths") {
    let s = fs::read_to_string(&mech_paths).unwrap();
    match parser::parse(&s) {
      Ok(tree) => { 
        let result = intrp.interpret(&tree);
        let pretty_json = format_parse_tree(&tree);

        let debug_flag = matches.get_flag("debug");
        if debug_flag {
          let tree_hash = hash_str(&format!("{:#?}", tree));
          let syntax_tree_str = format!("Tree Hash: {:?}\n{}", tree_hash, pretty_json);

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
          let result_str = match result {
            Ok(r) => format!("{}", r.pretty_print()),
            Err(err) => format!("{:?}", err),
          };

          let data = vec!["🌳 Syntax Tree", &syntax_tree_str, 
                          "💻 Interpreter", &interpreter_str, 
                          "🌟 Result",      &result_str];
          let mut table = tabled::Table::new(data);
          table.with(Style::modern())
               .with(Panel::header(format!("Runtime Debug Info")))
               .with(Alignment::left());
    
          println!("{table}");
        } else {
          let result_str = match result {
            Ok(r) => format!("{}", r.pretty_print()),
            Err(err) => format!("{:?}", err),
          };
          println!("{}", result_str);
        }
      },
      Err(err) => {
        if let MechErrorKind::ParserError(report, _) = err.kind {
          parser::print_err_report(&s, &report);
        } else {
          panic!("Unexpected error type");
        }
      }
    }
    let repl_flag = matches.get_flag("repl");
    if !repl_flag {
      return Ok(());
    }
  } 
  
  #[cfg(windows)]
  control::set_virtual_terminal(true).unwrap();
  let mut stdo = stdout();
  stdo.execute(terminal::Clear(terminal::ClearType::All));
  stdo.execute(cursor::MoveTo(0,0));
  stdo.execute(Print(text_logo));
  stdo.execute(cursor::MoveToNextLine(1));
  println!("\n                {}                ",format!("v{}",VERSION).truecolor(246,192,78));
  println!("           {}           \n", "www.mech-lang.org");

  'REPL: loop {
    io::stdout().flush().unwrap();
    // Print a prompt 
    // 4, 8, 15, 16, 23, 42
    print!("{}", ">: ".truecolor(246,192,78));
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    match parser::parse(&input) {
      Ok(tree) => { 
        let now = Instant::now();
        let result = intrp.interpret(&tree);
        let elapsed_time = now.elapsed();
        let cycle_duration = elapsed_time.as_nanos() as f64;

        match result {
          Ok(r) => println!("{}", r.pretty_print()),
          Err(err) => println!("{:?}", err),
        }
        println!("{:0.2?} ns", cycle_duration / 1000000.0);

      }
      Err(err) => {
        if let MechErrorKind::ParserError(report, _) = err.kind {
          parser::print_err_report(&input, &report);
        } else {
          panic!("Unexpected error type");
        }
      }
    }
  }
  
  Ok(())
}
