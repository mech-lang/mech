use crate::*;
use std::hash::{DefaultHasher, Hash, Hasher};

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
        out = eval_fenced_code_block(&block.code, p, false)?;
        // Save the output of the last code block in the parent interpreter
        // so we can reference it later.
        let (last_code, _) = block.code.last().unwrap();
        let out_id = hash_str(&format!("{:?}", last_code));
        p.out_values.borrow_mut().insert(out_id, out.clone());
      } else {
        let mut sub_interpreters = p.sub_interpreters.borrow_mut();

        let mut new_sub_interpreter =  Interpreter::new(code_id, 10_000);
        new_sub_interpreter.set_functions(p.functions().clone());

        let mut pp = sub_interpreters
          .entry(code_id)
          .or_insert(Box::new(new_sub_interpreter))
          .as_mut();
        out = eval_fenced_code_block(&block.code, pp, true)?;
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
        let mut new_sub_interpreter = Interpreter::new(mika_interp_id, 10_000);
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
  code: &Vec<(MechCode, Option<Comment>)>,
  interpreter: &Interpreter,
  isolate_errors: bool,
) -> MResult<Value> {
  let mut out = Value::Empty;
  for (c, cmmnt) in code {
    match mech_code(c, interpreter) {
      Ok(value) => out = value,
      Err(err) => {
        if isolate_errors {
          return Ok(Value::String(Ref::new(err.full_chain_message())));
        }
        return Err(err);
      }
    }
    if let Some(cmmnt) = cmmnt {
      match comment(cmmnt, interpreter) {
        Ok(_) => {}
        Err(err) => {
          if isolate_errors {
            return Ok(Value::String(Ref::new(err.full_chain_message())));
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

#[cfg(feature = "functions")]
fn module_import_item_path(item: &ModuleImportPath) -> String {
  item.to_string()
}


#[cfg(feature = "functions")]
fn context_export_error(module: &str, item: &str) -> MechError {
  MechError::new(
    GenericError { msg: format!("Module export `{module}/{item}` is a context export; import it with `+> @name := {module}/{item}`") },
    None,
  ).with_compiler_loc()
}

#[cfg(feature = "functions")]
fn is_context_export(p: &Interpreter, module: &str, item: &str) -> bool {
  p.module_manifests
    .borrow()
    .export(module, item)
    .is_some_and(|export| export.kind == ModuleManifestExportKind::Context)
}

#[cfg(feature = "functions")]
pub fn module_import_runtime(import: &ModuleImport, p: &Interpreter) -> MResult<Value> {
  let module = import.module.to_string();
  match import.kind {
    ModuleImportKind::Module => {
      if import.alias.is_some() {
        return Err(MechError::new(
          GenericError { msg: "Module import alias is only supported for item imports".to_string() },
          None,
        ).with_compiler_loc());
      }
      load_module(&mut p.functions().borrow_mut(), &module)?;
      Ok(Value::Empty)
    }
    ModuleImportKind::Item => {
      let item = import.item.as_ref().ok_or_else(|| {
        MechError::new(MissingFunctionError { function_id: hash_str(&module) }, None).with_compiler_loc()
      })?;
      let item = module_import_item_path(item);
      match &import.alias {
        None => {
          if is_context_export(p, &module, &item) {
            return Err(context_export_error(&module, &item));
          }
          import_module_item(&mut p.functions().borrow_mut(), &module, &item)?;
        }
        Some(ModuleImportAlias::Value(alias)) => {
          if is_context_export(p, &module, &item) {
            return Err(context_export_error(&module, &item));
          }
          import_module_item_as(&mut p.functions().borrow_mut(), &module, &item, &alias.to_string())?;
        }
        Some(ModuleImportAlias::Context(alias)) => {
          p.bind_context_export(alias, &module, &item)?;
        }
      }
      Ok(Value::Empty)
    }
    ModuleImportKind::Glob => {
      if import.alias.is_some() {
        return Err(MechError::new(
          GenericError { msg: "Module import alias is only supported for item imports".to_string() },
          None,
        ).with_compiler_loc());
      }
      if p.module_manifests.borrow().manifest(&module).is_some_and(|manifest| manifest.exports.iter().any(|export| export.kind == ModuleManifestExportKind::Context)) {
        return Err(MechError::new(
          GenericError { msg: "Glob imports do not support context exports; import context exports explicitly with `+> @name := module/item`".to_string() },
          None,
        ).with_compiler_loc());
      }
      import_module_glob(&mut p.functions().borrow_mut(), &module)?;
      Ok(Value::Empty)
    }
    ModuleImportKind::Group => {
      let group_items = import.group_items.as_ref().ok_or_else(|| {
        MechError::new(MissingFunctionError { function_id: hash_str(&module) }, None).with_compiler_loc()
      })?;

      for group_item in group_items {
        let item = module_import_item_path(&group_item.item);
        if is_context_export(p, &module, &item) {
          return Err(MechError::new(
            GenericError { msg: format!("Grouped imports do not support context exports; import `{module}/{item}` with `+> @name := {module}/{item}`") },
            None,
          ).with_compiler_loc());
        }
        import_module_item(&mut p.functions().borrow_mut(), &module, &item)?;
      }
      Ok(Value::Empty)
    }
  }
}

pub fn mech_code(code: &MechCode, p: &Interpreter) -> MResult<Value> {
  let out = match &code {
    MechCode::ActivationScope(scope) => activation_scope(scope, p),
    MechCode::Expression(expr) => expression(&expr, None, p),
    MechCode::Statement(stmt) => {
      #[cfg(feature = "subscript_formula")]
      reset_current_string_access_expression_live(p);
      statement(&stmt, None, p)
    },
    #[cfg(feature = "functions")]
    MechCode::Import(import) => module_import_runtime(import, p),
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

#[derive(Debug, Clone)]
struct ActivationTriggerMustBeStableReference;
impl MechErrorKind for ActivationTriggerMustBeStableReference {
  fn name(&self) -> &str { "ActivationTriggerMustBeStableReference" }
  fn message(&self) -> String { "An activation trigger must refer to existing reactive storage.".to_string() }
}
#[derive(Debug, Clone)]
struct ActivationScopeMutableDefinitionUnsupported;
impl MechErrorKind for ActivationScopeMutableDefinitionUnsupported {
  fn name(&self) -> &str { "ActivationScopeMutableDefinitionUnsupported" }
  fn message(&self) -> String { "Mutable variable definitions are not supported inside an activation scope.".to_string() }
}
#[derive(Debug, Clone)]
struct ActivationScopeExecutionUnsupported;
impl MechErrorKind for ActivationScopeExecutionUnsupported {
  fn name(&self) -> &str { "ActivationScopeExecutionUnsupported" }
  fn message(&self) -> String { "Activation-scope execution has not yet been implemented.".to_string() }
}

/// Validate source-only restrictions without evaluating or registering body code.
fn validate_activation_body(body: &[(MechCode, Option<Comment>)]) -> MResult<()> {
  for (code, _) in body {
    match code {
      MechCode::ActivationScope(_) => return Err(MechError::new(GenericError { msg: "Nested activation scopes are not supported.".to_string() }, None).with_tokens(code.tokens())),
      MechCode::Statement(Statement::VariableDefine(def)) if def.mutable => return Err(MechError::new(ActivationScopeMutableDefinitionUnsupported, None).with_tokens(code.tokens())),
      _ => {}
    }
  }
  Ok(())
}

fn activation_scope(scope: &ActivationScope, p: &Interpreter) -> MResult<Value> {
  let var = match &scope.trigger { Expression::Var(var) => var, _ => return Err(MechError::new(ActivationTriggerMustBeStableReference, None).with_tokens(scope.trigger.tokens())) };
  let trigger = p.state.borrow().get_symbol(hash_str(&var.name.to_string())).ok_or_else(|| MechError::new(ActivationTriggerMustBeStableReference, None).with_tokens(scope.trigger.tokens()))?;
  if trigger.borrow().reactive_root_cell_ids().is_empty() { return Err(MechError::new(ActivationTriggerMustBeStableReference, None).with_tokens(scope.trigger.tokens())); }
  validate_activation_body(&scope.body)?;
  Err(MechError::new(ActivationScopeExecutionUnsupported, None).with_tokens(scope.tokens()))
}
