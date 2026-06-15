// ---------------------------------------------------------------------------
// Program execution
// ---------------------------------------------------------------------------

// These are the main methods responsible for executing Mech programs within the runtime. They handle the orchestration of program execution, including setting up the execution context, managing module imports and dependencies, emitting events for diagnostics, and ensuring that execution adheres to the runtime's limits and policies.

// There are two main entry points for execution:

// - `run_string`: Executes a string of Mech source code directly. This is for lightweight execution of ad-hoc code snippets, scripts, documents, configuration files, etc.
// - `run_module`: Executes a module by its version ID, handling the resolution of dependencies and the construction of the import environment. This is for executing more complex, modular code that depends on other modules and is part of the larger program structure.

// Both methods have corresponding _with_context versions that accept a mutable reference to a RuntimeContext, allowing for execution within the context of an active transaction. This ensures that any changes made during execution are properly staged within the transaction that if an error occurs, the transaction can be rolled back to maintain consistency.

use super::*;

const DEFAULT_RESOURCE_SUBJECT: &str = "task://main";


#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeAddressTarget {
  Interpreter(SourceScope),
  Context(RuntimeContextBinding),
  Unknown,
}

fn address_binding_key(target: &str, name: &str) -> String {
  format!("@{target}/{name}")
}

fn resolve_runtime_address_target(
  record: &ModuleVersionRecord,
  _scope: &SourceScope,
  context_registry: &RuntimeContextRegistry,
  target: &str,
) -> RuntimeAddressTarget {
  for metadata in &record.scopes {
    if let SourceScope::Interpreter(interpreter) = &metadata.scope {
      if interpreter.namespace_str == target {
        return RuntimeAddressTarget::Interpreter(metadata.scope.clone());
      }
    }
  }

  if let Some(binding) = context_registry.get(target) {
    return RuntimeAddressTarget::Context(binding.clone());
  }

  RuntimeAddressTarget::Unknown
}

fn context_registry_for_scope(
  record: &ModuleVersionRecord,
  scope: &SourceScope,
) -> MResult<RuntimeContextRegistry> {
  let declarations = record
    .scopes
    .iter()
    .find(|metadata| &metadata.scope == scope)
    .map(|metadata| metadata.contexts.as_slice())
    .unwrap_or(&[]);
  RuntimeContextRegistry::from_declarations(scope.clone(), declarations)
}

fn runtime_context_base_uri(binding: &RuntimeContextBinding) -> String {
  match &binding.base {
    RuntimeContextBase::ResourceUri(uri) => uri.clone(),
  }
}

fn runtime_context_allows_read(
  binding: &RuntimeContextBinding,
  path: &str,
) -> bool {
  binding.capabilities.iter().any(|capability| {
    if capability.operation != "read" {
      return false;
    }

    match &capability.scope {
      RuntimeContextCapabilityScope::Wildcard => true,
      RuntimeContextCapabilityScope::Path(exact) => {
        if exact == path {
          return true;
        }
        if let Some(prefix) = exact.strip_suffix("/*") {
          let required_prefix = format!("{}/", prefix);
          return path.starts_with(&required_prefix);
        }
        false
      }
    }
  })
}

#[allow(dead_code)]
fn runtime_context_allows_write(
  binding: &RuntimeContextBinding,
  path: &str,
) -> bool {
  binding.capabilities.iter().any(|capability| {
    if capability.operation != "write" {
      return false;
    }

    match &capability.scope {
      RuntimeContextCapabilityScope::Wildcard => true,
      RuntimeContextCapabilityScope::Path(exact) => {
        if exact == path {
          return true;
        }
        if let Some(prefix) = exact.strip_suffix("/*") {
          let required_prefix = format!("{}/", prefix);
          return path.starts_with(&required_prefix);
        }
        false
      }
    }
  })
}



fn program_codes(
  tree: &mech_core::Program,
) -> impl Iterator<Item = (&mech_core::MechCode, &Option<mech_core::Comment>)> {
  tree.body.sections.iter().flat_map(|section| {
    section.elements.iter().flat_map(|element| match element {
      mech_core::SectionElement::MechCode(codes) => codes
        .iter()
        .map(|(code, comment)| (code, comment))
        .collect::<Vec<_>>(),
      mech_core::SectionElement::FencedMechCode(fenced) => fenced
        .code
        .iter()
        .map(|(code, comment)| (code, comment))
        .collect::<Vec<_>>(),
      _ => Vec::new(),
    })
  })
}

fn single_code_program(
  code: mech_core::MechCode,
  comment: Option<mech_core::Comment>,
) -> mech_core::Program {
  mech_core::Program {
    title: None,
    body: mech_core::Body {
      sections: vec![mech_core::Section {
        subtitle: None,
        elements: vec![mech_core::SectionElement::MechCode(vec![(code, comment)])],
      }],
    },
  }
}

fn bind_runtime_value_on_program(
  program: &mut MechProgram,
  var: &mech_core::Var,
  value: Value,
  mutable: bool,
) -> MResult<()> {
  if var.context.is_some() {
    return Err(MechError::new(
      mech_core::GenericError { msg: "AddressedAssignmentUnsupported: addressed assignment is not supported yet".to_string() },
      None,
    ).with_compiler_loc().with_tokens(var.tokens()));
  }
  let (id, name) = (var.name.hash(), var.name.to_string());
  let symbols = program.interpreter_mut().symbols();
  let mut symbols = symbols.borrow_mut();
  symbols.insert(id, value, mutable);
  symbols.dictionary.borrow_mut().insert(id, name);
  Ok(())
}

fn resolve_runtime_value(value: Value) -> Value {
  match value {
    Value::MutableReference(value) => value.borrow().clone(),
    other => other,
  }
}


fn string_value_expression(value: String) -> MResult<mech_core::Expression> {
  Ok(mech_core::Expression::Literal(mech_core::Literal::String(mech_core::MechString {
    text: mech_core::Token::new(
      mech_core::TokenKind::String,
      mech_core::SourceRange::default(),
      value.chars().collect(),
    ),
  })))
}

impl MechRuntime {

  fn bind_browser_resource_roots_from_program(
    &mut self,
    tree: &mech_core::Program,
  ) -> MResult<()> {
    for (code, _) in program_codes(tree) {
      let mech_core::MechCode::Statement(mech_core::Statement::ContextDeclaration(ctx)) = code else {
        continue;
      };
      let mech_core::ContextBase::ResourceUri(uri) = &ctx.base else {
        continue;
      };
      let uri = uri.to_string();
      if uri.starts_with("browser://dom/") {
        self.bind_resource_root(ctx.name.to_string(), uri)?;
      }
    }
    Ok(())
  }

