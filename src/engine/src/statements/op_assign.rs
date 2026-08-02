#[cfg(feature = "math")]
use super::variable_assign::assignment_registration_operand;
#[cfg(feature = "math")]
use super::{AddressedAssignmentUnsupported, NotMutableError, UndefinedVariableError};
#[cfg(all(
    any(
        feature = "math_add_assign",
        feature = "math_sub_assign",
        feature = "math_div_assign",
        feature = "math_mul_assign"
    ),
    any(feature = "subscript_formula", feature = "subscript_range")
))]
use crate::FunctionSpecializer;
#[cfg(any(
    feature = "math_add_assign",
    feature = "math_sub_assign",
    feature = "math_div_assign",
    feature = "math_mul_assign"
))]
use crate::Subscript;
#[cfg(all(
    any(
        feature = "math_add_assign",
        feature = "math_sub_assign",
        feature = "math_div_assign",
        feature = "math_mul_assign"
    ),
    feature = "subscript_range"
))]
use crate::subscript_range;
#[cfg(feature = "math_add_assign")]
use crate::{AddAssignRange, AddAssignRangeAll, AddAssignValue};
#[cfg(feature = "math_div_assign")]
use crate::{DivAssignRange, DivAssignRangeAll, DivAssignValue};
use crate::{Environment, InterpreterExecution, MResult, Value};
#[cfg(all(
    any(
        feature = "math_add_assign",
        feature = "math_sub_assign",
        feature = "math_div_assign",
        feature = "math_mul_assign"
    ),
    feature = "subscript_formula"
))]
use crate::{MatrixAssignScalar, MatrixAssignScalarAll, subscript_formula_ix};
#[cfg(feature = "math")]
use crate::{
    MechError, OpAssign, OpAssignOp, execute_catalog_operation_with_registration_arguments,
    expression,
};
#[cfg(all(
    any(
        feature = "math_add_assign",
        feature = "math_sub_assign",
        feature = "math_div_assign",
        feature = "math_mul_assign"
    ),
    any(feature = "subscript_formula", feature = "subscript_range")
))]
use crate::{MechFunction, OperationId};
#[cfg(feature = "math_mul_assign")]
use crate::{MulAssignRange, MulAssignRangeAll, MulAssignValue};
#[cfg(feature = "math_sub_assign")]
use crate::{SubAssignRange, SubAssignRangeAll, SubAssignValue};
#[cfg(any(
    feature = "math_add_assign",
    feature = "math_sub_assign",
    feature = "math_div_assign",
    feature = "math_mul_assign"
))]
use paste::paste;

#[cfg(feature = "math")]
pub fn op_assign(
    op_assgn: &OpAssign,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<Value> {
    let mut source = expression(&op_assgn.expression, env, p)?;
    let slc = &op_assgn.target;
    if slc.context.is_some() {
        return Err(MechError::new(AddressedAssignmentUnsupported, None)
            .with_compiler_loc()
            .with_tokens(slc.tokens()));
    }
    let id = slc.name.hash();
    let sink = {
        let mut state_brrw = p.state.borrow_mut();
        match state_brrw.get_mutable_symbol(id) {
      Some(val) => val.borrow().clone(),
      None => {
        match state_brrw.contains_symbol(id) {
          true => return Err(MechError::new(
            NotMutableError { id },
            Some("(!)> Mutable variables are defined with the `~` operator. *e.g.*: {{~x := 123}}".to_string()),
          ).with_compiler_loc().with_tokens(slc.name.tokens())),
          false => return Err(MechError::new(
            UndefinedVariableError { id, name: slc.name.to_string() },
            Some("(!)> Variables are defined with the `:=` operator. *e.g.*: {{x := 123}}".to_string()),
          ).with_compiler_loc().with_tokens(slc.name.tokens())),
        }
      }
    }
    };
    match &slc.subscript {
        Some(sbscrpt) => {
            // todo: this only works for the first subscript, it needs to work for multiple subscripts
            for s in sbscrpt {
                let fxn = match op_assgn.op {
                    #[cfg(feature = "math_add_assign")]
                    OpAssignOp::Add => add_assign(&s, &sink, &source, env, p)?,
                    #[cfg(feature = "math_sub_assign")]
                    OpAssignOp::Sub => sub_assign(&s, &sink, &source, env, p)?,
                    #[cfg(feature = "math_div_assign")]
                    OpAssignOp::Div => div_assign(&s, &sink, &source, env, p)?,
                    #[cfg(feature = "math_mul_assign")]
                    OpAssignOp::Mul => mul_assign(&s, &sink, &source, env, p)?,
                    _ => todo!(),
                };
                return Ok(fxn);
            }
        }
        None => {
            let plan = p.plan();
            let registration_source = assignment_registration_operand(&source);
            let compile_arguments = vec![sink, source];
            let registration_arguments = vec![registration_source];
            return match op_assgn.op {
                #[cfg(feature = "math_add_assign")]
                OpAssignOp::Add => execute_catalog_operation_with_registration_arguments(
                    p,
                    &plan,
                    "assign/add",
                    compile_arguments,
                    registration_arguments,
                ),
                #[cfg(feature = "math_sub_assign")]
                OpAssignOp::Sub => execute_catalog_operation_with_registration_arguments(
                    p,
                    &plan,
                    "math/sub-assign",
                    compile_arguments,
                    registration_arguments,
                ),
                #[cfg(feature = "math_div_assign")]
                OpAssignOp::Div => execute_catalog_operation_with_registration_arguments(
                    p,
                    &plan,
                    "math/div-assign",
                    compile_arguments,
                    registration_arguments,
                ),
                #[cfg(feature = "math_mul_assign")]
                OpAssignOp::Mul => execute_catalog_operation_with_registration_arguments(
                    p,
                    &plan,
                    "math/mul-assign",
                    compile_arguments,
                    registration_arguments,
                ),
                _ => todo!(),
            };
        }
    }
    unreachable!(); // subscript should have thrown an error if we can't access an element
}

#[cfg(all(
    any(
        feature = "math_add_assign",
        feature = "math_sub_assign",
        feature = "math_div_assign",
        feature = "math_mul_assign"
    ),
    any(feature = "subscript_formula", feature = "subscript_range")
))]
fn catalog_op_assignment_function(
    p: &InterpreterExecution<'_>,
    canonical_name: &str,
    arguments: &[Value],
) -> MResult<Box<dyn MechFunction>> {
    p.specialize_visible_operation_named(
        OperationId::from_name(canonical_name),
        Some(canonical_name),
        arguments,
    )
}

