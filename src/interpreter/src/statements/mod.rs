#![forbid(unsafe_code)]

use crate::*;

mod context;
mod destructure;
mod integrity;

pub use context::context_declaration;
#[cfg(feature = "tuple")]
pub use destructure::tuple_destructure;
#[cfg(feature = "invariant_define")]
pub use integrity::invariant_define;

mod op_assign;
mod variable_assign;
mod variable_define;

#[cfg(feature = "math")]
pub use op_assign::op_assign;
#[cfg(feature = "math_add_assign")]
pub use op_assign::add_assign;
#[cfg(feature = "math_sub_assign")]
pub use op_assign::sub_assign;
#[cfg(feature = "math_div_assign")]
pub use op_assign::mul_assign;
#[cfg(feature = "math_mul_assign")]
pub use op_assign::div_assign;
#[cfg(feature = "variable_assign")]
pub use variable_assign::variable_assign;
#[cfg(all(feature = "subscript", feature = "assign"))]
pub use variable_assign::subscript_ref;
#[cfg(feature = "variable_define")]
pub use variable_define::variable_define;

// Statements
// ----------------------------------------------------------------------------

pub fn statement(stmt: &Statement, env: Option<&Environment>, p: &InterpreterExecution<'_>) -> MResult<Value> {
  match stmt {
    Statement::ImportDeclaration(_) => Ok(Value::Empty),
    Statement::ExportDeclaration(_) => Ok(Value::Empty),
    Statement::ContextDeclaration(ctx) => context_declaration(ctx, p),
    #[cfg(feature = "tuple")]
    Statement::TupleDestructure(tpl_dstrct) => tuple_destructure(&tpl_dstrct, p),
    #[cfg(feature = "invariant_define")]
    Statement::InvariantDefine(inv_def) => invariant_define(&inv_def, p),
    #[cfg(feature = "variable_define")]
    Statement::VariableDefine(var_def) => variable_define(&var_def, p),
    #[cfg(feature = "variable_assign")]
    Statement::VariableAssign(var_assgn) => variable_assign(&var_assgn, env, p),
    #[cfg(feature = "kind_define")]
    Statement::KindDefine(knd_def) => kind_define(&knd_def, p),
    #[cfg(feature = "enum")]
    Statement::EnumDefine(enm_def) => {
      enum_define(&enm_def, p)?;
      Ok(Value::Empty)
    }
    #[cfg(feature = "math")]
    Statement::OpAssign(op_assgn) => op_assign(&op_assgn, env, p),
    #[cfg(feature = "state_machines")]
    Statement::FsmDeclare(fsm_decl) => fsm_declare(fsm_decl, env, p),
    //Statement::SplitTable => todo!(),
    //Statement::FlattenTable => todo!(),
    x => return Err(MechError::new(
        FeatureNotEnabledError,
        None
      ).with_compiler_loc().with_tokens(x.tokens())
    ),
  }
}

#[cfg(test)]
mod tests;

#[cfg(feature = "enum")]
pub fn enum_define(enm_def: &EnumDefine, p: &InterpreterExecution<'_>) -> MResult<()> {
  let id = enm_def.name.hash();
  let mut variants: Vec<(u64, Option<Value>)> = Vec::new();
  {
    let mut state_brrw = p.state.borrow_mut();
    for v in &enm_def.variants {
      let payload = match &v.value {
        Some(kind_annotation_node) => {
          let knd = kind_annotation(&kind_annotation_node.kind, p)?;
          let vk = knd.to_value_kind(&mut state_brrw.kinds)?;
          Some(Value::Kind(vk))
        }
        None => None,
      };
      variants.push((v.name.hash(), payload));
    }
  }
  let state = &p.state;
  let mut state_brrw = state.borrow_mut();
  let dictionary = state_brrw.dictionary.clone();
  {
    let mut dictionary_brrw = dictionary.borrow_mut();
    dictionary_brrw.insert(enm_def.name.hash(), enm_def.name.to_string());
    for variant in &enm_def.variants {
      dictionary_brrw.insert(variant.name.hash(), variant.name.to_string());
    }
  }
  let enm = MechEnum{id, variants, names: dictionary};
  let val = Value::Enum(Ref::new(enm.clone()));
  state_brrw.enums.insert(id, enm.clone());
  state_brrw.kinds.insert(id, val.kind());
  Ok(())
}

