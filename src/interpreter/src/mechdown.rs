use crate::*;
use std::hash::{DefaultHasher, Hash, Hasher};

const MECH_ERROR_HTML_PREFIX: &str = "__MECH_ERROR_HTML__:";

// Mechdown
// ----------------------------------------------------------------------------

#[cfg(feature = "symbol_table")]
fn update_ans_symbol(value: &Value, p: &Interpreter) {
  let resolved_value = match value {
    Value::MutableReference(reference) => reference.borrow().clone(),
    _ => value.clone(),
  };
  let ans_id = hash_str("ans");
  let symbols = p.symbols();
  let mut symbols_brrw = symbols.borrow_mut();
  symbols_brrw.insert(ans_id, resolved_value, false);
  symbols_brrw
    .dictionary
    .borrow_mut()
    .insert(ans_id, "ans".to_string());
  p.dictionary().borrow_mut().insert(ans_id, "ans".to_string());
}

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
    SectionElement::Prompt(x) => x.hash(&mut hasher),
    SectionElement::InfoBlock(x) => x.hash(&mut hasher),
    SectionElement::QuestionBlock(x) => x.hash(&mut hasher),
    SectionElement::WarningBlock(x) => x.hash(&mut hasher),
    SectionElement::ErrorBlock(x) => x.hash(&mut hasher),
    SectionElement::IdeaBlock(x) => x.hash(&mut hasher),
    SectionElement::Image(x) => x.hash(&mut hasher),
    SectionElement::Float((el, _direction)) => {
      // Floated nodes should still be interpreted just like their wrapped
      // section element (e.g. fenced Mech code, inline eval in captions, etc).
      return section_element(el, p);
    },
    SectionElement::Citation(x) => x.hash(&mut hasher),
    SectionElement::Equation(x) => x.hash(&mut hasher),
    SectionElement::Abstract(x) => x.hash(&mut hasher),
    SectionElement::Diagram(x) => x.hash(&mut hasher),
    SectionElement::MechCode(code) => {
      for line in code {
        out = mech_code(&line.code, p)?;
        match &line.terminal.comment {
          Some(cmmnt) => {
            let cmmnt_value = comment(cmmnt, p)?;
          }
          None => {}
        }
      }
      return Ok(out)
    },
    #[cfg(feature = "functions")]
    SectionElement::FencedMechCode(block) => {
      if block.config.disabled == true {
        return Ok(Value::Empty);
      }
      let code_id = block.config.namespace;
      if code_id == 0 {
        out = eval_fenced_code_block(&block.code, p, false)?;
        // Save the output of the last code block in the parent interpreter
        // so we can reference it later.
        let last_code = &block.code.last().unwrap().code;
        let out_id = hash_str(&format!("{:?}", last_code));
        p.out_values.borrow_mut().insert(out_id, out.clone());
      } else {
        let mut sub_interpreters = p.sub_interpreters.borrow_mut();

        let mut new_sub_interpreter =  Interpreter::new(code_id);
        new_sub_interpreter.set_functions(p.functions().clone());

        let mut pp = sub_interpreters
          .entry(code_id)
          .or_insert(Box::new(new_sub_interpreter))
          .as_mut();
        out = eval_fenced_code_block(&block.code, pp, true)?;
        // Save the output of the last code block in the parent interpreter
        // so we can reference it later.
        let last_code = &block.code.last().unwrap().code;
        let out_id = hash_str(&format!("{:?}", last_code));
        pp.out_values.borrow_mut().insert(out_id, out.clone());
      }
      return Ok(out)
    },
    SectionElement::Subtitle(x) => x.hash(&mut hasher),
    SectionElement::CodeBlock(x) => x.hash(&mut hasher),
    SectionElement::Comment(par) => {
      for el in par.paragraph.elements.iter() {
        let (code_id,value) = match paragraph_element(&el, p) {
          Ok(val) => val,
          _ => continue,
        };
        p.out_values.borrow_mut().insert(code_id, value.clone());
      }
      return Ok(Value::Empty);
    },
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
    SectionElement::QuoteBlock(x) => x.hash(&mut hasher),
    SectionElement::ThematicBreak => {return Ok(Value::Empty);}
    SectionElement::List(x) => x.hash(&mut hasher),
    SectionElement::SuccessBlock(x) => x.hash(&mut hasher),
    SectionElement::ErrorBlock(x) => x.hash(&mut hasher),
    SectionElement::WarningBlock(x) => x.hash(&mut hasher),
    SectionElement::InfoBlock(x) => x.hash(&mut hasher),
    SectionElement::IdeaBlock(x) => x.hash(&mut hasher),
    SectionElement::FigureTable(x) => {
      for row in &x.rows {
        for figure in row {
          for el in &figure.caption.elements {
            let (code_id, value) = match paragraph_element(el, p) {
              Ok(val) => val,
              _ => continue,
            };
            p.out_values.borrow_mut().insert(code_id, value.clone());
          }
        }
      }
      x.hash(&mut hasher);
    },
    #[cfg(feature = "mika")]
    SectionElement::Mika((m,s)) => {
      if let Some(mika_section) = s {
        let mika_interp_id = mika_interpreter_id(p.id, m, s);
        let mut sub_interpreters = p.sub_interpreters.borrow_mut();
        let mut new_sub_interpreter = Interpreter::new(mika_interp_id);
        new_sub_interpreter.set_functions(p.functions().clone());
        let pp = sub_interpreters
          .entry(mika_interp_id)
          .or_insert(Box::new(new_sub_interpreter))
          .as_mut();
        let _ = section(&mika_section.elements, pp)?;
      }
      return Ok(Value::Atom(Ref::new(MechAtom::from_name(&m.to_string()))));
    },
    x => {return Err(MechError::new(
        FeatureNotEnabledError,
        Some(format!("Feature not enabled for section element: {:?}", x)),
      ).with_compiler_loc().with_tokens(x.tokens())
    );}
  };
  let hash = hasher.finish();
  Ok(Value::Id(hash))
}

