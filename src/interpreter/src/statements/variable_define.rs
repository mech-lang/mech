#[cfg(feature = "variable_define")]
use super::{
  AddressedAssignmentUnsupported, VariableAlreadyDefinedError, detach_variable_value,
};
#[cfg(all(
  feature = "variable_define",
  feature = "kind_annotation",
  feature = "convert",
  feature = "atom"
))]
use super::UnableToConvertAtomError;
#[cfg(all(
  feature = "variable_define",
  feature = "kind_annotation",
  feature = "convert",
  feature = "atom",
  feature = "enum"
))]
use super::{UnableToConvertAtomToEnumVariantError, value_matches_enum_variant};
#[cfg(all(
  feature = "variable_define",
  feature = "kind_annotation",
  feature = "convert",
  feature = "record"
))]
use super::UnableToConvertRecordError;
#[cfg(feature = "variable_define")]
use crate::{
  InterpreterExecution, MResult, MechError, NativeFunctionCompiler, Ref, Value, VariableDefine,
  execute_initialized_indexed_compiler, expression,
};
#[cfg(all(
  feature = "variable_define",
  feature = "kind_annotation",
  feature = "convert"
))]
use crate::{ConvertKind, ValueKind, kind_annotation};
#[cfg(all(
  feature = "variable_define",
  feature = "kind_annotation",
  feature = "convert",
  feature = "matrix"
))]
use crate::{ConvertMatToMat, ConvertScalarToMat};
#[cfg(all(feature = "variable_define", feature = "subscript_formula"))]
use crate::{
  mark_string_access_value_live, reset_current_string_access_expression_live,
  take_current_string_access_expression_live,
};
#[cfg(feature = "variable_define")]
use crate::stdlib::define::VarDefine;

