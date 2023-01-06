//const SOURCE_FILES: &[(&str, &[u8])] = &include!(concat!(env!("OUT_DIR"), "/mech_app_files.rs"));

extern crate mech;
use mech::core::Core;
use mech::syntax::compiler::Compiler;
use mech::*;

fn main() {

  let mut compiler = Compiler::new();

  let runner = ProgramRunner::new("Mech Run");
  let mech_client = runner.run().unwrap();

  /*for (name, data) in SOURCE_FILES {
    println!("File {} is {} bytes", name, data.len());
    let mech_code = String::from_utf8(data.to_vec()).unwrap();
    let blocks = compiler.compile_str(&mech_code).unwrap();
    mech_client.send(RunLoopMessage::Code(MechCode::MiniBlocks(mech::minify_blocks(&blocks))));
  }*/

  let mech_code = include_str!("../../project/main.mec");
  let blocks = compiler.compile_str(&mech_code).unwrap();
  mech_client.send(RunLoopMessage::Code((1,MechCode::MiniBlocks(mech::minify_blocks(&blocks)))));

  let thread_receiver = mech_client.incoming.clone();
  // Some state variables to control receive loop
  let mut skip_receive = false;
  let mut to_exit = false;
  let mut exit_code = 0;

  // Get all responses from the thread
  'run_receive_loop: loop {
    match thread_receiver.recv() {
      Ok(ClientMessage::Timing(freqeuncy)) => {

      },
      Ok(ClientMessage::String(message)) => {
        println!("{:?}", message);
      },
      Ok(ClientMessage::Error(error)) => {

      },
      Ok(ClientMessage::Transaction(txn)) => {

      },
      Ok(ClientMessage::Done) => {

      },
      Ok(ClientMessage::Exit(this_code)) => {

      }
      Ok(ClientMessage::StepDone) => {
        /*if debug_flag{
          mech_client.send(RunLoopMessage::PrintDebug);
        }
        if out_tables.len() > 0 {
          for table_name in &out_tables {
            mech_client.send(RunLoopMessage::PrintTable(hash_str(table_name)));
          }
        }
        if repl_flag {
          break 'run_receive_loop;
        }*/
        //let output_id: u64 = hash_str("mech/output"); 
        //mech_client.send(RunLoopMessage::GetTable(output_id));
        //std::process::exit(0);
      },
      Err(x) => {
        //println!("{} {}", "[Error]".bright_red(), x);
        //io::stdout().flush().unwrap();
        //std::process::exit(1);
      }
      q => {
        println!("else: {:?}", q);
      },
    };
  }
}