use crate::*;
use std::hash::{DefaultHasher, Hash, Hasher};

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
  let mut hasher = DefaultHasher::new();
  let mut out = Value::Empty; 
  match element {
    SectionElement::MechCode(code) => {
      for c in code {
        out = mech_code(&c, plan.clone(), symbols.clone(), functions.clone())?;
      }
      return Ok(out)
    },
    SectionElement::Section(sctn) => {return section(sctn, plan.clone(), symbols.clone(), functions.clone());},
    SectionElement::CodeBlock(x) => x.hash(&mut hasher),
    SectionElement::Comment(x) => x.hash(&mut hasher),
    SectionElement::Paragraph(x) => x.hash(&mut hasher),
    SectionElement::UnorderedList(x) => x.hash(&mut hasher),
    SectionElement::Grammar(x) => x.hash(&mut hasher),
    SectionElement::Table(x) => x.hash(&mut hasher),
    SectionElement::BlockQuote(x) => x.hash(&mut hasher),
    SectionElement::Hyperlink(x) => x.hash(&mut hasher),
    SectionElement::OrderedList => todo!(),
    SectionElement::ThematicBreak => todo!(),
    SectionElement::Image => todo!(),
  };
  let hash = hasher.finish();
  Ok(Value::Id(hash))
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
  