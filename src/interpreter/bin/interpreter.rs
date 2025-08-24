//use mech_interpreter::*;
//use mech_core::*;
//#[cfg(feature = "serde")]
//use bincode::config::standard;

fn main() {
  /*// 1. Create a new interpreter
  let mut intrp = Interpreter::new(0);

  // 2. Include the serialized code at compile time
  //    Suppose you saved it previously as "program.mecb" using bincode::encode_to_vec
  let serialized_code: &[u8] = include_bytes!("bar.mecb");

  // 3. Deserialize into MechSourceCode
  #[cfg(feature = "serde")]
  let (decoded_code, _len): (MechSourceCode, _) =
      bincode::serde::decode_from_slice(serialized_code, standard()).unwrap();
  let decoded_code: MechSourceCode = MechSourceCode::String("This is a test".to_string());
  match decoded_code {
    MechSourceCode::Program(code_vec) => {
      for c in code_vec {
        match c { 
          MechSourceCode::Tree(tree) => {
            // 4. Interpret the tree
            match intrp.interpret(&tree) {
              Ok(result) => {
                //println!("Interpretation Result: {:?}", result);
              }
              Err(err) => {
                eprintln!("Error during interpretation: {:?}", err);
              }
            }
          },
          _ => {
            //println!("Skipping non-tree MechSourceCode variant.");
          }
        }
      }
    }
    _ => {
      //println!("The included file does not contain a valid Mech program.");
    }
  }
  //println!("Loaded interpreter code: {:?}", intrp.code);*/
}