  fn run_tree_on_program(
    &mut self,
    _context: &mut RuntimeContext,
    program: &mut MechProgram,
    tree: &mech_core::Program,
  ) -> MResult<Value> {
    if !self.program_has_browser_resource_forms(tree) {
      return program.run_tree(tree);
    }

    let mut result = Value::Empty;
    for (code, comment) in program_codes(tree) {
      match code {
        mech_core::MechCode::Statement(mech_core::Statement::ContextDeclaration(ctx))
          if self.is_browser_dom_context_declaration(ctx) =>
        {
          result = Value::Empty;
        }
        mech_core::MechCode::Statement(mech_core::Statement::VariableDefine(var_def))
          if self.variable_define_targets_browser_binding(var_def) =>
        {
          let expression = self.lower_browser_resource_reads_in_expression(&var_def.expression)?;
          let value = self.evaluate_expression_on_program(program, &expression)?;
          bind_runtime_value_on_program(program, &var_def.var, resolve_runtime_value(value.clone()), var_def.mutable)?;
          result = value;
        }
        mech_core::MechCode::Statement(mech_core::Statement::VariableDefine(var_def)) => {
          let expression = self.lower_browser_resource_reads_in_expression(&var_def.expression)?;
          let mut var_def = var_def.clone();
          var_def.expression = expression;
          let single = single_code_program(
            mech_core::MechCode::Statement(mech_core::Statement::VariableDefine(var_def)),
            comment.clone(),
          );
          result = program.run_tree(&single)?;
        }
        mech_core::MechCode::Statement(mech_core::Statement::VariableAssign(assign))
          if self.assignment_targets_browser_resource(assign) =>
        {
          let expression = self.lower_browser_resource_reads_in_expression(&assign.expression)?;
          let value = self.evaluate_expression_on_program(program, &expression)?;
          let binding = assign.target.context.as_ref().expect("checked browser binding").to_string();
          self.write_bound_resource(
            &binding,
            &assign.target.name.to_string(),
            &resolve_runtime_value(value.clone()),
          )?;
          result = value;
        }
        mech_core::MechCode::Statement(mech_core::Statement::VariableAssign(assign)) => {
          let mut assign = assign.clone();
          assign.expression = self.lower_browser_resource_reads_in_expression(&assign.expression)?;
          let single = single_code_program(
            mech_core::MechCode::Statement(mech_core::Statement::VariableAssign(assign)),
            comment.clone(),
          );
          result = program.run_tree(&single)?;
        }
        mech_core::MechCode::Expression(expression) => {
          let expression = self.lower_browser_resource_reads_in_expression(expression)?;
          let single = single_code_program(mech_core::MechCode::Expression(expression), comment.clone());
          result = program.run_tree(&single)?;
        }
        _ => {
          let single = single_code_program(code.clone(), comment.clone());
          result = program.run_tree(&single)?;
        }
      }
    }
    Ok(result)
  }

  fn program_has_browser_resource_forms(&self, tree: &mech_core::Program) -> bool {
    program_codes(tree).any(|(code, _)| match code {
      mech_core::MechCode::Statement(mech_core::Statement::ContextDeclaration(ctx)) => {
        self.is_browser_dom_context_declaration(ctx)
      }
      mech_core::MechCode::Statement(mech_core::Statement::VariableDefine(var_def)) => {
        self.variable_define_targets_browser_binding(var_def)
          || self.expression_reads_browser_resource(&var_def.expression)
      }
      mech_core::MechCode::Statement(mech_core::Statement::VariableAssign(assign)) => {
        self.assignment_targets_browser_resource(assign)
          || self.expression_reads_browser_resource(&assign.expression)
      }
      mech_core::MechCode::Expression(expression) => {
        self.expression_reads_browser_resource(expression)
      }
      _ => false,
    })
  }

  fn is_browser_dom_context_declaration(&self, ctx: &mech_core::ContextDeclaration) -> bool {
    match &ctx.base {
      mech_core::ContextBase::ResourceUri(uri) => uri.to_string().starts_with("browser://dom/"),
      _ => false,
    }
  }

  fn is_browser_dom_resource_binding(&self, binding: &str) -> bool {
    self
      .resource_bindings
      .get(binding)
      .is_some_and(|binding| binding.base_uri == BROWSER_DOM_PROVIDER_URI)
  }

  fn variable_define_targets_browser_binding(&self, var_def: &mech_core::VariableDefine) -> bool {
    var_def
      .var
      .context
      .as_ref()
      .is_some_and(|context| self.is_browser_dom_resource_binding(&context.to_string()))
  }

  fn assignment_targets_browser_resource(&self, assign: &mech_core::VariableAssign) -> bool {
    assign
      .target
      .context
      .as_ref()
      .is_some_and(|context| self.is_browser_dom_resource_binding(&context.to_string()))
  }

  fn expression_reads_browser_resource(&self, expression: &mech_core::Expression) -> bool {
    match expression {
      mech_core::Expression::Var(var) => var
        .context
        .as_ref()
        .is_some_and(|context| self.is_browser_dom_resource_binding(&context.to_string())),
      mech_core::Expression::Formula(factor) => self.factor_reads_browser_resource(factor),
      mech_core::Expression::FunctionCall(call) => call
        .args
        .iter()
        .any(|(_, expression)| self.expression_reads_browser_resource(expression)),
      mech_core::Expression::FsmPipe(pipe) => pipe
        .start
        .args
        .as_ref()
        .is_some_and(|args| args.iter().any(|(_, expression)| self.expression_reads_browser_resource(expression))),
      mech_core::Expression::Literal(_) => false,
      mech_core::Expression::Match(match_expression) => {
        self.expression_reads_browser_resource(&match_expression.source)
          || match_expression.arms.iter().any(|arm| {
            arm
              .guard
              .as_ref()
              .is_some_and(|guard| self.expression_reads_browser_resource(guard))
              || self.expression_reads_browser_resource(&arm.expression)
          })
      }
      mech_core::Expression::Range(range) => {
        self.factor_reads_browser_resource(&range.start)
          || range
            .increment
            .as_ref()
            .is_some_and(|(_, increment)| self.factor_reads_browser_resource(increment))
          || self.factor_reads_browser_resource(&range.terminal)
      }
      mech_core::Expression::Slice(slice) => self.slice_reads_browser_resource(slice),
      mech_core::Expression::Structure(structure) => self.structure_reads_browser_resource(structure),
      mech_core::Expression::SetComprehension(comprehension) => {
        self.expression_reads_browser_resource(&comprehension.expression)
          || comprehension
            .qualifiers
            .iter()
            .any(|qualifier| self.comprehension_qualifier_reads_browser_resource(qualifier))
      }
      mech_core::Expression::MatrixComprehension(comprehension) => {
        self.expression_reads_browser_resource(&comprehension.expression)
          || comprehension
            .qualifiers
            .iter()
            .any(|qualifier| self.comprehension_qualifier_reads_browser_resource(qualifier))
      }
    }
  }

