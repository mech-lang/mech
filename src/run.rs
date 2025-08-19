
use mech_syntax::*;
use mech_core::*;
use std::time::Instant;
use crate::*;

#[macro_export]
macro_rules! print_tree {
  ($tree:expr) => {
    #[cfg(feature = "pretty_print")]
    println!("{}", $tree.pretty_print());
    #[cfg(not(feature = "pretty_print"))]
    println!("{:#?}", $tree);
  };
}

#[macro_export]
macro_rules! print_symbols {
  ($intrp:expr) => {
    #[cfg(feature = "pretty_print")]
    println!("{}",$intrp.pretty_print_symbols());  
    #[cfg(not(feature = "pretty_print"))]
    println!("{:#?}", $intrp.symbols());
  };
}

#[macro_export]
macro_rules! print_plan {
  ($intrp:expr) => {
    #[cfg(feature = "pretty_print")]
    println!("{}", $intrp.plan().pretty_print());
    #[cfg(not(feature = "pretty_print"))]
    println!("{:#?}", $intrp.plan());
  };
}


pub fn run_mech_code(intrp: &mut Interpreter, code: &MechFileSystem, tree_flag: bool, debug_flag: bool, time_flag: bool) -> MResult<Value> {
  let sources = code.sources();
  let sources = sources.read().unwrap();
  for (file,source) in sources.sources_iter() {
    match source {
      MechSourceCode::Program(ref code_vec) => {
        for c in code_vec {
          match c {
            MechSourceCode::Tree(tree) => {
              if tree_flag {
                print_tree!(tree);
              }
              let now = Instant::now();
              let result = intrp.interpret(tree);
              let elapsed_time = now.elapsed();
              let cycle_duration = elapsed_time.as_nanos() as f64;
              if time_flag {
                println!("Cycle Time: {} ns", cycle_duration);
              }
              if debug_flag {
                print_symbols!(intrp);
                print_plan!(intrp);
              }
              return result;
            },
            _ => todo!(),
          }
        }
      }
      MechSourceCode::String(s) => {
        let now = Instant::now();
        let parse_result = parser::parse(&s.trim());
        let elapsed_time = now.elapsed();
        let parse_duration = elapsed_time.as_nanos() as f64;
        match parse_result {
          Ok(tree) => { 
            if tree_flag {
              print_tree!(tree);
            }
            let now = Instant::now();
            let result = intrp.interpret(&tree);
            let elapsed_time = now.elapsed();
            let cycle_duration = elapsed_time.as_nanos() as f64;
            if time_flag {
              println!("Parse Time: {} ns", parse_duration);
            }
            if time_flag {
              println!("Cycle Time: {} ns", cycle_duration);
            }
            if debug_flag {
              print_symbols!(intrp);
              print_plan!(intrp); 
            }
            return result;
          },
          Err(err) => return Err(err),
        }
      }
      MechSourceCode::ByteCode(program) => {
        let now = Instant::now();
        let result = intrp.run_program(&program);
        let elapsed_time = now.elapsed();
        let cycle_duration = elapsed_time.as_nanos() as f64;
        if time_flag {
          println!("Cycle Time: {} ns", cycle_duration);
        }
        if debug_flag {
          print_symbols!(intrp);
          print_plan!(intrp);
        }
        return result;
      }
      x => todo!("Unsupported source code type: {:?}", x),
    }
  }
  Ok(Value::Empty)
}