
use mech_syntax::*;
use mech_core::*;
use std::time::Instant;
use crate::*;

pub fn run_mech_code(intrp: &mut Interpreter, code: &MechFileSystem, tree_flag: bool, debug_flag: bool, time_flag: bool) -> MResult<Value> {
  let sources = code.sources();
  let sources = sources.read().unwrap();
  for (file,source) in sources.sources_iter() {
    match source {
      MechSourceCode::String(s) => {
        let now = Instant::now();
        let parse_result = parser::parse(&s.trim());
        let elapsed_time = now.elapsed();
        let parse_duration = elapsed_time.as_nanos() as f64;
        match parse_result {
          Ok(tree) => { 
            if tree_flag {
              println!("{}", &tree.pretty_print());
            }
            let now = Instant::now();
            let result = intrp.interpret(&tree);
            let elapsed_time = now.elapsed();
            let cycle_duration = elapsed_time.as_nanos() as f64;
            if time_flag {
              println!("Parse Time: {} ns", parse_duration);
              println!("Cycle Time: {} ns", cycle_duration);
            }
            if debug_flag {
              println!("{}", pretty_print_symbols(&intrp));
              println!("{}", pretty_print_plan(&intrp)); 
            }
            return result;
          },
          Err(err) => return Err(err),
        }
      }
      _ => todo!(),
    }
  }
  Ok(Value::Empty)
}