  fn factor_reads_browser_resource(&self, factor: &mech_core::Factor) -> bool {
    match factor {
      mech_core::Factor::Expression(expression) => self.expression_reads_browser_resource(expression),
      mech_core::Factor::Negate(factor)
      | mech_core::Factor::Not(factor)
      | mech_core::Factor::Parenthetical(factor)
      | mech_core::Factor::Transpose(factor) => self.factor_reads_browser_resource(factor),
      mech_core::Factor::Term(term) => {
        self.factor_reads_browser_resource(&term.lhs)
          || term
            .rhs
            .iter()
            .any(|(_, factor)| self.factor_reads_browser_resource(factor))
      }
    }
  }

  fn slice_reads_browser_resource(&self, slice: &mech_core::Slice) -> bool {
    slice
      .subscript
      .iter()
      .any(|subscript| self.subscript_reads_browser_resource(subscript))
  }

  fn subscript_reads_browser_resource(&self, subscript: &mech_core::Subscript) -> bool {
    match subscript {
      mech_core::Subscript::Brace(subscripts) | mech_core::Subscript::Bracket(subscripts) => {
        subscripts
          .iter()
          .any(|subscript| self.subscript_reads_browser_resource(subscript))
      }
      mech_core::Subscript::Formula(factor) => self.factor_reads_browser_resource(factor),
      mech_core::Subscript::Range(range) => {
        self.factor_reads_browser_resource(&range.start)
          || range
            .increment
            .as_ref()
            .is_some_and(|(_, increment)| self.factor_reads_browser_resource(increment))
          || self.factor_reads_browser_resource(&range.terminal)
      }
      _ => false,
    }
  }

  fn structure_reads_browser_resource(&self, structure: &mech_core::Structure) -> bool {
    match structure {
      mech_core::Structure::Empty => false,
      mech_core::Structure::Map(map) => map.elements.iter().any(|mapping| {
        self.expression_reads_browser_resource(&mapping.key)
          || self.expression_reads_browser_resource(&mapping.value)
      }),
      mech_core::Structure::Matrix(matrix) => matrix.rows.iter().any(|row| {
        row
          .columns
          .iter()
          .any(|column| self.expression_reads_browser_resource(&column.element))
      }),
      mech_core::Structure::Record(record) => record
        .bindings
        .iter()
        .any(|binding| self.expression_reads_browser_resource(&binding.value)),
      mech_core::Structure::Set(set) => set
        .elements
        .iter()
        .any(|expression| self.expression_reads_browser_resource(expression)),
      mech_core::Structure::Table(table) => table.rows.iter().any(|row| {
        row
          .columns
          .iter()
          .any(|column| self.expression_reads_browser_resource(&column.element))
      }),
      mech_core::Structure::Tuple(tuple) => tuple
        .elements
        .iter()
        .any(|expression| self.expression_reads_browser_resource(expression)),
      mech_core::Structure::TupleStruct(tuple_struct) => {
        self.expression_reads_browser_resource(&tuple_struct.value)
      }
    }
  }

  fn comprehension_qualifier_reads_browser_resource(
    &self,
    qualifier: &mech_core::ComprehensionQualifier,
  ) -> bool {
    match qualifier {
      mech_core::ComprehensionQualifier::Generator((_, expression)) => {
        self.expression_reads_browser_resource(expression)
      }
      mech_core::ComprehensionQualifier::Filter(expression) => {
        self.expression_reads_browser_resource(expression)
      }
      mech_core::ComprehensionQualifier::Let(var_def) => {
        self.expression_reads_browser_resource(&var_def.expression)
      }
    }
  }

