use crate::*;
use std::hash::{DefaultHasher, Hash, Hasher};

// Mechdown
// ----------------------------------------------------------------------------

pub fn program(program: &Program, p: &Interpreter) -> MResult<Value> {
  body(&program.body, p)
}

pub fn body(body: &Body, p: &Interpreter) -> MResult<Value> {
  let mut result = Ok(Value::Empty);
  for sec in &body.sections {
    result = Ok(section(&sec, p)?);
  } 
  result
}

pub fn section(section: &Section, p: &Interpreter) -> MResult<Value> {
  let mut result = Ok(Value::Empty);
  for el in &section.elements {
    result = Ok(section_element(&el, p)?);
  }
  result
}

pub fn section_element(element: &SectionElement, p: &Interpreter) -> MResult<Value> {
  let mut hasher = DefaultHasher::new();
  let mut out = Value::Empty; 
  match element {
    SectionElement::Image(x) => x.hash(&mut hasher),
    SectionElement::Float(x) => x.hash(&mut hasher),
    SectionElement::Citation(x) => x.hash(&mut hasher),
    SectionElement::Equation(x) => x.hash(&mut hasher),
    SectionElement::Abstract(x) => x.hash(&mut hasher),
    SectionElement::MechCode(code) => {
      for c in code {
        out = mech_code(&c, p)?;
      }
      return Ok(out)
    },
    SectionElement::FencedMechCode((code,code_id)) => {
      if *code_id == 0 {
        for c in code {
          out = mech_code(&c, &p)?;
        }
        // Save the output of the last code block in the parent interpreter
        // so we can reference it later.
        let last_code = code.last().unwrap();
        let out_id = hash_str(&format!("{:?}", last_code));
        p.out_values.borrow_mut().insert(out_id, out.clone());
      } else {
        let mut sub_interpreters = p.sub_interpreters.borrow_mut();
        let mut pp = sub_interpreters
          .entry(*code_id)
          .or_insert(Box::new(Interpreter::new(*code_id)))
          .as_mut();
        for c in code {
          out = mech_code(&c, &pp)?;
        }
        // Save the output of the last code block in the parent interpreter
        // so we can reference it later.
        let last_code = code.last().unwrap();
        let out_id = hash_str(&format!("{:?}", last_code));
        pp.out_values.borrow_mut().insert(out_id, out.clone());
      }
      return Ok(out)
    },
    SectionElement::Subtitle(x) => x.hash(&mut hasher),
    SectionElement::CodeBlock(x) => x.hash(&mut hasher),
    SectionElement::Comment(x) => x.hash(&mut hasher),
    SectionElement::Footnote(x) => x.hash(&mut hasher),
    SectionElement::Paragraph(x) => {
      for el in x.elements.iter() {
        let (code_id,value) = match paragraph_element(&el, p) {
          Ok(val) => val,
          _ => continue,
        };
        p.out_values.borrow_mut().insert(code_id, value.clone());
      }
    },
    SectionElement::Grammar(x) => x.hash(&mut hasher),
    SectionElement::Table(x) => {
      for row in &x.rows {
        for cell in row {
          for el in &cell.elements {
            let (code_id,value) = match paragraph_element(&el, p) {
              Ok(val) => val,
              _ => continue,
            };
            p.out_values.borrow_mut().insert(code_id, value.clone());
          }
        }
      }
      x.hash(&mut hasher); 
    },
    SectionElement::BlockQuote(x) => x.hash(&mut hasher),
    SectionElement::ThematicBreak => {return Ok(Value::Empty);}
    SectionElement::List(x) => x.hash(&mut hasher),
  };
  let hash = hasher.finish();
  Ok(Value::Id(hash))
}

pub fn paragraph_element(element: &ParagraphElement, p: &Interpreter) -> MResult<(u64,Value)> {
  let result = match element {
    ParagraphElement::EvalInlineMechCode(expr) => {
      let code_id = hash_str(&format!("{:?}", expr));
      match expression(&expr, p) {
        Ok(val) => (code_id,val),
        Err(e) => (code_id,Value::Empty), // the expression failed perhaps because the value isn't defined yet.
        _ => todo!(), // What do we do in the case when it really is an error though?
                      // What we really need to do is just defer the execution of this thing to the very end
      }
    }
    _ => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::None});}
  };
  Ok(result)
}

pub fn mech_code(code: &MechCode, p: &Interpreter) -> MResult<Value> {
  match &code {
    MechCode::Expression(expr) => expression(&expr, p),
    MechCode::Statement(stmt) => statement(&stmt, p),
    MechCode::FsmSpecification(_) => todo!(),
    MechCode::FsmImplementation(_) => todo!(),
    MechCode::FunctionDefine(fxn_def) => {
      let usr_fxn = function_define(&fxn_def, p)?;
      p.insert_function(usr_fxn);
      Ok(Value::Empty)
    },
    MechCode::Comment(_) => Ok(Value::Empty),
  }
}
  