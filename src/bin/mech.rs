#![feature(hash_extract_if)]
#![allow(warnings)]
use mech::*;
use mech_core::*;
use mech_syntax::parser;
//use mech_syntax::analyzer::*;
use mech_interpreter::interpreter::*;
use std::time::Instant;
use std::fs;
use std::env;
use std::io;
use nom::{
  IResult,
  bytes::complete::tag,
  branch::alt,
  bytes::complete::take_while,
  combinator::opt,
  character::complete::{space0,space1,digit1},
};
use colored::*;
use std::io::{Write, BufReader, BufWriter, stdout};
use crossterm::{
  ExecutableCommand, QueueableCommand,
  terminal, cursor, style::Print,
};
use clap::{arg, command, value_parser, Arg, ArgAction, Command};
use std::path::PathBuf;
use tabled::{
  builder::Builder,
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};
use serde_json;
use std::panic;

#[derive(Debug)]
enum ReplCommand {
  Help,
  Quit,
  Symbols(Option<String>),
  Plan,
  Whos(Option<String>),
  Clear(Option<String>),
  Clc,
  Load(String),
  Save(String),
  Step(Option<usize>),
  Unknown(String),  
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn parse_repl_command(input: &str) -> IResult<&str, ReplCommand> {
  let (input, _) = tag(":")(input)?;
  let (input, command) = alt((
    step_rpl,
    save_rpl,
    help_rpl,
    quit_rpl,
    symbols_rpl,
    plan_rpl,
    whos_rpl,
    clear_rpl,
    clc_rpl,
    load_rpl,
  ))(input)?;
  Ok((input, command))
}

fn help_rpl(input: &str) -> IResult<&str, ReplCommand> {
  let (input, _) = alt((tag("h"), tag("help")))(input)?;
  Ok((input, ReplCommand::Help))
}

fn quit_rpl(input: &str) -> IResult<&str, ReplCommand> {
  let (input, _) = alt((tag("q"), tag("quit"), tag("exit")))(input)?;
  Ok((input, ReplCommand::Quit))
}

fn symbols_rpl(input: &str) -> IResult<&str, ReplCommand> {
  let (input, _) = alt((tag("s"), tag("symbols")))(input)?;
  let (input, _) = space0(input)?;
  let (input, name) = opt(take_while(|c: char| c.is_alphanumeric()))(input)?;
  Ok((input, ReplCommand::Symbols(name.map(|s| s.to_string()))))
}

fn plan_rpl(input: &str) -> IResult<&str, ReplCommand> {
  let (input, _) = alt((tag("p"), tag("plan")))(input)?;
  Ok((input, ReplCommand::Plan))
}

fn whos_rpl(input: &str) -> IResult<&str, ReplCommand> {
  let (input, _) = alt((tag("w"), tag("whos")))(input)?;
  let (input, _) = space0(input)?;
  let (input, name) = opt(take_while(|c: char| c.is_alphanumeric()))(input)?;
  Ok((input, ReplCommand::Whos(name.map(|s| s.to_string()))))
}

fn clear_rpl(input: &str) -> IResult<&str, ReplCommand> {
  let (input, _) = tag("clear")(input)?;
  Ok((input, ReplCommand::Clear(None)))
}

fn clc_rpl(input: &str) -> IResult<&str, ReplCommand> {
  let (input, _) = tag("clc")(input)?;
  Ok((input, ReplCommand::Clc))
}

fn load_rpl(input: &str) -> IResult<&str, ReplCommand> {
  let (input, _) = tag("load")(input)?;
  let (input, _) = space1(input)?;
  Ok((input, ReplCommand::Load(input.to_string())))
}

fn save_rpl(input: &str) -> IResult<&str, ReplCommand> {
  let (input, _) = tag("save")(input)?;
  let (input, _) = space1(input)?;
  let (input, path) = take_while(|c: char| c.is_alphanumeric())(input)?;
  Ok((input, ReplCommand::Save(path.to_string())))
}

fn step_rpl(input: &str) -> IResult<&str, ReplCommand> {
  let (input, _) = tag("step")(input)?;
  let (input, _) = space0(input)?;
  let (input, count) = opt(digit1)(input)?;
  Ok((input, ReplCommand::Step(count.map(|s| s.parse().unwrap()))))
}


fn main() -> Result<(), MechError> {
  /*panic::set_hook(Box::new(|panic_info| {
    if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
      println!("Mech Language Error: {}", s);
      // Check for underflow error message
      if s.contains("underflow") {
          println!("Underflow error occurred!");
      }
    } else {
        println!("Mech Language Error: Unknown panic");
    }
  }));*/
  
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
    .arg(Arg::new("tree")
        .long("tree")
        .help("Print parse tree")
        .action(ArgAction::SetTrue))   
    .arg(Arg::new("time")
        .short('t')
        .long("time")
        .help("Measure how long the programs takes to execute.")
        .action(ArgAction::SetTrue))       
    .arg(Arg::new("repl")
        .short('r')
        .long("repl")
        .help("Start REPL")
        .action(ArgAction::SetTrue))
    .get_matches();

  let debug_flag = matches.get_flag("debug");
  let tree_flag = matches.get_flag("tree");
  let repl_flag = matches.get_flag("repl");
  let time_flag = matches.get_flag("time");

  let mut intrp = Interpreter::new();
  if let Some(mech_paths) = matches.get_one::<String>("mech_paths") {
    let s = fs::read_to_string(&mech_paths).unwrap();
    
    let now = Instant::now();
    let parse_result = parser::parse(&s);
    let elapsed_time = now.elapsed();
    let parse_duration = elapsed_time.as_nanos() as f64;

    match parse_result {
      Ok(tree) => { 
        let now = Instant::now();
        let result = intrp.interpret(&tree);
        let elapsed_time = now.elapsed();
        let cycle_duration = elapsed_time.as_nanos() as f64;
        
        let result_str = match result {
          Ok(r) => format!("{}", r.pretty_print()),
          Err(err) => format!("{:?}", err),
        };

        if debug_flag {
          println!("{}", intrp.symbols.borrow().pretty_print());
          println!("{}", pretty_print_plan(&intrp));
        } 
        if tree_flag {
          println!("{}", pretty_print_tree(&tree));
        }
        if time_flag {
          println!("Parse Time:   {:0.2?} ns", parse_duration);
          println!("Compile Time: {:0.2?} ns", cycle_duration);
        }

        println!("{}", result_str);
      },
      Err(err) => {
        if let MechErrorKind::ParserError(report, _) = err.kind {
          parser::print_err_report(&s, &report);
        } else {
          panic!("Unexpected error type");
        }
      }
    }
    if !repl_flag {
      return Ok(());
    }
  } 
  
  #[cfg(windows)]
  control::set_virtual_terminal(true).unwrap();
  clc();
  let mut stdo = stdout();
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

    if input.chars().nth(0) == Some(':') {
      // loop
      let repl_command = parse_repl_command(&input.as_str());

      match repl_command {
        Ok((_, ReplCommand::Help)) => println!("Mech REPL Commands: \n  :help, :h - Display this help message\n  :quit, :q, :exit - Quit the REPL\n  :symbols, :s - Display all symbols\n  :plan, :p - Display the plan\n  :whos, :w - Display all symbols\n  :clear - Clear the screen\n  :clc - Clear the screen\n  :load - Load a file\n  :save - Save a file\n  :step - Step through the plan"),
        Ok((_, ReplCommand::Quit)) => break 'REPL,
        Ok((_, ReplCommand::Symbols(name))) => println!("{}", intrp.symbols.borrow().pretty_print()),
        Ok((_, ReplCommand::Plan)) => println!("{}", pretty_print_plan(&intrp)),
        Ok((_, ReplCommand::Whos(name))) => println!("{}",whos(&intrp)),
        Ok((_, ReplCommand::Clear(name))) => {
          // Drop the old interpreter replace it with a new one
          intrp = Interpreter::new();
        }
        Ok((_, ReplCommand::Clc)) => clc(),
        Ok((_, ReplCommand::Load(path))) => {
          println!("Loading: {:?}", path.trim());
          let s = match fs::read_to_string(&path.trim()) {
            Ok(s) => s,
            Err(err) => {
              println!("Error reading file: {:?} {:?}", err, path);
              continue;
            }
          };
          let now = Instant::now();
          let parse_result = parser::parse(&s);
          let elapsed_time = now.elapsed();
          let parse_duration = elapsed_time.as_nanos() as f64;
          match parse_result {
            Ok(tree) => { 
              let now = Instant::now();
              let result = intrp.interpret(&tree);
              let elapsed_time = now.elapsed();
              let cycle_duration = elapsed_time.as_nanos() as f64;
              let result_str = match result {
                Ok(r) => format!("{}", r.pretty_print()),
                Err(err) => format!("{:?}", err),
              };
              println!("{}", result_str);
            },
            Err(err) => {
              if let MechErrorKind::ParserError(report, _) = err.kind {
                parser::print_err_report(&s, &report);
              } else {
                panic!("Unexpected error type");
              }
            }
          }
        }
        Ok((_, ReplCommand::Step(count))) => {
          let n = match count {
            Some(n) => n,
            None => 1,
          };
          let plan = intrp.plan.as_ptr();
          let plan_brrw = unsafe { &*plan };
          let now = Instant::now();
          for i in 0..n {
            for fxn in plan_brrw {
              fxn.solve();
            }
          }
          let elapsed_time = now.elapsed();
          let cycle_duration = elapsed_time.as_nanos() as f64;
          println!("{:0.2?} ns", cycle_duration);
        }
        x => {
          let err = MechError{
            file: file!().to_string(),  
            tokens: vec![],
            msg: "".to_string(),
            id: line!(),
            kind: MechErrorKind::UnknownCommand(input.clone()),
          };
          println!("{:?}",x);
        }
      }
    } else if input.trim() == "" {
      // loop
    } else {
      // Treat as code
      match parser::parse(&input) {
        Ok(tree) => { 
          let now = Instant::now();
          let result = intrp.interpret(&tree);
          let elapsed_time = now.elapsed();
          let cycle_duration = elapsed_time.as_nanos() as f64;

          match result {
            Ok(r) => println!("{:?}\n{}", r.kind(), r.pretty_print()),
            Err(err) => println!("{:?}", err),
          }
          println!("{:0.2?} ns", cycle_duration);

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
  }
  
  Ok(())
}