  fn lower_browser_resource_reads_in_expression(
    &mut self,
    expression: &mech_core::Expression,
  ) -> MResult<mech_core::Expression> {
    match expression {
      mech_core::Expression::Var(var) => {
        let Some(context) = &var.context else {
          return Ok(expression.clone());
        };
        let binding = context.to_string();
        if !self.is_browser_dom_resource_binding(&binding) {
          return Ok(expression.clone());
        }
        let value = resolve_runtime_value(
          self.read_bound_resource(&binding, &var.name.to_string())?,
        );
        let Value::String(value) = value else {
          return Err(browser_runtime_resource_error(
            var.name.to_string(),
            "browser DOM resource reads must return string values in this PR",
          ));
        };
        string_value_expression(value.borrow().clone())
      }
      mech_core::Expression::Formula(factor) => {
        Ok(mech_core::Expression::Formula(self.lower_browser_resource_reads_in_factor(factor)?))
      }
      mech_core::Expression::FunctionCall(call) => {
        let args = call
          .args
          .iter()
          .map(|(name, expression)| {
            Ok((
              name.clone(),
              self.lower_browser_resource_reads_in_expression(expression)?,
            ))
          })
          .collect::<MResult<Vec<_>>>()?;
        Ok(mech_core::Expression::FunctionCall(mech_core::FunctionCall {
          name: call.name.clone(),
          args,
        }))
      }
      mech_core::Expression::FsmPipe(pipe) => {
        let mut pipe = pipe.clone();
        if let Some(args) = &pipe.start.args {
          pipe.start.args = Some(
            args
              .iter()
              .map(|(name, expression)| {
                Ok((
                  name.clone(),
                  self.lower_browser_resource_reads_in_expression(expression)?,
                ))
              })
              .collect::<MResult<Vec<_>>>()?,
          );
        }
        Ok(mech_core::Expression::FsmPipe(pipe))
      }
      mech_core::Expression::Literal(_) => Ok(expression.clone()),
      mech_core::Expression::Match(match_expression) => {
        let mut match_expression = match_expression.as_ref().clone();
        match_expression.source = self.lower_browser_resource_reads_in_expression(&match_expression.source)?;
        match_expression.arms = match_expression
          .arms
          .iter()
          .map(|arm| {
            Ok(mech_core::MatchArm {
              pattern: arm.pattern.clone(),
              guard: arm
                .guard
                .as_ref()
                .map(|guard| self.lower_browser_resource_reads_in_expression(guard))
                .transpose()?,
              expression: self.lower_browser_resource_reads_in_expression(&arm.expression)?,
            })
          })
          .collect::<MResult<Vec<_>>>()?;
        Ok(mech_core::Expression::Match(Box::new(match_expression)))
      }
      mech_core::Expression::Range(range) => {
        let mut range = range.as_ref().clone();
        range.start = self.lower_browser_resource_reads_in_factor(&range.start)?;
        range.increment = match &range.increment {
          Some((operator, increment)) => Some((
            operator.clone(),
            self.lower_browser_resource_reads_in_factor(increment)?,
          )),
          None => None,
        };
        range.terminal = self.lower_browser_resource_reads_in_factor(&range.terminal)?;
        Ok(mech_core::Expression::Range(Box::new(range)))
      }
      mech_core::Expression::Slice(slice) => {
        Ok(mech_core::Expression::Slice(self.lower_browser_resource_reads_in_slice(slice)?))
      }
      mech_core::Expression::Structure(structure) => {
        Ok(mech_core::Expression::Structure(self.lower_browser_resource_reads_in_structure(structure)?))
      }
      mech_core::Expression::SetComprehension(comprehension) => {
        let mut comprehension = comprehension.as_ref().clone();
        comprehension.expression = self.lower_browser_resource_reads_in_expression(&comprehension.expression)?;
        comprehension.qualifiers = comprehension
          .qualifiers
          .iter()
          .map(|qualifier| self.lower_browser_resource_reads_in_comprehension_qualifier(qualifier))
          .collect::<MResult<Vec<_>>>()?;
        Ok(mech_core::Expression::SetComprehension(Box::new(comprehension)))
      }
      mech_core::Expression::MatrixComprehension(comprehension) => {
        let mut comprehension = comprehension.as_ref().clone();
        comprehension.expression = self.lower_browser_resource_reads_in_expression(&comprehension.expression)?;
        comprehension.qualifiers = comprehension
          .qualifiers
          .iter()
          .map(|qualifier| self.lower_browser_resource_reads_in_comprehension_qualifier(qualifier))
          .collect::<MResult<Vec<_>>>()?;
        Ok(mech_core::Expression::MatrixComprehension(Box::new(comprehension)))
      }
    }
  }

  fn lower_browser_resource_reads_in_factor(
    &mut self,
    factor: &mech_core::Factor,
  ) -> MResult<mech_core::Factor> {
    match factor {
      mech_core::Factor::Expression(expression) => Ok(mech_core::Factor::Expression(Box::new(
        self.lower_browser_resource_reads_in_expression(expression)?,
      ))),
      mech_core::Factor::Negate(factor) => Ok(mech_core::Factor::Negate(Box::new(
        self.lower_browser_resource_reads_in_factor(factor)?,
      ))),
      mech_core::Factor::Not(factor) => Ok(mech_core::Factor::Not(Box::new(
        self.lower_browser_resource_reads_in_factor(factor)?,
      ))),
      mech_core::Factor::Parenthetical(factor) => Ok(mech_core::Factor::Parenthetical(Box::new(
        self.lower_browser_resource_reads_in_factor(factor)?,
      ))),
      mech_core::Factor::Term(term) => {
        let rhs = term
          .rhs
          .iter()
          .map(|(operator, factor)| {
            Ok((operator.clone(), self.lower_browser_resource_reads_in_factor(factor)?))
          })
          .collect::<MResult<Vec<_>>>()?;
        Ok(mech_core::Factor::Term(Box::new(mech_core::Term {
          lhs: self.lower_browser_resource_reads_in_factor(&term.lhs)?,
          rhs,
        })))
      }
      mech_core::Factor::Transpose(factor) => Ok(mech_core::Factor::Transpose(Box::new(
        self.lower_browser_resource_reads_in_factor(factor)?,
      ))),
    }
  }

  fn lower_browser_resource_reads_in_slice(
    &mut self,
    slice: &mech_core::Slice,
  ) -> MResult<mech_core::Slice> {
    Ok(mech_core::Slice {
      name: slice.name.clone(),
      context: slice.context.clone(),
      subscript: slice
        .subscript
        .iter()
        .map(|subscript| self.lower_browser_resource_reads_in_subscript(subscript))
        .collect::<MResult<Vec<_>>>()?,
    })
  }

  fn lower_browser_resource_reads_in_subscript(
    &mut self,
    subscript: &mech_core::Subscript,
  ) -> MResult<mech_core::Subscript> {
    match subscript {
      mech_core::Subscript::Brace(subscripts) => Ok(mech_core::Subscript::Brace(
        subscripts
          .iter()
          .map(|subscript| self.lower_browser_resource_reads_in_subscript(subscript))
          .collect::<MResult<Vec<_>>>()?,
      )),
      mech_core::Subscript::Bracket(subscripts) => Ok(mech_core::Subscript::Bracket(
        subscripts
          .iter()
          .map(|subscript| self.lower_browser_resource_reads_in_subscript(subscript))
          .collect::<MResult<Vec<_>>>()?,
      )),
      mech_core::Subscript::Formula(factor) => Ok(mech_core::Subscript::Formula(
        self.lower_browser_resource_reads_in_factor(factor)?,
      )),
      mech_core::Subscript::Range(range) => {
        let mut range = range.clone();
        range.start = self.lower_browser_resource_reads_in_factor(&range.start)?;
        range.increment = match &range.increment {
          Some((operator, increment)) => Some((
            operator.clone(),
            self.lower_browser_resource_reads_in_factor(increment)?,
          )),
          None => None,
        };
        range.terminal = self.lower_browser_resource_reads_in_factor(&range.terminal)?;
        Ok(mech_core::Subscript::Range(range))
      }
      _ => Ok(subscript.clone()),
    }
  }

