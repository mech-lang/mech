use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// CartesianProduct ------------------------------------------------------------------------

/// Keep eager Cartesian-product materialization within the same deterministic
/// output-cardinality boundary as the powerset kernel.
const MAX_CARTESIAN_PRODUCT_OUTPUT_CARDINALITY: usize = 65_536;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SetCartesianProductLimitExceeded {
    pub lhs: usize,
    pub rhs: usize,
    pub maximum: usize,
}

impl MechErrorKind for SetCartesianProductLimitExceeded {
    fn name(&self) -> &str {
        "SetCartesianProductLimitExceeded"
    }

    fn message(&self) -> String {
        format!(
            "set/cartesian-product inputs have cardinalities {} and {}, exceeding the maximum output cardinality of {}",
            self.lhs, self.rhs, self.maximum,
        )
    }
}

fn cartesian_product_output_len(lhs: usize, rhs: usize) -> MResult<usize> {
    let output_len = lhs.checked_mul(rhs).ok_or_else(|| {
        MechError::new(
            SetCartesianProductLimitExceeded {
                lhs,
                rhs,
                maximum: MAX_CARTESIAN_PRODUCT_OUTPUT_CARDINALITY,
            },
            None,
        )
        .with_compiler_loc()
    })?;
    if output_len > MAX_CARTESIAN_PRODUCT_OUTPUT_CARDINALITY {
        return Err(MechError::new(
            SetCartesianProductLimitExceeded {
                lhs,
                rhs,
                maximum: MAX_CARTESIAN_PRODUCT_OUTPUT_CARDINALITY,
            },
            None,
        )
        .with_compiler_loc());
    }
    Ok(output_len)
}

pub(crate) fn validate_set_cartesian_product_contract(args: &FunctionArgs) -> MResult<()> {
    let contract = "set_cartesian_product";
    let (lhs_len, lhs_kind) = match args.input_value(0) {
        Some(LegacyValue::Set(value)) => {
            let value = value.borrow();
            (value.set.len(), value.kind.clone())
        }
        _ => {
            return Err(function_shape_contract_violation(
                contract,
                "input 0 must be a set",
            ));
        }
    };
    let (rhs_len, rhs_kind) = match args.input_value(1) {
        Some(LegacyValue::Set(value)) => {
            let value = value.borrow();
            (value.set.len(), value.kind.clone())
        }
        _ => {
            return Err(function_shape_contract_violation(
                contract,
                "input 1 must be a set",
            ));
        }
    };
    cartesian_product_output_len(lhs_len, rhs_len)?;
    let output_kind = match args.output_value() {
        LegacyValue::Set(value) => value.borrow().kind.clone(),
        _ => {
            return Err(function_shape_contract_violation(
                contract,
                "output must be a set",
            ));
        }
    };
    let expected = ValueKind::Tuple(vec![lhs_kind, rhs_kind]);
    if output_kind != expected {
        return Err(function_shape_contract_violation(
            contract,
            format!("output element schema is {output_kind}, expected {expected}"),
        ));
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) struct SetCartesianProductFxn {
    lhs: Ref<MechSet>,
    rhs: Ref<MechSet>,
    out: Ref<MechSet>,
}
impl MechFunctionFactory for SetCartesianProductFxn {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Binary(out, arg1, arg2) => {
                let lhs: Ref<MechSet> = arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                let rhs: Ref<MechSet> = arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                let out: Ref<MechSet> = out.try_function_ref(FunctionArgumentRole::Output)?;
                Ok(Box::new(SetCartesianProductFxn { lhs, rhs, out }))
            }
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}
impl MechFunctionImpl for SetCartesianProductFxn {
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let lhs_ptr: &MechSet = &*(self.lhs.as_ptr());
            let rhs_ptr: &MechSet = &*(self.rhs.as_ptr());
            let output_len =
                cartesian_product_output_len(lhs_ptr.set.len(), rhs_ptr.set.len())?;

            // Construct the complete next value before replacing the reactive
            // output so a rejected expansion retains the previous result.
            let output_kind = ValueKind::Tuple(vec![lhs_ptr.kind.clone(), rhs_ptr.kind.clone()]);
            let mut next = MechSet::new(output_kind.clone(), output_len);
            for elem1 in &lhs_ptr.set {
                for elem2 in &rhs_ptr.set {
                    next.set.insert(LegacyValue::Tuple(Ref::new(MechTuple {
                        elements: vec![Box::new(elem1.clone()), Box::new(elem2.clone())],
                    })));
                }
            }
            next.sync_cardinality_from_contents();
            // Empty materialization does not erase the declared element
            // schema. Downstream native contracts must see the same tuple
            // kind regardless of the current input cardinalities.
            next.kind = output_kind;

            *self.out.as_mut_ptr() = next;
        };
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        LegacyValue::Set(self.out.clone())
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for SetCartesianProductFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("SetCartesianProductFxn");
        compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
    }
}

