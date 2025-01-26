use crate::*;

// Statements
// ----------------------------------------------------------------------------

pub fn body(body: &Body, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut result = None;
  for sec in &body.sections {
    result = Some(section(&sec, plan.clone(), symbols.clone(), functions.clone())?);
  }
  Ok(result.unwrap())
}

pub fn section(section: &Section, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut result = None;
  for el in &section.elements {
    result = Some(section_element(&el, plan.clone(), symbols.clone(), functions.clone())?);
  }
  Ok(result.unwrap())
}

pub fn section_element(element: &SectionElement, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut out = Value::Empty; 
  let out = match element {
    SectionElement::MechCode(code) => {
      for c in code {
        out = mech_code(&c, plan.clone(), symbols.clone(), functions.clone())?;
      }
      out
    },
    SectionElement::Section(sctn) => Value::Empty,
    SectionElement::Comment(cmmnt) => Value::Empty,
    SectionElement::Paragraph(p) => Value::Empty,
    SectionElement::UnorderedList(ul) => Value::Empty,
    SectionElement::CodeBlock => Value::Empty,
    SectionElement::OrderedList => Value::Empty,
    SectionElement::BlockQuote => Value::Empty,
    SectionElement::ThematicBreak => Value::Empty,
    SectionElement::Image => Value::Empty,
  };
  Ok(out)
}

pub fn mech_code(code: &MechCode, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match &code {
    MechCode::Expression(expr) => expression(&expr, plan.clone(), symbols.clone(), functions.clone()),
    MechCode::Statement(stmt) => statement(&stmt, plan.clone(), symbols.clone(), functions.clone()),
    MechCode::FsmSpecification(_) => todo!(),
    MechCode::FsmImplementation(_) => todo!(),
    MechCode::FunctionDefine(fxn_def) => {
      let usr_fxn = function_define(&fxn_def, functions.clone())?;
      let mut fxns_brrw = functions.borrow_mut();
      fxns_brrw.functions.insert(usr_fxn.id, usr_fxn);
      Ok(Value::Empty)
    },
    MechCode::Comment(_) => Ok(Value::Empty),
  }
}
  