  fn lower_browser_resource_reads_in_structure(
    &mut self,
    structure: &mech_core::Structure,
  ) -> MResult<mech_core::Structure> {
    match structure {
      mech_core::Structure::Empty => Ok(mech_core::Structure::Empty),
      mech_core::Structure::Map(map) => Ok(mech_core::Structure::Map(mech_core::Map {
        elements: map
          .elements
          .iter()
          .map(|mapping| {
            Ok(mech_core::Mapping {
              key: self.lower_browser_resource_reads_in_expression(&mapping.key)?,
              value: self.lower_browser_resource_reads_in_expression(&mapping.value)?,
            })
          })
          .collect::<MResult<Vec<_>>>()?,
      })),
      mech_core::Structure::Matrix(matrix) => Ok(mech_core::Structure::Matrix(mech_core::nodes::Matrix {
        rows: matrix
          .rows
          .iter()
          .map(|row| {
            Ok(mech_core::MatrixRow {
              columns: row
                .columns
                .iter()
                .map(|column| {
                  Ok(mech_core::MatrixColumn {
                    element: self.lower_browser_resource_reads_in_expression(&column.element)?,
                  })
                })
                .collect::<MResult<Vec<_>>>()?,
            })
          })
          .collect::<MResult<Vec<_>>>()?,
      })),
      mech_core::Structure::Record(record) => Ok(mech_core::Structure::Record(mech_core::Record {
        bindings: record
          .bindings
          .iter()
          .map(|binding| {
            Ok(mech_core::Binding {
              name: binding.name.clone(),
              kind: binding.kind.clone(),
              value: self.lower_browser_resource_reads_in_expression(&binding.value)?,
            })
          })
          .collect::<MResult<Vec<_>>>()?,
      })),
      mech_core::Structure::Set(set) => Ok(mech_core::Structure::Set(mech_core::Set {
        elements: set
          .elements
          .iter()
          .map(|expression| self.lower_browser_resource_reads_in_expression(expression))
          .collect::<MResult<Vec<_>>>()?,
      })),
      mech_core::Structure::Table(table) => Ok(mech_core::Structure::Table(mech_core::Table {
        header: table.header.clone(),
        rows: table
          .rows
          .iter()
          .map(|row| {
            Ok(mech_core::TableRow {
              columns: row
                .columns
                .iter()
                .map(|column| {
                  Ok(mech_core::TableColumn {
                    element: self.lower_browser_resource_reads_in_expression(&column.element)?,
                  })
                })
                .collect::<MResult<Vec<_>>>()?,
            })
          })
          .collect::<MResult<Vec<_>>>()?,
      })),
      mech_core::Structure::Tuple(tuple) => Ok(mech_core::Structure::Tuple(mech_core::Tuple {
        elements: tuple
          .elements
          .iter()
          .map(|expression| self.lower_browser_resource_reads_in_expression(expression))
          .collect::<MResult<Vec<_>>>()?,
      })),
      mech_core::Structure::TupleStruct(tuple_struct) => {
        Ok(mech_core::Structure::TupleStruct(mech_core::TupleStruct {
          name: tuple_struct.name.clone(),
          value: Box::new(self.lower_browser_resource_reads_in_expression(&tuple_struct.value)?),
        }))
      }
    }
  }

  fn lower_browser_resource_reads_in_comprehension_qualifier(
    &mut self,
    qualifier: &mech_core::ComprehensionQualifier,
  ) -> MResult<mech_core::ComprehensionQualifier> {
    match qualifier {
      mech_core::ComprehensionQualifier::Generator((pattern, expression)) => {
        Ok(mech_core::ComprehensionQualifier::Generator((
          pattern.clone(),
          self.lower_browser_resource_reads_in_expression(expression)?,
        )))
      }
      mech_core::ComprehensionQualifier::Filter(expression) => {
        Ok(mech_core::ComprehensionQualifier::Filter(
          self.lower_browser_resource_reads_in_expression(expression)?,
        ))
      }
      mech_core::ComprehensionQualifier::Let(var_def) => {
        let mut var_def = var_def.clone();
        var_def.expression = self.lower_browser_resource_reads_in_expression(&var_def.expression)?;
        Ok(mech_core::ComprehensionQualifier::Let(var_def))
      }
    }
  }

  fn evaluate_expression_on_program(
    &mut self,
    program: &mut MechProgram,
    expression: &mech_core::Expression,
  ) -> MResult<Value> {
    let single = single_code_program(mech_core::MechCode::Expression(expression.clone()), None);
    program.run_tree(&single).map(resolve_runtime_value)
  }

  pub fn run_string(&mut self, source: &str) -> MResult<Value> {
    let mut context = self.runtime_context()?;
    self.run_string_with_context(&mut context, source)
  }
  pub fn run_string_with_context(
    &mut self,
    context: &mut RuntimeContext,
    source: &str,
  ) -> MResult<Value> {
    context.validate()?;
    context.charge_step()?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ProgramStarted {
        task_id: context.task,
      },
    )?;

    let tree = mech_syntax::parser::parse(source.trim())?;
    self.bind_browser_resource_roots_from_program(&tree)?;

    let program_config = self.program.config.clone();
    let mut program = std::mem::replace(
      &mut self.program,
      MechProgram::new(program_config),
    );

    self.register_runtime_program_host_functions(
      context,
      &mut program,
    )?;

    let runtime_ptr: *mut MechRuntime = self;
    let context_ptr: *mut RuntimeContext = context;

