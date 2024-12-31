
use mech_syntax::*;
use mech_core::*;
use std::time::Instant;
use crate::*;

pub fn parse_and_run_mech_code(paths: &Vec<String>, intrp: &mut Interpreter) -> MResult<Value> {
  match read_mech_files(&paths) {
    Ok(code) => {
      for c in code {
        match c {
          (file,MechSourceCode::String(s)) => {
            let now = Instant::now();
            let parse_result = parser::parse(&s.trim());
            let elapsed_time = now.elapsed();
            let parse_duration = elapsed_time.as_nanos() as f64;
            match parse_result {
              Ok(tree) => { 
                let now = Instant::now();
                let result = intrp.interpret(&tree);
                let elapsed_time = now.elapsed();
                let cycle_duration = elapsed_time.as_nanos() as f64;
                let result_str = match result {
                  Ok(ref r) => format!("{}", r.pretty_print()),
                  Err(ref err) => format!("{:?}", err),
                };
                println!("\n{}\n", result_str);
                return result;
              },
              Err(err) => return Err(err),
            }
          }
          _ => todo!(),
        }
      }
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::NoCode});
    }
    Err(err) => todo!(),
  }
}