#[cfg(feature = "kind_define")]
pub fn kind_define(knd_def: &KindDefine, p: &InterpreterExecution<'_>) -> MResult<Value> {
  let id = knd_def.name.hash();
  let kind = kind_annotation(&knd_def.kind.kind, p)?;
  let value_kind = kind.to_value_kind(&p.state.borrow().kinds)?;
  let functions = p.functions();
  let mut kinds = &mut p.state.borrow_mut().kinds;
  kinds.insert(id, value_kind.clone());
  Ok(Value::Kind(value_kind))
}

#[cfg(all(feature = "enum", feature = "atom"))]
fn value_matches_enum_variant(value: &Value, enum_id: u64, state: &ProgramState) -> bool {
  let my_enum = match state.enums.get(&enum_id) {
    Some(enm) => enm,
    None => return false,
  };
  let names_brrw = my_enum.names.borrow();
  let atom_matches_variant = |variant_id: u64, atom_id: u64, atom_name: &str| {
    if variant_id == atom_id {
      return true;
    }
    let variant_name = match names_brrw.get(&variant_id) {
      Some(name) => name.as_str(),
      None => return false,
    };
    let short_variant = variant_name.rsplit('/').next().unwrap_or(variant_name);
    let short_atom = atom_name.rsplit('/').next().unwrap_or(atom_name);
    short_variant == short_atom
  };
  match value {
    Value::Enum(enum_value) => {
      let enum_value_brrw = enum_value.borrow();
      if enum_value_brrw.id != enum_id {
        return false;
      }
      if enum_value_brrw.variants.len() != 1 {
        return false;
      }
      let (variant_id, payload) = &enum_value_brrw.variants[0];
      let (_, declared_payload_kind) = match my_enum.variants.iter().find(|(known_variant, _)| {
        *known_variant == *variant_id
      }) {
        Some(v) => v,
        None => return false,
      };
      match (payload, declared_payload_kind) {
        (None, None) => true,
        (Some(payload_value), Some(Value::Kind(expected_kind))) => match expected_kind {
          ValueKind::Enum(inner_enum_id, _) => value_matches_enum_variant(payload_value, *inner_enum_id, state),
          _ => {
            payload_value.kind() == expected_kind.clone() ||
            ConvertKind{}.compile(&vec![payload_value.clone(), Value::Kind(expected_kind.clone())]).is_ok()
          }
        },
        _ => false,
      }
    }
    Value::Atom(atom_variant) => {
      let atom_brrw = atom_variant.borrow();
      let variant_id = atom_brrw.id();
      let atom_name = atom_brrw.name();
      my_enum.variants.iter().any(|(known_variant, payload_kind)| {
        atom_matches_variant(*known_variant, variant_id, &atom_name) && payload_kind.is_none()
      })
    }
    #[cfg(feature = "tuple")]
    Value::Tuple(tuple_val) => {
      let tuple_brrw = tuple_val.borrow();
      if tuple_brrw.elements.len() != 2 {
        return false;
      }
      let variant_atom = match tuple_brrw.elements[0].as_ref() {
        Value::Atom(atom) => atom.borrow(),
        _ => return false,
      };
      let variant_id = variant_atom.id();
      let atom_name = variant_atom.name();
      let payload = tuple_brrw.elements[1].as_ref();
      let (_, declared_payload_kind) = match my_enum.variants.iter().find(|(known_variant, _)| {
        atom_matches_variant(*known_variant, variant_id, &atom_name)
      }) {
        Some(v) => v,
        None => return false,
      };
      match declared_payload_kind {
        Some(Value::Kind(expected_kind)) => match expected_kind {
          ValueKind::Enum(inner_enum_id, _) => value_matches_enum_variant(payload, *inner_enum_id, state),
          _ => {
            payload.kind() == expected_kind.clone() ||
            ConvertKind{}.compile(&vec![payload.clone(), Value::Kind(expected_kind.clone())]).is_ok()
          }
        },
        _ => false,
      }
    }
    _ => false,
  }
}