    let previous_target = ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      slot.replace(Some(RuntimeProgramHostTarget {
        runtime: runtime_ptr,
        context: context_ptr,
      }))
    });

    let result = self.run_tree_on_program(context, &mut program, &tree);

    ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      slot.replace(previous_target);
    });

    self.program = program;

    match &result {
      Ok(_) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramCompleted {
            task_id: context.task,
          },
        )?;
      }
      Err(error) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramFailed {
            task_id: context.task,
            message: format!("{:?}", error),
          },
        )?;
      }
    }

    result
  }


  pub fn run_tree(&mut self, tree: &mech_core::Program) -> MResult<Value> {
    let mut context = self.runtime_context()?;
    self.run_tree_with_context(&mut context, tree)
  }

  pub fn run_tree_with_context(
    &mut self,
    context: &mut RuntimeContext,
    tree: &mech_core::Program,
  ) -> MResult<Value> {
    context.validate()?;
    context.charge_step()?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ProgramStarted {
        task_id: context.task,
      },
    )?;

    self.bind_browser_resource_roots_from_program(tree)?;

    let program_config = self.program.config.clone();
    let mut program = std::mem::replace(
      &mut self.program,
      MechProgram::new(program_config),
    );

    self.register_runtime_program_host_functions(
      context,
      &mut program,
    )?;

    let runtime_ptr: *mut MechRuntime = self;
    let context_ptr: *mut RuntimeContext = context;

    let previous_target = ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      slot.replace(Some(RuntimeProgramHostTarget {
        runtime: runtime_ptr,
        context: context_ptr,
      }))
    });

    let result = self.run_tree_on_program(context, &mut program, tree);

    ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      slot.replace(previous_target);
    });

    self.program = program;

    match &result {
      Ok(_) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramCompleted {
            task_id: context.task,
          },
        )?;
      }
      Err(error) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramFailed {
            task_id: context.task,
            message: format!("{:?}", error),
          },
        )?;
      }
    }

    result
  }

  pub fn out_string(&self) -> String {
    self.program.out_string()
  }

  pub fn has_interpreter(&self, interpreter_id: u64) -> bool {
    self.program.has_interpreter(interpreter_id)
  }

  pub fn output_value_for_interpreter(
    &self,
    interpreter_id: u64,
    output_id: u64,
  ) -> Option<Value> {
    self.program.output_value_for_interpreter(interpreter_id, output_id)
  }

  pub fn symbol_name_for_interpreter_output(
    &self,
    interpreter_id: u64,
    output_id: u64,
  ) -> Option<String> {
    self.program.symbol_name_for_interpreter_output(interpreter_id, output_id)
  }

  pub fn symbol_values_for_interpreter(
    &self,
    interpreter_id: u64,
    names: &[String],
  ) -> Option<Vec<(String, Value)>> {
    self.program.symbol_values_for_interpreter(interpreter_id, names)
  }

  pub fn bind_ans_for_interpreter(
    &mut self,
    interpreter_id: u64,
    value: &Value,
  ) -> MResult<()> {
    if self.program.bind_ans_for_interpreter(interpreter_id, value) {
      return Ok(());
    }

    Err(MechError::new(
      RuntimeInvalidOperationError {
        operation: "bind_ans_for_interpreter",
        reason: format!("interpreter id {} not found", interpreter_id),
      },
      None,
    ))
  }

  #[cfg(feature = "functions")]
  pub fn step(&mut self, count: u64) -> MResult<()> {
    self.program.step(count)
  }

  pub fn run_module(&mut self, version: ModuleVersionId) -> MResult<Value> {
    let mut context = self.runtime_context()?
      .with_module_version(version);

    self.run_module_with_context(&mut context, version)
  }

  pub fn run_module_scope(
    &mut self,
    version: ModuleVersionId,
    scope: SourceScope,
  ) -> MResult<Value> {
    let mut context = self.runtime_context()?
      .with_module_version(version);

    self.run_module_scope_with_context(&mut context, version, scope)
  }

  pub fn run_module_with_context(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
  ) -> MResult<Value> {
    self.run_module_scope_with_context(context, version, SourceScope::Program)
  }

  pub fn run_module_scope_with_context(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
    scope: SourceScope,
  ) -> MResult<Value> {
    let mut seen = HashSet::new();
    let mut module_instances = HashMap::new();

    let instance = self.execute_module_isolated_for_scope(
      context,
      version,
      &scope,
      &mut seen,
      &mut module_instances,
    )?;

    Ok(instance.result)
  }

  fn execute_module_isolated_for_scope(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
    scope: &SourceScope,
    seen: &mut HashSet<(ModuleVersionId, SourceScope)>,
    module_instances: &mut HashMap<(ModuleVersionId, SourceScope), ModuleInstance>,
  ) -> MResult<ModuleInstance> {
    context.validate()?;
    context.charge_step()?;

    let instance_key = (version, scope.clone());

    if let Some(instance) = module_instances.get(&instance_key).cloned() {
      return Ok(instance);
    }

    if seen.contains(&instance_key) {
      return Ok(ModuleInstance { version, exports: HashMap::new(), result: Value::Empty });
    }
    seen.insert(instance_key.clone());

    let Some(record) = self.store.get_module_version(version)? else {
      return Err(MechError::new(RuntimeRecordNotFoundError { record_type: "module_version", id: version.to_string() }, None));
    };
    validate_module_import_edges(&record)?;
    let Some(source) = record.source.clone() else {
      return Err(MechError::new(RuntimeInvalidOperationError { operation: "run_module", reason: "module version has no source".to_string() }, None));
    };

    let context_registry = context_registry_for_scope(&record, scope)?;

    for edge in &record.import_edges {
      if &edge.scope != scope {
        continue;
      }

      self.execute_module_isolated_for_scope(
        context,
        edge.dependency,
        &SourceScope::Program,
        seen,
        module_instances,
      )?;
    }

    let mut import_environment = self.build_import_environment_for_scope(
      context,
      &record,
      scope,
      module_instances,
    )?;

    let address_environment = self.build_address_environment_for_scope(
      context,
      version,
      &record,
      scope,
      &context_registry,
      seen,
      module_instances,
    )?;
    import_environment.extend(address_environment);
    let mut module_program = MechProgram::new(MechProgramConfig {
      name: self.config.name.clone(),
      environment: MechProgramEnvironment {
        trace_enabled: self.config.diagnostics.trace_enabled,
        debug_enabled: self.config.diagnostics.debug_enabled,
        profile_enabled: self.config.diagnostics.profile_enabled,
        rounds_per_step: self.config.limits.max_steps_per_turn.unwrap_or(10_000) as usize,
      },
    });

    {
      let symbols = module_program.interpreter_mut().symbols();
      let mut symbols_brrw = symbols.borrow_mut();
      for (name, value_ref) in import_environment {
        let id = hash_str(&name);
        symbols_brrw.symbols.insert(id, value_ref);
        symbols_brrw.dictionary.borrow_mut().insert(id, name);
      }
    }

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ModuleExecutionStarted { module_version: version },
    )?;
    let scoped_source = module_source_for_scope(&source, scope)?;
    let result = self.run_module_source_on_program(context, &mut module_program, &scoped_source);
    let result = match result {
      Ok(value) => value,
      Err(error) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ModuleExecutionFailed {
            module_version: version,
            message: format!("{:?}", error),
          },
        )?;
        return Err(error);
      }
    };
    let mut exports = HashMap::new();
    {
      let symbols = module_program.interpreter_mut().symbols();
      let symbols_brrw = symbols.borrow();
      for export in exports_for_scope(&record, scope) {
        let id = hash_str(&export.name);
        let Some(value_ref) = symbols_brrw.get(id) else {
          let error = MechError::new(RuntimeModuleExportNotFound { dependency: record.id.to_string(), export: export.name.clone() }, None);
          self.emit_event_to_context(
            context,
            RuntimeEventKind::ModuleExecutionFailed {
              module_version: version,
              message: format!("{:?}", error),
            },
          )?;
          return Err(error);
        };
        exports.insert(export.name.clone(), value_ref.clone());
      }
    }

    let instance = ModuleInstance { version, exports, result };
    module_instances.insert(instance_key, instance.clone());
    self.emit_event_to_context(
      context,
      RuntimeEventKind::ModuleExecutionCompleted { module_version: version },
    )?;
    Ok(instance)
  }

  fn run_module_source_on_program(
    &mut self,
    context: &mut RuntimeContext,
    program: &mut MechProgram,
    source: &MechSourceCode,
  ) -> MResult<Value> {
    self.register_runtime_program_host_functions(context, program)?;

    let runtime_ptr: *mut MechRuntime = self;
    let context_ptr: *mut RuntimeContext = context;

    let previous_target = ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      slot.replace(Some(RuntimeProgramHostTarget {
        runtime: runtime_ptr,
        context: context_ptr,
      }))
    });

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ProgramStarted {
        task_id: context.task,
      },
    )?;

    let result = match source {
      MechSourceCode::String(source) => {
        let stripped = strip_module_declarations_for_execution(source);
        program.run_source(&MechSourceCode::String(stripped))
      }
      other => program.run_source(other),
    };

    ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      slot.replace(previous_target);
    });

    match &result {
      Ok(_) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramCompleted {
            task_id: context.task,
          },
        )?;
      }
      Err(error) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramFailed {
            task_id: context.task,
            message: format!("{:?}", error),
          },
        )?;
      }
    }

    result
  }

  fn build_address_environment_for_scope(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
    record: &ModuleVersionRecord,
    scope: &SourceScope,
    context_registry: &RuntimeContextRegistry,
    seen: &mut HashSet<(ModuleVersionId, SourceScope)>,
    module_instances: &mut HashMap<(ModuleVersionId, SourceScope), ModuleInstance>,
  ) -> MResult<HashMap<String, mech_core::ValRef>> {
    let scoped_refs = record
      .scopes
      .iter()
      .find(|metadata| &metadata.scope == scope)
      .map(|metadata| metadata.address_references.as_slice())
      .unwrap_or(&[]);

    let mut requested_by_scope: HashMap<SourceScope, Vec<SourceAddressReference>> = HashMap::new();
    let mut bindings = HashMap::new();
    for reference in scoped_refs {
      match resolve_runtime_address_target(record, scope, context_registry, &reference.target) {
        RuntimeAddressTarget::Interpreter(interpreter_scope) => {
          if !matches!(scope, SourceScope::Program) {
            return Err(MechError::new(UnknownAddressTarget { target: reference.target.clone() }, None));
          }
          requested_by_scope.entry(interpreter_scope).or_default().push(reference.clone());
        }
        RuntimeAddressTarget::Context(binding) => {
          let base_uri = runtime_context_base_uri(&binding);
          if !runtime_context_allows_read(&binding, &reference.name) {
            return Err(MechError::new(
              RuntimeResourceCapabilityDenied {
                context_name: binding.name.clone(),
                operation: "read".to_string(),
                path: reference.name.clone(),
              },
              None,
            ));
          }
          if !self.grants.allows(
            DEFAULT_RESOURCE_SUBJECT,
            &base_uri,
            &RuntimeCapabilityOperation::Read,
            &reference.name,
          ) {
            return Err(MechError::new(
              RuntimeCapabilityGrantDenied {
                subject: DEFAULT_RESOURCE_SUBJECT.to_string(),
                resource: base_uri,
                operation: RuntimeCapabilityOperation::Read,
                path: reference.name.clone(),
              },
              None,
            ));
          }
          let value = self.resources.read(RuntimeResourceReadRequest {
            base_uri,
            path: reference.name.clone(),
            context_name: binding.name.clone(),
          })?;
          bindings.insert(
            address_binding_key(&reference.target, &reference.name),
            Ref::new(value),
          );
        }
        RuntimeAddressTarget::Unknown => {
          return Err(MechError::new(UnknownAddressTarget { target: reference.target.clone() }, None));
        }
      }
    }

    for (interpreter_scope, requested_refs) in requested_by_scope {
      let instance = self.execute_module_isolated_for_scope(
        context,
        version,
        &interpreter_scope,
        seen,
        module_instances,
      )?;

      for reference in requested_refs {
        let Some(export_value) = instance.exports.get(&reference.name) else {
          return Err(MechError::new(RuntimeModuleExportNotFound { dependency: record.id.to_string(), export: reference.name.clone() }, None));
        };
        bindings.insert(address_binding_key(&reference.target, &reference.name), export_value.clone());
      }
    }

    Ok(bindings)
  }

  fn build_import_environment_for_scope(
    &mut self,
    context: &mut RuntimeContext,
    importer: &ModuleVersionRecord,
    scope: &SourceScope,
    module_instances: &HashMap<(ModuleVersionId, SourceScope), ModuleInstance>,
  ) -> MResult<HashMap<String, mech_core::ValRef>> {
    let mut bindings = HashMap::new();
    let mut ownership: HashMap<String, String> = HashMap::new();

    for edge in &importer.import_edges {
      if &edge.scope != scope {
        continue;
      }

      let import = &edge.import;
      let dependency = edge.dependency;

      let dependency_key = (dependency, SourceScope::Program);
      let Some(dependency_instance) = module_instances.get(&dependency_key) else {
        return Err(MechError::new(RuntimeInvalidOperationError { operation: "build_import_environment", reason: format!("dependency instance missing for {}", dependency) }, None));
      };

      match &import.kind {
        SourceImportKind::DependencyOnly | SourceImportKind::Namespace => {
          let Some(namespace) = module_namespace_for_import(import) else { continue; };
          for (export_name, export_value) in &dependency_instance.exports {
            let binding = format!("{}/{}", namespace, export_name);
            if let Some(first) = ownership.insert(binding.clone(), import.specifier.clone()) {
              return Err(MechError::new(RuntimeModuleImportConflict { binding, first_import: first, second_import: import.specifier.clone() }, None));
            }
            bindings.insert(format!("{}/{}", namespace, export_name), export_value.clone());
          }
        }
        SourceImportKind::Single { name } => {
          if matches!(import.alias, Some(crate::resolver::SourceImportAlias::Context(_))) {
            continue;
          }
          let Some(export_value) = dependency_instance.exports.get(name) else {
            return Err(MechError::new(RuntimeModuleExportNotFound { dependency: import.specifier.clone(), export: name.clone() }, None));
          };

          let binding = match &import.alias {
            Some(crate::resolver::SourceImportAlias::Value(alias)) => alias.clone(),
            Some(crate::resolver::SourceImportAlias::Context(_)) => continue,
            None => name.clone(),
          };

          if let Some(first) = ownership.insert(binding.clone(), import.specifier.clone()) {
            return Err(MechError::new(RuntimeModuleImportConflict { binding: binding.clone(), first_import: first, second_import: import.specifier.clone() }, None));
          }
          bindings.insert(binding, export_value.clone());
        }
        SourceImportKind::Wildcard => {
          for (export_name, export_value) in &dependency_instance.exports {
            if let Some(first) = ownership.insert(export_name.clone(), import.specifier.clone()) {
              return Err(MechError::new(RuntimeModuleImportConflict { binding: export_name.clone(), first_import: first, second_import: import.specifier.clone() }, None));
            }
            bindings.insert(export_name.clone(), export_value.clone());
          }
        }
      }

      self.emit_event_to_context(
        context,
        RuntimeEventKind::ModuleImportLinked {
          importer: importer.id,
          dependency,
          specifier: import.specifier.clone(),
        },
      )?;
    }

    Ok(bindings)
  }
}