#[cfg(feature = "variable_define")]
pub fn variable_define(var_def: &VariableDefine, p: &InterpreterExecution<'_>) -> MResult<Value> {
  if var_def.var.context.is_some() {
    return Err(MechError::new(AddressedAssignmentUnsupported, None)
      .with_compiler_loc()
      .with_tokens(var_def.var.tokens()));
  }
  let var_id = var_def.var.name.hash();
  let var_name = var_def.var.name.to_string();
  {
    let symbols = p.symbols();
    if symbols.borrow().contains(var_id) {
      return Err(MechError::new(
        VariableAlreadyDefinedError { id: var_id },
        None
      ).with_compiler_loc().with_tokens(var_def.var.name.tokens()));
    }
  }
  let plan = p.plan();
  #[cfg(feature = "subscript_formula")]
  reset_current_string_access_expression_live(p);
  let mut result = expression(&var_def.expression, None, p)?;
  #[cfg(feature = "subscript_formula")]
  let string_access_result_is_live = take_current_string_access_expression_live(p);
  #[cfg(all(feature = "kind_annotation", feature = "convert"))]
  if let Some(knd_anntn) =  &var_def.var.kind {
    let knd = kind_annotation(&knd_anntn.kind,p)?;
    let mut state_brrw = &mut p.state.borrow_mut();
    let target_knd = knd.to_value_kind(&mut state_brrw.kinds)?;
    // Do kind checking
    match (&result, &target_knd) {
      // Atom is a variant of an enum
      #[cfg(all(feature = "atom", feature = "enum"))]
      (Value::Atom(atom_variant), ValueKind::Enum(enum_id, target_enum_variant_name)) => {
        let atom_name = atom_variant.borrow().name();
        if !value_matches_enum_variant(&result, *enum_id, &*state_brrw) {
          return Err(MechError::new(
            UnableToConvertAtomToEnumVariantError { atom_name: atom_name.clone(), target_enum_variant_name: target_enum_variant_name.clone() },
            None
          ).with_compiler_loc().with_tokens(var_def.expression.tokens()));
        }
      }
      #[cfg(all(feature = "tuple", feature = "atom", feature = "enum"))]
      (Value::Tuple(tuple_val), ValueKind::Enum(enum_id, target_enum_variant_name)) => {
        let atom_name = format!("{:?}", tuple_val);
        if !value_matches_enum_variant(&result, *enum_id, &*state_brrw) {
          return Err(MechError::new(
            UnableToConvertAtomToEnumVariantError { atom_name, target_enum_variant_name: target_enum_variant_name.clone() },
            None
          ).with_compiler_loc().with_tokens(var_def.expression.tokens()));
        }
      }
      // Atoms can't convert into anything else.
      #[cfg(feature = "atom")]
      (Value::Atom(given_variant_id), target_kind) => {
        return Err(MechError::new(
          UnableToConvertAtomError { atom_id: given_variant_id.borrow().0.0},
          None
        ).with_compiler_loc().with_tokens(var_def.expression.tokens()));
      }
      #[cfg(feature = "record")]
      (Value::Record(rec), ref target_kind @ ValueKind::Record(target_rec_knd)) => {
        let rec_brrw = rec.borrow();
        let rec_knd = rec_brrw.kind();
        if &rec_knd != *target_kind {
          return Err(MechError::new(
            UnableToConvertRecordError { source_record_kind: rec_knd.clone(), target_record_kind: (*target_kind).clone() },
            None
          ).with_compiler_loc().with_tokens(var_def.expression.tokens()));
        }
      }
      #[cfg(feature = "matrix")]
      (Value::MutableReference(v), ValueKind::Matrix(target_matrix_knd,_)) => {
        let value = v.borrow().clone();
        if value.is_matrix() {
          result = execute_initialized_indexed_compiler(p, &plan, &ConvertMatToMat{}, vec![result.clone(), Value::Kind(target_knd.clone())])?;
        } else {
          let value_kind = value.kind();
          if value_kind.deref_kind() != target_matrix_knd.as_ref().clone() && value_kind != *target_matrix_knd.clone() {
            result = execute_initialized_indexed_compiler(p, &plan, &ConvertKind{}, vec![result.clone(), Value::Kind(target_matrix_knd.as_ref().clone())])?;
          };
          result = execute_initialized_indexed_compiler(p, &plan, &ConvertScalarToMat{}, vec![result.clone(), Value::Kind(target_knd.clone())])?;
        }
      }
      #[cfg(feature = "matrix")]
      (value, ValueKind::Matrix(target_matrix_knd,_)) => {
        if value.is_matrix() {
          result = execute_initialized_indexed_compiler(p, &plan, &ConvertMatToMat{}, vec![result.clone(), Value::Kind(target_knd.clone())])?;
        } else {
          let value_kind = value.kind();
          if value_kind.deref_kind() != target_matrix_knd.as_ref().clone() && value_kind != *target_matrix_knd.clone() {
            result = execute_initialized_indexed_compiler(p, &plan, &ConvertKind{}, vec![result.clone(), Value::Kind(target_matrix_knd.as_ref().clone())])?;
          };
          result = execute_initialized_indexed_compiler(p, &plan, &ConvertScalarToMat{}, vec![result.clone(), Value::Kind(target_knd.clone())])?;
        }
      }
      // Kind isn't checked
      x => {
        result = execute_initialized_indexed_compiler(p, &plan, &ConvertKind{}, vec![result.clone(), Value::Kind(target_knd)])?;
      },
    };
    let detached_result = detach_variable_value(&result);
    #[cfg(feature = "subscript_formula")]
    if string_access_result_is_live {
      mark_string_access_value_live(p, &detached_result);
    }
    // Save symbol to interpreter
    let val_ref = state_brrw.save_symbol(var_id, var_name.clone(), detached_result.clone(), var_def.mutable);
    // Add variable define step to plan
    let var_define_arguments = vec![detached_result.clone(), Value::String(Ref::new(var_name.clone())), Value::Bool(Ref::new(var_def.mutable))];
    let var_def_fxn = VarDefine{}.compile(&var_define_arguments)?;
    plan.register_function(var_def_fxn, &[])?;
    return Ok(detached_result);
  }
  let mut state_brrw = p.state.borrow_mut();
  let detached_result = detach_variable_value(&result);
  #[cfg(feature = "subscript_formula")]
  if string_access_result_is_live {
    mark_string_access_value_live(p, &detached_result);
  }
  // Save symbol to interpreter
  let val_ref = state_brrw.save_symbol(var_id,var_name.clone(),detached_result.clone(),var_def.mutable);
  // Add variable define step to plan
  let var_define_arguments = vec![detached_result.clone(), Value::String(Ref::new(var_name.clone())), Value::Bool(Ref::new(var_def.mutable))];
  let var_def_fxn = VarDefine{}.compile(&var_define_arguments)?;
  plan.register_function(var_def_fxn, &[])?;
  return Ok(detached_result);
}

