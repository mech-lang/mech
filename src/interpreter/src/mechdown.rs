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
    SectionElement::Prompt(x) => x.hash(&mut hasher),
    SectionElement::InfoBlock(x) => x.hash(&mut hasher),
    SectionElement::QuestionBlock(x) => x.hash(&mut hasher),
    SectionElement::WarningBlock(x) => x.hash(&mut hasher),
    SectionElement::ErrorBlock(x) => x.hash(&mut hasher),
    SectionElement::IdeaBlock(x) => x.hash(&mut hasher),
    SectionElement::Image(x) => x.hash(&mut hasher),
    SectionElement::Float(x) => x.hash(&mut hasher),
    SectionElement::Citation(x) => x.hash(&mut hasher),
    SectionElement::Equation(x) => x.hash(&mut hasher),
    SectionElement::Abstract(x) => x.hash(&mut hasher),
    SectionElement::Diagram(x) => x.hash(&mut hasher),
    SectionElement::MechCode(code) => {
      for (c,cmmnt) in code {
        out = mech_code(&c, p)?;
        match cmmnt {
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
        for (c,_) in &block.code {
          out = mech_code(&c, &p)?;
        }
        // Save the output of the last code block in the parent interpreter
        // so we can reference it later.
        let (last_code, _) = block.code.last().unwrap();
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
        for (c,_) in &block.code {
          out = mech_code(&c, &pp)?;
        }
        // Save the output of the last code block in the parent interpreter
        // so we can reference it later.
        let (last_code,_) = block.code.last().unwrap();
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
      if let Some(table_value) = inline_table_paragraph_to_value(x)? {
        return Ok(table_value);
      }
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
      if let Ok(table_value) = markdown_table_to_value(x) {
        return Ok(table_value);
      }
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
    #[cfg(feature = "mika")]
    SectionElement::Mika((m,s)) => {
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

pub fn paragraph_element(element: &ParagraphElement, p: &Interpreter) -> MResult<(u64,Value)> {
  let result = match element {
    ParagraphElement::EvalInlineMechCode(expr) => {
      let code_id = hash_str(&format!("{:?}", expr));
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
  match &code {
    MechCode::Expression(expr) => expression(&expr, None, p),
    MechCode::Statement(stmt) => statement(&stmt, None, p),
    //MechCode::FsmSpecification(_) => todo!(),
    //MechCode::FsmImplementation(_) => todo!(),
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
  }
}

fn markdown_table_to_value(table: &MarkdownTable) -> MResult<Value> {
  let mut headers: Vec<String> = table.header.iter().map(|header| header.to_string().trim().to_string()).collect();
  let rows: Vec<Vec<String>> = table.rows.iter().map(|row| {
    row.iter().map(|column| column.to_string().trim().to_string()).collect()
  }).collect();

  if headers.is_empty() && rows.len() == 1 && !rows[0].is_empty() {
    headers = vec![rows[0][0].clone()];
    let compact_records: Vec<_> = rows[0]
      .iter()
      .skip(1)
      .map(|value_text| MechRecord::new(vec![(headers[0].as_str(), parse_markdown_table_value(value_text))]))
      .collect();
    let compact_table = MechTable::from_records(compact_records)?;
    return Ok(Value::Table(Ref::new(compact_table)));
  }

  if headers.is_empty() {
    return Err(MechError::new(
      GenericError { msg: "Markdown table is missing a header row.".to_string() },
      None,
    ));
  }

  let mut records = Vec::with_capacity(rows.len());
  for row in &rows {
    if row.len() != headers.len() {
      return Err(MechError::new(
        GenericError { msg: "Markdown table row has a different number of columns than the header.".to_string() },
        None,
      ));
    }

    let mut fields = Vec::with_capacity(row.len());
    for (text, header) in row.iter().zip(headers.iter()) {
      let value = parse_markdown_table_value(text);
      fields.push((header.as_str(), value));
    }
    records.push(MechRecord::new(fields));
  }

  let table = MechTable::from_records(records)?;
  Ok(Value::Table(Ref::new(table)))
}

fn parse_markdown_table_value(text: &str) -> Value {
  if text.eq_ignore_ascii_case("true") || text == "✓" {
    return Value::Bool(Ref::new(true));
  }
  if text.eq_ignore_ascii_case("false") || text == "✗" {
    return Value::Bool(Ref::new(false));
  }
  if let Ok(value) = text.parse::<f64>() {
    return Value::F64(Ref::new(value));
  }
  Value::String(Ref::new(text.to_string()))
}

fn inline_table_paragraph_to_value(paragraph: &Paragraph) -> MResult<Option<Value>> {
  let text = paragraph.to_string();
  let trimmed = text.trim();
  if !(trimmed.starts_with('|') && trimmed.ends_with('|')) {
    return Ok(None);
  }

  let cells: Vec<String> = trimmed
    .split('|')
    .map(|cell| cell.trim())
    .filter(|cell| !cell.is_empty())
    .map(|cell| cell.to_string())
    .collect();

  if cells.len() < 2 {
    return Ok(None);
  }

  let header = vec![cells[0].clone()];
  let mut records = Vec::with_capacity(cells.len() - 1);
  for value_text in cells.iter().skip(1) {
    records.push(MechRecord::new(vec![(header[0].as_str(), parse_markdown_table_value(value_text))]));
  }

  let table = MechTable::from_records(records)?;
  Ok(Some(Value::Table(Ref::new(table))))
}
