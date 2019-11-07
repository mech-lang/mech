extern crate mech_program;
extern crate libloading;
use mech_program::{ProgramRunner, RunLoop, ClientMessage};

#[test]
fn program_test() {
  use libloading::Library;
  let system_mechanism: Library = Library::new("../mechanisms/system/target/debug/mech_system.dll").expect("Can't load library");
  let mut runner = ProgramRunner::new("test", 1000);
  let native_rust = unsafe {
    system_mechanism.get::<fn()>(b"foo\0").expect("Symbol not present")
  };
  native_rust();
  let outgoing = runner.program.outgoing.clone();
  runner.load_program("
  #test = 1 + 1
    ".to_string());
  let running = runner.run();
  assert_eq!(1,2);
}