#[cfg(feature = "source")]
fn set_cartesian_product_fxn(lhs: LegacyValue, rhs: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    match (lhs, rhs) {
        (LegacyValue::Set(lhs), LegacyValue::Set(rhs)) => {
            let output_len =
                cartesian_product_output_len(lhs.borrow().set.len(), rhs.borrow().set.len())?;
            Ok(Box::new(SetCartesianProductFxn {
                lhs: lhs.clone(),
                rhs: rhs.clone(),
                out: Ref::new(MechSet::new(
                    ValueKind::Tuple(vec![lhs.borrow().kind.clone(), rhs.borrow().kind.clone()]),
                    output_len,
                )),
            }))
        }
        x => Err(MechError::new(
            UnhandledFunctionArgumentKind2 {
                arg: (x.0.kind(), x.1.kind()),
                fxn_name: "set/cartesian-product".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index_set(cardinality: usize) -> MechSet {
        MechSet::from_vec(
            (0..cardinality)
                .map(|index| LegacyValue::Index(Ref::new(index)))
                .collect(),
        )
    }

    #[test]
    fn cartesian_product_rejects_unbounded_initial_and_reactive_inputs() {
        let initial_error =
            cartesian_product_output_len(MAX_CARTESIAN_PRODUCT_OUTPUT_CARDINALITY + 1, 1)
                .unwrap_err();
        assert_eq!(
            initial_error.kind_name(),
            "SetCartesianProductLimitExceeded"
        );

        let lhs = Ref::new(index_set(2));
        let rhs = Ref::new(index_set(2));
        let out = Ref::new(MechSet::new(ValueKind::Empty, 0));
        let function = SetCartesianProductFxn {
            lhs: lhs.clone(),
            rhs: rhs.clone(),
            out: out.clone(),
        };
        function.solve_result().unwrap();
        assert_eq!(out.borrow().set.len(), 4);
        let previous = out.borrow().clone();

        *lhs.borrow_mut() = index_set(257);
        *rhs.borrow_mut() = index_set(257);
        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "SetCartesianProductLimitExceeded");
        assert_eq!(*out.borrow(), previous);
    }

    #[test]
    fn cartesian_product_preserves_tuple_schema_when_either_input_is_empty() {
        let lhs = Ref::new(MechSet::new(ValueKind::Index, 0));
        let rhs = Ref::new(index_set(2));
        let output_kind = ValueKind::Tuple(vec![ValueKind::Index, ValueKind::Index]);
        let out = Ref::new(MechSet::new(output_kind.clone(), 0));
        let function = SetCartesianProductFxn {
            lhs: lhs.clone(),
            rhs: rhs.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();

        assert!(out.borrow().set.is_empty());
        assert_eq!(out.borrow().kind, output_kind);
        validate_set_cartesian_product_contract(&FunctionArgs::Binary(
            LegacyValue::Set(out),
            LegacyValue::Set(lhs),
            LegacyValue::Set(rhs),
        ))
        .unwrap();
    }
}

#[cfg(feature = "source")]
pub struct SetCartesianProduct {}
#[cfg(feature = "source")]
impl FunctionSpecializer for SetCartesianProduct {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let lhs = arguments[0].clone();
        let rhs = arguments[1].clone();
        match set_cartesian_product_fxn(lhs.clone(), rhs.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(x) => match (lhs, rhs) {
                (LegacyValue::MutableReference(lhs), LegacyValue::MutableReference(rhs)) => {
                    set_cartesian_product_fxn(lhs.borrow().clone(), rhs.borrow().clone())
                }
                (lhs, LegacyValue::MutableReference(rhs)) => {
                    set_cartesian_product_fxn(lhs.clone(), rhs.borrow().clone())
                }
                (LegacyValue::MutableReference(lhs), rhs) => {
                    set_cartesian_product_fxn(lhs.borrow().clone(), rhs.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (x.0.kind(), x.1.kind()),
                        fxn_name: "set/cartesian-product".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