fn module_source_for_scope(
  source: &MechSourceCode,
  scope: &SourceScope,
) -> MResult<MechSourceCode> {
  match scope {
    SourceScope::Program => Ok(source.clone()),
    SourceScope::Interpreter(interpreter) => {
      let MechSourceCode::String(source_text) = source else {
        return Err(MechError::new(
          RuntimeInvalidOperationError {
            operation: "run_module_scope",
            reason: "interpreter scope execution requires string source".to_string(),
          },
          None,
        ));
      };

      let tree = mech_syntax::parser::parse(source_text.trim())?;

      if let Some(source) = source_from_parsed_fenced_blocks(&tree, interpreter.namespace)? {
        return Ok(MechSourceCode::String(source));
      }

      Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "run_module_scope",
          reason: format!("interpreter scope `{}` not found", interpreter.namespace_str),
        },
        None,
      ))
    }
  }
}

fn source_from_parsed_fenced_blocks(
  tree: &mech_core::Program,
  namespace: u64,
) -> MResult<Option<String>> {
  let mut blocks = Vec::new();

  for section in &tree.body.sections {
    for element in &section.elements {
      if let mech_core::SectionElement::FencedMechCode(fenced) = element {
        if fenced.config.namespace == namespace {
          let block = source_from_parsed_fenced_code(fenced)?;
          blocks.push(block.trim_end().to_string());
        }
      }
    }
  }

  if blocks.is_empty() {
    Ok(None)
  } else {
    Ok(Some(blocks.join("\n")))
  }
}

