use mech_interpreter::*;
use mech_core::*;
use mech_syntax::*;
fn main() {
  let mut intrp = Interpreter::new(0);
  match parser::parse("~x := [1 2 3 4 5]; x += [10 20 30 40 50]") {
    Ok(tree) => {
      let mut intrp = Interpreter::new(0);
      let _ = intrp.interpret(&tree).unwrap();
      let bytecode = intrp.compile().unwrap();
      match ParsedProgram::from_bytes(&bytecode) {
        Ok(prog) => {
          match intrp.run_program(&prog) {
            Ok(result) => {
              println!("Program executed successfully. Result: {}", result);
            },
            Err(e) => {
              eprintln!("Error running program: {:?}", e);
            }
          }
        },
        Err(e) => {
          eprintln!("Error deserializing program: {:?}", e);
        }
      }
    },
    Err(err) => { panic!("{:?}", err); }
  }
  /*let mut intrp = Interpreter::new(0);
  let serialized_code: &[u8] = include_bytes!("output.mecb");
  #[cfg(feature = "program")]
  match ParsedProgram::from_bytes(serialized_code) {
    Ok(prog) => {
      match intrp.run_program(&prog) {
        Ok(result) => {
          println!("Program executed successfully. Result: {}", result);
        },
        Err(e) => {
          eprintln!("Error running program: {:?}", e);
        }
      }
    },
    Err(e) => {
      eprintln!("Error deserializing program: {:?}", e);
    }
  }*/
}