#[cfg(feature = "state_machines")]
pub fn fsm_declare(fsm_decl: &FsmDeclare, env: Option<&Environment>, p: &InterpreterExecution<'_>) -> MResult<Value> {
  let result = crate::state_machines::execute_fsm_pipe(&fsm_decl.pipe, env, p)?;
  let id = fsm_decl.fsm.name.hash();
  let name = fsm_decl.fsm.name.to_string();
  #[cfg(feature = "symbol_table")]
  {
    let symbols = p.symbols();
    let mut symbols_brrw = symbols.borrow_mut();
    symbols_brrw.insert(id, detach_variable_value(&result), false);
    symbols_brrw.dictionary.borrow_mut().insert(id, name);
  }
  Ok(result)
}

fn detach_variable_value(value: &Value) -> Value {
  match value {
    Value::MutableReference(reference) => detach_variable_value(&reference.borrow()),
    _ => value.clone(),
  }
}

#[derive(Debug, Clone)]
pub struct AddressedAssignmentUnsupported;
impl MechErrorKind for AddressedAssignmentUnsupported {
  fn name(&self) -> &str { "AddressedAssignmentUnsupported" }
  fn message(&self) -> String {
    "addressed assignment is not supported yet".to_string()
  }
}

#[derive(Debug, Clone)]
pub struct UnableToConvertAtomToEnumVariantError {
  pub atom_name: String,
  pub target_enum_variant_name: String,
}
impl MechErrorKind for UnableToConvertAtomToEnumVariantError {
  fn name(&self) -> &str {
    "UnableToConvertAtomToEnumVariant"
  }
  fn message(&self) -> String {
    format!("Unable to convert atom variant `{} to enum <{}>", self.atom_name, self.target_enum_variant_name)
  }
}

#[derive(Debug, Clone)]
pub struct UnableToConvertAtomError {
  pub atom_id: u64,
}
impl MechErrorKind for UnableToConvertAtomError {
  fn name(&self) -> &str {
    "UnableToConvertAtom"
  }
  fn message(&self) -> String {
    format!("Unable to atom  {}", self.atom_id)
  }
}

#[derive(Debug, Clone)]
pub struct VariableAlreadyDefinedError {
  pub id: u64,
}
impl MechErrorKind for VariableAlreadyDefinedError {
  fn name(&self) -> &str { "VariableAlreadyDefined" }
  fn message(&self) -> String {
    format!("Variable already defined: {}", self.id)
  }
}

#[derive(Debug, Clone)]
pub struct UndefinedVariableError {
  pub id: u64,
  pub name: String,
}
impl MechErrorKind for UndefinedVariableError {
  fn name(&self) -> &str { "UndefinedVariable" }

  fn message(&self) -> String {
    format!("Undefined variable `{}` (id: {})", self.name, self.id)
  }
}

#[derive(Debug, Clone)]
pub struct NotMutableError {
  pub id: u64,
}
impl MechErrorKind for NotMutableError {
  fn name(&self) -> &str { "NotMutable" }
  fn message(&self) -> String {
    format!("Variable is not mutable: {}", self.id)
  }
}

#[cfg(feature = "record")]
#[derive(Debug, Clone)]
pub struct UnableToConvertRecordError {
  pub source_record_kind: ValueKind,
  pub target_record_kind: ValueKind,
}
#[cfg(feature = "record")]
impl MechErrorKind for UnableToConvertRecordError {
  fn name(&self) -> &str {
    "UnableToConvertRecord"
  }
  fn message(&self) -> String {
    format!("Unable to convert record of kind `{:?}` to record of kind `{:?}`", self.source_record_kind, self.target_record_kind)
  }
}
