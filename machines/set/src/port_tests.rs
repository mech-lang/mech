#[cfg(all(
    feature = "union",
    feature = "difference",
    feature = "intersection",
    feature = "powerset",
    feature = "cartesian_product",
    feature = "subset",
    feature = "equals",
    feature = "size",
    feature = "bool",
    feature = "u64"
))]
mod typed_factories {
    use crate::{
        operations::{
            cartesian_product::SetCartesianProductFxn, difference::SetDifferenceFxn,
            intersection::SetIntersectionFxn, powerset::SetPowersetFxn, union::SetUnionFxn,
        },
        relations::{equals::SetEqualsFxn, subset::SetSubsetFxn},
        setdata::size::SetSizeFxn,
    };
    use mech_core::{
        FunctionArgs, FunctionInvocation, IncorrectNumberOfArguments, LegacyValue, MechError,
        MechFunctionFactory, MechSet, Ref, ToValue, ValueKind,
    };

    fn set(values: &[usize]) -> Ref<MechSet> {
        Ref::new(MechSet::from_vec(
            values
                .iter()
                .copied()
                .map(|value| LegacyValue::Index(Ref::new(value)))
                .collect(),
        ))
    }

    fn set_output() -> Ref<MechSet> {
        Ref::new(MechSet::new(ValueKind::Empty, 0))
    }

    fn factory_error(
        result: Result<Box<dyn mech_core::MechFunction>, MechError>,
    ) -> MechError {
        match result {
            Ok(_) => panic!("factory unexpectedly accepted invalid arguments"),
            Err(error) => error,
        }
    }

