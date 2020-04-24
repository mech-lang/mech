extern crate mech_program;
extern crate mech_utilities;
extern crate mech_core;
extern crate crossbeam_channel;
extern crate mech_syntax;
use mech_program::{ProgramRunner, RunLoop, ClientMessage};
use mech_utilities::{Watcher, RunLoopMessage};
use mech_core::{Core, Value, Hasher, Index, Table};
use mech_syntax::compiler::Compiler;

use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;

fn main() {
  
  let mut runner = ProgramRunner::new("test", 1000);
  //let outgoing = runner.program.outgoing.clone();
  /*runner.load_program("
block
  #data = [10 20
           30 40]".to_string());

  let mut compiler = Compiler::new();
  let mut core = Core::new(100,100);
  compiler.compile_string(r#"
block
  #ans = #data"#.to_string());
  core.register_blocks(compiler.blocks);
  runner.load_core(core);*/


  let running = runner.run();
  running.send(RunLoopMessage::Table(Hasher::hash_str("test")));
  //running.send(RunLoopMessage::PrintRuntime);
  loop {
    loop {
      match running.receive() {
        (Ok(ClientMessage::Table(table))) => {
            println!("{:?}", table);
        },
        (Ok(ClientMessage::Done)) => {
          break;
        },
        (Ok(ClientMessage::String(message))) => {
          println!("{}", message);
        }
        message => println!("{:?}", message),
      }
    }
  }
}