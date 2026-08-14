#![forbid(unsafe_code)]

use crate::{
    Environment, FeatureNotEnabledError, InterpreterExecution, LegacyValue, MResult, MechError,
    Statement,
};

mod context;
mod destructure;
mod integrity;

pub(crate) use context::{context_assign, context_read};
pub use context::{context_declaration, context_send};
#[cfg(feature = "tuple")]
pub use destructure::tuple_destructure;
#[cfg(feature = "invariant_define")]
pub use integrity::invariant_define;

mod enums;
mod errors;
mod kinds;
mod state_machines;

#[cfg(feature = "enum")]
pub use enums::enum_define;
#[cfg(feature = "record")]
pub use errors::UnableToConvertRecordError;
pub use errors::{
    AddressedAssignmentUnsupported, NotMutableError, UnableToConvertAtomError,
    UnableToConvertAtomToEnumVariantError, UndefinedContextError, UndefinedVariableError,
    VariableAlreadyDefinedError,
};
#[cfg(feature = "kind_define")]
pub use kinds::kind_define;
#[cfg(feature = "state_machines")]
pub use state_machines::fsm_declare;

mod op_assign;
mod variable_assign;
mod variable_define;

#[cfg(feature = "math_add_assign")]
pub use op_assign::add_assign;
#[cfg(feature = "math_div_assign")]
pub use op_assign::div_assign;
#[cfg(feature = "math_mul_assign")]
pub use op_assign::mul_assign;
#[cfg(any(
    feature = "math_add_assign",
    feature = "math_sub_assign",
    feature = "math_div_assign",
    feature = "math_mul_assign"
))]
pub use op_assign::op_assign;
#[cfg(feature = "math_sub_assign")]
pub use op_assign::sub_assign;
#[cfg(all(feature = "subscript", feature = "assign"))]
pub use variable_assign::subscript_ref;
#[cfg(feature = "variable_assign")]
pub use variable_assign::variable_assign;
#[cfg(feature = "variable_define")]
pub use variable_define::variable_define;

// Statements
// ----------------------------------------------------------------------------

pub fn statement(
    stmt: &Statement,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    match stmt {
        Statement::ImportDeclaration(_) => Ok(LegacyValue::Empty),
        Statement::ExportDeclaration(_) => Ok(LegacyValue::Empty),
        Statement::ContextDeclaration(ctx) => context_declaration(ctx, p),
        Statement::ContextSend(send) => context_send(send, p),
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
            Ok(LegacyValue::Empty)
        }
        #[cfg(any(
            feature = "math_add_assign",
            feature = "math_sub_assign",
            feature = "math_div_assign",
            feature = "math_mul_assign"
        ))]
        Statement::OpAssign(op_assgn) => op_assign(&op_assgn, env, p),
        #[cfg(feature = "state_machines")]
        Statement::FsmDeclare(fsm_decl) => fsm_declare(fsm_decl, env, p),
        //Statement::SplitTable => todo!(),
        //Statement::FlattenTable => todo!(),
        x => {
            return Err(MechError::new(FeatureNotEnabledError, None)
                .with_compiler_loc()
                .with_tokens(x.tokens()));
        }
    }
}

#[cfg(test)]
mod tests;