    fn assert_binary_set_factories_are_equivalent<F>()
    where
        F: MechFunctionFactory,
    {
        let lhs = set(&[1, 2]);
        let rhs = set(&[2, 3]);
        let legacy_out = set_output();
        let invocation_out = set_output();

        let legacy = F::new(FunctionArgs::Binary(
            legacy_out.to_value(),
            lhs.to_value(),
            rhs.to_value(),
        ))
        .unwrap();
        let invocation = F::new_invocation(
            FunctionArgs::Binary(
                invocation_out.to_value(),
                lhs.to_value(),
                rhs.to_value(),
            )
            .into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();

        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
    }

    fn assert_binary_bool_factories_are_equivalent<F>()
    where
        F: MechFunctionFactory,
    {
        let lhs = set(&[1, 2]);
        let rhs = set(&[1, 2, 3]);
        let legacy_out = Ref::new(false);
        let invocation_out = Ref::new(false);

        let legacy = F::new(FunctionArgs::Binary(
            legacy_out.to_value(),
            lhs.to_value(),
            rhs.to_value(),
        ))
        .unwrap();
        let invocation = F::new_invocation(
            FunctionArgs::Binary(
                invocation_out.to_value(),
                lhs.to_value(),
                rhs.to_value(),
            )
            .into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();

        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
    }

    #[test]
    fn selected_binary_set_factories_match_the_legacy_adapter() {
        assert_binary_set_factories_are_equivalent::<SetUnionFxn>();
        assert_binary_set_factories_are_equivalent::<SetDifferenceFxn>();
        assert_binary_set_factories_are_equivalent::<SetIntersectionFxn>();
        assert_binary_set_factories_are_equivalent::<SetCartesianProductFxn>();
    }

    #[test]
    fn selected_relation_factories_match_the_legacy_adapter() {
        assert_binary_bool_factories_are_equivalent::<SetSubsetFxn>();
        assert_binary_bool_factories_are_equivalent::<SetEqualsFxn>();
    }

    #[test]
    fn powerset_and_size_factories_match_the_legacy_adapter() {
        let input = set(&[1, 2]);
        let legacy_powerset_out = set_output();
        let invocation_powerset_out = set_output();
        let legacy_powerset = SetPowersetFxn::new(FunctionArgs::Unary(
            legacy_powerset_out.to_value(),
            input.to_value(),
        ))
        .unwrap();
        let invocation_powerset = SetPowersetFxn::new_invocation(
            FunctionArgs::Unary(invocation_powerset_out.to_value(), input.to_value()).into(),
        )
        .unwrap();
        legacy_powerset.solve_result().unwrap();
        invocation_powerset.solve_result().unwrap();
        assert_eq!(
            *legacy_powerset_out.borrow(),
            *invocation_powerset_out.borrow()
        );

        let legacy_size_out = Ref::new(0_u64);
        let invocation_size_out = Ref::new(0_u64);
        let legacy_size = SetSizeFxn::new(FunctionArgs::Unary(
            legacy_size_out.to_value(),
            input.to_value(),
        ))
        .unwrap();
        let invocation_size = SetSizeFxn::new_invocation(
            FunctionArgs::Unary(invocation_size_out.to_value(), input.to_value()).into(),
        )
        .unwrap();
        legacy_size.solve_result().unwrap();
        invocation_size.solve_result().unwrap();
        assert_eq!(*legacy_size_out.borrow(), *invocation_size_out.borrow());
    }

    #[test]
    fn binary_and_unary_factories_preserve_input_positions() {
        let lhs = set(&[1, 2]);
        let rhs = set(&[2, 3]);
        let difference_out = set_output();
        let difference = SetDifferenceFxn::new_invocation(
            FunctionArgs::Binary(difference_out.to_value(), lhs.to_value(), rhs.to_value()).into(),
        )
        .unwrap();
        difference.solve_result().unwrap();
        assert_eq!(*difference_out.borrow(), *set(&[1]).borrow());

        let powerset_out = set_output();
        let powerset = SetPowersetFxn::new_invocation(
            FunctionArgs::Unary(powerset_out.to_value(), lhs.to_value()).into(),
        )
        .unwrap();
        lhs.borrow_mut()
            .set
            .insert(LegacyValue::Index(Ref::new(3)));
        powerset.solve_result().unwrap();
        assert_eq!(powerset_out.borrow().set.len(), 8);
    }

    #[test]
    fn typed_factories_reject_wrong_types_and_layouts() {
        let out = set_output();
        let rhs = set(&[1]);
        let wrong_type = factory_error(SetUnionFxn::new_invocation(
            FunctionArgs::Binary(
                out.to_value(),
                LegacyValue::Bool(Ref::new(true)),
                rhs.to_value(),
            )
            .into(),
        ));
        assert_eq!(wrong_type.kind_name(), "FunctionArgumentTypeMismatch");

        let wrong_binary_layout = factory_error(SetUnionFxn::new_invocation(
            FunctionArgs::Unary(out.to_value(), rhs.to_value()).into(),
        ));
        assert_eq!(wrong_binary_layout.kind_name(), "IncorrectNumberOfArguments");
        let wrong_binary_layout = wrong_binary_layout
            .kind_as::<IncorrectNumberOfArguments>()
            .unwrap();
        assert_eq!((wrong_binary_layout.expected, wrong_binary_layout.found), (2, 1));

        let size_out = Ref::new(0_u64);
        let wrong_unary_layout = factory_error(SetSizeFxn::new_invocation(
            FunctionArgs::Binary(size_out.to_value(), rhs.to_value(), rhs.to_value()).into(),
        ));
        assert_eq!(wrong_unary_layout.kind_name(), "IncorrectNumberOfArguments");
        let wrong_unary_layout = wrong_unary_layout
            .kind_as::<IncorrectNumberOfArguments>()
            .unwrap();
        assert_eq!((wrong_unary_layout.expected, wrong_unary_layout.found), (1, 2));
    }

    #[test]
    fn typed_factories_do_not_traverse_legacy_wrappers() {
        let out = set_output();
        let lhs = set(&[1]);
        let rhs = set(&[2]);
        let typed = LegacyValue::Typed(
            Box::new(lhs.to_value()),
            ValueKind::Set(Box::new(ValueKind::Index), Some(1)),
        );
        let typed_error = factory_error(SetUnionFxn::new_invocation(
            FunctionArgs::Binary(out.to_value(), typed, rhs.to_value()).into(),
        ));
        assert_eq!(typed_error.kind_name(), "FunctionArgumentTypeMismatch");

        let mutable = LegacyValue::MutableReference(Ref::new(lhs.to_value()));
        let mutable_error = factory_error(SetUnionFxn::new_invocation(
            FunctionArgs::Binary(out.to_value(), mutable, rhs.to_value()).into(),
        ));
        assert_eq!(mutable_error.kind_name(), "FunctionArgumentTypeMismatch");
    }

    #[test]
    fn function_invocation_keeps_the_exact_argument_layout() {
        let out = set_output();
        let lhs = set(&[1]);
        let rhs = set(&[2]);
        let invocation: FunctionInvocation =
            FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value()).into();
        let (_, first, second) = invocation.expect_binary().unwrap();

        assert!(first.try_ref::<MechSet>().unwrap().same_handle(&lhs));
        assert!(second.try_ref::<MechSet>().unwrap().same_handle(&rhs));
    }
}
