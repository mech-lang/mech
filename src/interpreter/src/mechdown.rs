use crate::*;
use std::hash::{DefaultHasher, Hash, Hasher};

// Statements
// ----------------------------------------------------------------------------

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
        match el {
          ParagraphElement::InlineMechCode(expr) => {
            let value = expression(&expr, p)?;
            let code_id = hash_str(&format!("{:?}", expr));
            p.out_values.borrow_mut().insert(code_id, value.clone());
          } 
          _ => (),
        }
      }
    },
    SectionElement::Grammar(x) => x.hash(&mut hasher),
    SectionElement::Table(x) => x.hash(&mut hasher),
    SectionElement::BlockQuote(x) => x.hash(&mut hasher),
    SectionElement::ThematicBreak => {return Ok(Value::Empty);}
    SectionElement::List(x) => x.hash(&mut hasher),
  };
  let hash = hasher.finish();
  Ok(Value::Id(hash))
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
  