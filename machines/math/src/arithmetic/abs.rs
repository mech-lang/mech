use crate::*;

// Abs ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::fabs;
#[cfg(feature = "f32")]
use libm::fabsf;

#[cfg(any(feature = "u8", feature = "u16", feature = "u32", feature = "u64", feature = "u128"))]
macro_rules! uabs_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = (*$arg).clone();
        }
    };
}

#[cfg(any(feature = "u8", feature = "u16", feature = "u32", feature = "u64", feature = "u128"))]
macro_rules! uabs_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                (&mut (*$out))[i] = (&(*$arg))[i].clone();
            }
        }
    };
}

#[cfg(any(
    feature = "i8",
    feature = "i16",
    feature = "i32",
    feature = "i64",
    feature = "i128"
))]
trait RuntimeCheckedAbs: Copy {
    fn runtime_checked_abs(self) -> Option<Self>;
}

#[cfg(any(
    feature = "i8",
    feature = "i16",
    feature = "i32",
    feature = "i64",
    feature = "i128"
))]
macro_rules! impl_runtime_checked_abs {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RuntimeCheckedAbs for $type {
                fn runtime_checked_abs(self) -> Option<Self> {
                    self.checked_abs()
                }
            }
        )+
    };
}

#[cfg(any(
    feature = "i8",
    feature = "i16",
    feature = "i32",
    feature = "i64",
    feature = "i128"
))]
impl_runtime_checked_abs!(i8, i16, i32, i64, i128);

#[cfg(any(
    feature = "i8",
    feature = "i16",
    feature = "i32",
    feature = "i64",
    feature = "i128"
))]
fn checked_abs_value<T: RuntimeCheckedAbs>(value: T) -> MResult<T> {
    value
        .runtime_checked_abs()
        .ok_or_else(|| arithmetic_overflow::<T>("absolute value"))
}

#[cfg(any(
    feature = "i8",
    feature = "i16",
    feature = "i32",
    feature = "i64",
    feature = "i128"
))]
macro_rules! checked_abs_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            let next = checked_abs_value(*$arg)?;
            *$out = next;
        }
    };
}

#[cfg(any(
    feature = "i8",
    feature = "i16",
    feature = "i32",
    feature = "i64",
    feature = "i128"
))]
macro_rules! checked_abs_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            let mut next = (*$arg).clone();
            for value in next.iter_mut() {
                *value = checked_abs_value(*value)?;
            }
            *$out = next;
        }
    };
}

#[cfg(any(feature = "c64", feature = "r64"))]
macro_rules! abs_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = (*$arg).abs();
        }
    };
}

#[cfg(any(feature = "c64", feature = "r64"))]
macro_rules! abs_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                (&mut (*$out))[i] = (&(*$arg))[i].abs();
            }
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! fabs_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = fabs((*$arg));
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! fabs_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = fabs(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! fabsf_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            (*$out) = fabsf((*$arg));
        }
    };
}

#[cfg(feature = "f32")]
macro_rules! fabsf_vec_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$arg).len() {
                ((&mut (*$out))[i]) = fabsf(((&(*$arg))[i]));
            }
        }
    };
}

#[cfg(feature = "u8")]
impl_math_unop!(MathAbs, u8, uabs);
#[cfg(feature = "u16")]
impl_math_unop!(MathAbs, u16, uabs);
#[cfg(feature = "u32")]
impl_math_unop!(MathAbs, u32, uabs);
#[cfg(feature = "u64")]
impl_math_unop!(MathAbs, u64, uabs);
#[cfg(feature = "u128")]
impl_math_unop!(MathAbs, u128, uabs);

#[cfg(feature = "i8")]
impl_math_unop!(MathAbs, i8, checked_abs);
#[cfg(feature = "i16")]
impl_math_unop!(MathAbs, i16, checked_abs);
#[cfg(feature = "i32")]
impl_math_unop!(MathAbs, i32, checked_abs);
#[cfg(feature = "i64")]
impl_math_unop!(MathAbs, i64, checked_abs);
#[cfg(feature = "i128")]
impl_math_unop!(MathAbs, i128, checked_abs);

#[cfg(feature = "f32")]
impl_math_unop!(MathAbs, f32, fabsf);
#[cfg(feature = "f64")]
impl_math_unop!(MathAbs, f64, fabs);

#[cfg(feature = "c64")]
impl_math_unop!(MathAbs, C64, abs);

#[cfg(feature = "r64")]
impl_math_unop!(MathAbs, R64, abs);

#[cfg(all(test, feature = "i8"))]
mod checked_abs_tests {
    use super::*;

    #[test]
    fn signed_scalar_abs_rejects_minimum_and_retains_output() {
        let arg = Ref::new(7_i8);
        let out = Ref::new(19_i8);
        let function = MathAbsI8S {
            arg: arg.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();
        assert_eq!(*out.borrow(), 7);
        *arg.borrow_mut() = i8::MIN;

        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "MathArithmeticOverflow");
        assert_eq!(*out.borrow(), 7);
    }

    #[cfg(feature = "matrixd")]
    #[test]
    fn signed_matrix_abs_is_transactional_when_any_element_is_minimum() {
        let arg = Ref::new(DMatrix::from_row_slice(1, 2, &[-2_i8, 3]));
        let out = Ref::new(DMatrix::from_row_slice(1, 2, &[11_i8, 12]));
        let function = MathAbsI8MD {
            arg: arg.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();
        assert_eq!(&*out.borrow(), &DMatrix::from_row_slice(1, 2, &[2, 3]));
        *arg.borrow_mut() = DMatrix::from_row_slice(1, 2, &[-4, i8::MIN]);

        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "MathArithmeticOverflow");
        assert_eq!(&*out.borrow(), &DMatrix::from_row_slice(1, 2, &[2, 3]));
    }
}

impl_canonical_registered_math_unop_specializer!(MathAbs, "MathAbs");

#[cfg(all(test, feature = "source", feature = "f32"))]
mod canonical_source_tests {
    use super::*;

    #[test]
    fn f32_abs_binds_the_registered_runtime_factory() {
        let mut builder = FunctionCatalogBuilder::new();
        crate::catalog::register_math_abs_f32_s(&mut builder).unwrap();
        crate::catalog::install_canonical_source_specializer(
            &mut builder,
            "math/abs",
            Some("math"),
            Some("abs"),
            FunctionExposure::ModuleOnly,
            crate::MathAbs {},
        )
        .unwrap();
        let catalog = builder.build().unwrap();
        let invocation = SpecializationInvocation::from_cells(
            vec![ValueCell::from_exact(-3.0_f32).unwrap()].into_boxed_slice(),
        );
        let mut context =
            SpecializationContext::for_invocation(&invocation, Some(&catalog)).unwrap();

        let specialized = catalog
            .specializer(OperationId::from_name("math/abs"))
            .unwrap()
            .specializer
            .specialize_invocation(&invocation, &mut context)
            .unwrap();

        assert!(
            specialized
                .instance()
                .implementation()
                .to_string()
                .starts_with("MathAbsF32S")
        );
        specialized
            .instance()
            .implementation()
            .solve_result()
            .unwrap();
        let output = specialized.output().snapshot().unwrap();
        let ValueData::F32(output) = output.data() else {
            panic!("expected the exact f32 absolute-value output")
        };
        assert_eq!(output.to_f32().to_bits(), 3.0_f32.to_bits());
    }
}