fn source_from_parsed_fenced_code(
  fenced: &mech_core::FencedMechCode,
) -> MResult<String> {
  source_from_tokens(std::slice::from_ref(&fenced.source))
}

fn source_from_tokens(tokens: &[mech_core::Token]) -> MResult<String> {
  if tokens.is_empty() {
    return Ok(String::new());
  }

  if tokens.iter().any(|token| token.src_range.start.row == 0 || token.src_range.start.col == 0) {
    return Ok(tokens.iter().map(|token| token.to_string()).collect::<Vec<_>>().join(" "));
  }

  let mut source = String::new();
  let mut row = tokens[0].src_range.start.row;
  let mut col = tokens[0].src_range.start.col;

  for token in tokens {
    let start = &token.src_range.start;
    while row < start.row {
      source.push('\n');
      row += 1;
      col = 1;
    }
    while col < start.col {
      source.push(' ');
      col += 1;
    }

    let token_text = token.to_string();
    for ch in token_text.chars() {
      source.push(ch);
      if ch == '\n' {
        row += 1;
        col = 1;
      } else {
        col += 1;
      }
    }
  }

  Ok(source)
}

fn exports_for_scope<'a>(
  record: &'a ModuleVersionRecord,
  scope: &SourceScope,
) -> &'a [SourceExportDeclaration] {
  record
    .scopes
    .iter()
    .find(|metadata| &metadata.scope == scope)
    .map(|metadata| metadata.exports.as_slice())
    .unwrap_or(&[])
}

pub fn strip_module_declarations_for_execution(source: &str) -> String {
  source
    .lines()
    .filter(|line| {
      let trimmed = line.trim_start();
      !(trimmed.starts_with("+>") || trimmed.starts_with("<+") || trimmed.starts_with("@"))
    })
    .collect::<Vec<_>>()
    .join("\n")
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn runtime_has_interpreter_finds_root_interpreter() {
    let runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    assert!(runtime.has_interpreter(0));
  }

  #[test]
  fn runtime_output_value_for_interpreter_returns_value_after_run_string() {
    let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    let source = "```mech
1
```";
    let _ = runtime.run_string(source).unwrap();
    let root_id = runtime.program().interpreter().id;
    let output_id = {
      let out_values = runtime.program().interpreter().out_values.borrow();
      *out_values.keys().next().expect("expected output value after run_string")
    };
    let output = runtime.output_value_for_interpreter(root_id, output_id);
    assert!(output.is_some());
  }

  #[test]
  fn runtime_bind_ans_for_interpreter_binds_ans() {
    let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    let value = Value::U64(Ref::new(42));
    runtime.bind_ans_for_interpreter(0, &value).unwrap();
    let ans_id = hash_str("ans");
    let bound = runtime
      .program()
      .interpreter()
      .symbols()
      .borrow()
      .get(ans_id)
      .map(|value| value.borrow().clone());
    assert_eq!(bound, Some(value));
  }
}
