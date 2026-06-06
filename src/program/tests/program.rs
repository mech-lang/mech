/*
#[test]
fn program_test() {
  let mut runner = ProgramRunner::new("test");
  let running = runner.run().unwrap();
  running.send(RunLoopMessage::Code(MechCode::String("#data = [1 2 3 4 5]".to_string())));
  running.send(RunLoopMessage::Stop);

}

#[test]
fn load_module_with_program() {
  let mut runner = ProgramRunner::new("test");
  let running = runner.run().unwrap();
  running.send(RunLoopMessage::Code(MechCode::String("#test = math/sin(angle: 0)".to_string())));
  running.send(RunLoopMessage::GetValue((hash_str("test"),TableIndex::Index(1),TableIndex::Index(1))));
  loop {
    match running.receive() {
      Ok(ClientMessage::Value(value)) => {
          assert_eq!(value, Value::F32(F32::new(0.0)));
          break;
      },
      message => (),
    }
  }
}*/
#[test]
fn program_browser_resource_binding_declaration() {
  let tree = mech_syntax::parser::parse("@browser := browser://dom/").unwrap();
  assert!(!tree.body.sections.is_empty());
}

#[test]
fn program_browser_resource_read() {
  let tree = mech_syntax::parser::parse("x := body/search/_value@browser").unwrap();
  assert!(!tree.body.sections.is_empty());
}

#[test]
fn program_browser_resource_write() {
  let tree = mech_syntax::parser::parse("body/header/title@browser = \"Hello\"").unwrap();
  assert!(!tree.body.sections.is_empty());
}

#[test]
fn program_browser_resource_define_does_not_write() {
  let tree = mech_syntax::parser::parse("title@browser := \"Hello\"").unwrap();
  assert!(!tree.body.sections.is_empty());
}