#[cfg(feature = "functions")]
fn eval_fenced_code_block(
  code: &Vec<MechCodeLine>,
  interpreter: &Interpreter,
  isolate_errors: bool,
) -> MResult<Value> {
  let mut out = Value::Empty;
  for line in code {
    match mech_code(&line.code, interpreter) {
      Ok(value) => out = value,
      Err(err) => {
        if isolate_errors {
          #[cfg(feature = "pretty_print")]
          return Ok(Value::String(Ref::new(format!(
            "{MECH_ERROR_HTML_PREFIX}{}",
            err.to_html()
          ))));
          #[cfg(not(feature = "pretty_print"))]
          return Ok(Value::String(Ref::new(format!(
            "{MECH_ERROR_HTML_PREFIX}{}",
            err.full_chain_message()
          ))));
        }
        return Err(err);
      }
    }
    if let Some(cmmnt) = &line.terminal.comment {
      match comment(cmmnt, interpreter) {
        Ok(_) => {}
        Err(err) => {
          if isolate_errors {
            #[cfg(feature = "pretty_print")]
            return Ok(Value::String(Ref::new(format!(
              "{MECH_ERROR_HTML_PREFIX}{}",
              err.to_html()
            ))));
            #[cfg(not(feature = "pretty_print"))]
            return Ok(Value::String(Ref::new(format!(
              "{MECH_ERROR_HTML_PREFIX}{}",
              err.full_chain_message()
            ))));
          }
          return Err(err);
        }
      }
    }
  }
  Ok(out)
}

fn inline_eval_id(p: &Interpreter) -> u64 {
  let next_ix = {
    let mut counter = p.inline_eval_counter.borrow_mut();
    let current = *counter;
    *counter += 1;
    current
  };
  hash_str(&format!("inline-eval:{}:{}", p.id, next_ix))
}

#[cfg(feature = "mika")]
fn mika_interpreter_id(parent_id: u64, mika: &Mika, section: &Option<MikaSection>) -> u64 {
  hash_str(&format!("mika:{}:{:?}", parent_id, (mika, section)))
}

pub fn paragraph_element(element: &ParagraphElement, p: &Interpreter) -> MResult<(u64,Value)> {
  let result = match element {
    ParagraphElement::EvalInlineMechCode(expr) => {
      let code_id = inline_eval_id(p);
      match expression(&expr, None, p) {
        Ok(val) => (code_id,val),
        Err(e) => (code_id,Value::Empty), // the expression failed perhaps because the value isn't defined yet.
        _ => todo!(), // What do we do in the case when it really is an error though?
                      // What we really need to do is just defer the execution of this thing to the very end
      }
    }
    _ => {return Err(MechError::new(
        NotExecutableError{},
        None
      ).with_compiler_loc().with_tokens(element.tokens())
    );}
  };
  Ok(result)
}

pub fn comment(cmmt: &Comment, p: &Interpreter) -> MResult<Value> {
  let par = &cmmt.paragraph;
  for el in par.elements.iter() {
    let (code_id,value) = match paragraph_element(&el, p) {
      Ok(val) => val,
      _ => continue,
    };
    p.out_values.borrow_mut().insert(code_id, value.clone());
  }
  Ok(Value::Empty)
}

pub fn mech_code(code: &MechCode, p: &Interpreter) -> MResult<Value> {
  let out = match &code {
    MechCode::Expression(expr) => expression(&expr, None, p),
    MechCode::Statement(stmt) => statement(&stmt, None, p),
    #[cfg(feature = "state_machines")]
    MechCode::FsmSpecification(fsm_spec) => {
      crate::state_machines::register_fsm_specification(fsm_spec, p)?;
      Ok(Value::Empty)
    },
    #[cfg(not(feature = "state_machines"))]
    MechCode::FsmSpecification(_) => Ok(Value::Empty),
    #[cfg(feature = "state_machines")]
    MechCode::FsmImplementation(fsm_impl) => {
      crate::state_machines::register_fsm_implementation(fsm_impl, p)?;
      Ok(Value::Empty)
    },
    #[cfg(feature = "functions")]
    MechCode::FunctionDefine(fxn_def) => {
      function_define(&fxn_def, p)?;
      Ok(Value::Empty)
    },
    MechCode::Comment(cmmt) => comment(&cmmt, p),
    x => Err(MechError::new(
        FeatureNotEnabledError,
        None
      ).with_compiler_loc().with_tokens(x.tokens())
    ),
  }?;
  #[cfg(feature = "symbol_table")]
  update_ans_symbol(&out, p);
  Ok(out)
}