macro_rules! op_assign {
  ($fxn_name:ident, $op:tt, $range_operation:literal, $range_all_operation:literal) => {
    paste!{
      pub fn $fxn_name(sbscrpt: &Subscript, sink: &Value, source: &Value, env: Option<&Environment>, p: &InterpreterExecution<'_>) -> MResult<Value> {
        let plan = p.plan();
        match sbscrpt {
          Subscript::Dot(x) => {
            todo!()
          },
          Subscript::DotInt(x) => {
            todo!()
          },
          Subscript::Swizzle(x) => {
            todo!()
          },
          Subscript::Bracket(subs) => {
            let mut fxn_input = vec![sink.clone()];
            match &subs[..] {
              #[cfg(feature = "subscript_formula")]
              [Subscript::Formula(ix)] => {
                fxn_input.push(source.clone());
                let ixes = subscript_formula_ix(&subs[0], env, p)?;
                let shape = ixes.shape();
                fxn_input.push(ixes);
                match shape[..] {
                  [1,1] => { plan.borrow_mut().push(catalog_op_assignment_function(p, "assign", &fxn_input)?); }
                  [1,n] => { plan.borrow_mut().push(catalog_op_assignment_function(p, $range_operation, &fxn_input)?); }
                  [n,1] => { plan.borrow_mut().push(catalog_op_assignment_function(p, $range_operation, &fxn_input)?); }
                  _ => todo!(),
                }
              },
              #[cfg(feature = "subscript_formula")]
              [Subscript::Formula(ix1),Subscript::All] => {
                fxn_input.push(source.clone());
                let ix = subscript_formula_ix(&subs[0], env, p)?;
                let shape = ix.shape();
                fxn_input.push(ix);
                fxn_input.push(Value::IndexAll);
                match shape[..] {
                  [1,1] => { plan.borrow_mut().push(catalog_op_assignment_function(p, "assign", &fxn_input)?); }
                  [1,n] => { plan.borrow_mut().push(catalog_op_assignment_function(p, $range_all_operation, &fxn_input)?); }
                  [n,1] => { plan.borrow_mut().push(catalog_op_assignment_function(p, $range_all_operation, &fxn_input)?); }
                  _ => todo!(),
                }
              },
              #[cfg(feature = "subscript_range")]
              [Subscript::Range(ix)] => {
                fxn_input.push(source.clone());
                let ixes = subscript_range(&subs[0], env, p)?;
                fxn_input.push(ixes);
                plan.borrow_mut().push(catalog_op_assignment_function(p, $range_operation, &fxn_input)?);
              },
              #[cfg(feature = "subscript_range")]
              [Subscript::Range(ix), Subscript::All] => {
                fxn_input.push(source.clone());
                let ixes = subscript_range(&subs[0], env, p)?;
                fxn_input.push(ixes);
                fxn_input.push(Value::IndexAll);
                plan.borrow_mut().push(catalog_op_assignment_function(p, $range_all_operation, &fxn_input)?);
              },
              x => todo!("{:?}", x),
            };
            let plan_brrw = plan.borrow();
            let mut new_fxn = &plan_brrw.last().unwrap();
            new_fxn.solve();
            let res = new_fxn.out();
            return Ok(res);
          },
          Subscript::Brace(x) => todo!(),
          x => todo!("{:?}", x),
        }
      }
    }
  };
}

#[cfg(feature = "math_add_assign")]
op_assign!(
    add_assign,
    Add,
    "math/add-assign/range",
    "math/add-assign/range-all"
);
#[cfg(feature = "math_sub_assign")]
op_assign!(
    sub_assign,
    Sub,
    "math/sub-assign/range",
    "math/sub-assign/range-all"
);
#[cfg(feature = "math_mul_assign")]
op_assign!(
    mul_assign,
    Mul,
    "math/mul-assign/range",
    "math/mul-assign/range-all"
);
#[cfg(feature = "math_div_assign")]
op_assign!(
    div_assign,
    Div,
    "math/div-assign/range",
    "math/div-assign/range-all"
);
//#[cfg(feature = "math_pow")]
//op_assign!(pow_assign, Pow);
