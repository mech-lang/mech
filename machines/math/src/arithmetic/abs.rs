use crate::*;

// Abs ------------------------------------------------------------------------

#[cfg(feature = "f64")]
use libm::fabs;
#[cfg(feature = "f32")]
use libm::fabsf;

#[cfg(any(
    feature = "u8",
    feature = "u16",
    feature = "u32",
    feature = "u64",
    feature = "u128"
))]
macro_rules! uabs_op {
    (@managed $arg:expr) => {
        Ok(($arg))
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
    (@managed $arg:expr) => {
        checked_abs_value($arg)
    };
}

#[cfg(any(feature = "c64", feature = "r64"))]
macro_rules! abs_op {
    (@managed $arg:expr) => {
        Ok(($arg).abs())
    };
}

#[cfg(feature = "f64")]
macro_rules! fabs_op {
    (@managed $arg:expr) => {
        Ok(fabs(($arg)))
    };
}

#[cfg(feature = "f32")]
macro_rules! fabsf_op {
    (@managed $arg:expr) => {
        Ok(fabsf(($arg)))
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

    fn assert_snapshot_eq(actual: &ValueCell, expected: &Value) {
        let actual = actual.snapshot().unwrap();
        assert_eq!(actual.schema_key(), expected.schema_key());
        assert_eq!(actual.shape(), expected.shape());
        match (actual.data(), expected.data()) {
            (ValueData::I8(actual), ValueData::I8(expected)) => assert_eq!(actual, expected),
            #[cfg(feature = "matrixd")]
            (ValueData::Matrix(actual), ValueData::Matrix(expected)) => {
                let (snapshot::SequenceView::I8(actual), snapshot::SequenceView::I8(expected)) =
                    (actual.elements(), expected.elements())
                else {
                    panic!("expected I8 matrix")
                };
                assert_eq!(actual, expected);
            }
            _ => panic!("unexpected absolute-value result"),
        }
    }

    #[test]
    fn signed_scalar_abs_rejects_minimum_and_retains_output() {
        let arg = ValueCell::from_exact(7_i8).unwrap();
        let out = ValueCell::from_exact(19_i8).unwrap();
        let function = crate::catalog::bind_test_unary::<MathAbsI8S>(
            "math/abs",
            "MathAbsI8S",
            arg.clone(),
            out.clone(),
        );

        function.instance().solve_result().unwrap();
        let expected = ValueCell::from_exact(7_i8).unwrap().snapshot().unwrap();
        assert_snapshot_eq(&out, &expected);
        arg.replace(&ValueCell::from_exact(i8::MIN).unwrap().snapshot().unwrap())
            .unwrap();

        let version = out.published_version();
        let error = function.instance().solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "MathArithmeticOverflow");
        assert_snapshot_eq(&out, &expected);
        assert_eq!(out.published_version(), version);
    }

    #[cfg(feature = "matrixd")]
    #[test]
    fn signed_matrix_abs_is_transactional_when_any_element_is_minimum() {
        let arg = ValueCell::from_exact(DMatrix::from_row_slice(2, 3, &[-2_i8, 3, -4, 5, -6, 7]))
            .unwrap();
        let out = ValueCell::from_exact(DMatrix::from_element(2, 3, 0_i8)).unwrap();
        let function = crate::catalog::bind_test_unary::<MathAbsI8MD>(
            "math/abs",
            "MathAbsI8MD",
            arg.clone(),
            out.clone(),
        );
        function.instance().solve_result().unwrap();
        let expected = ValueCell::from_exact(DMatrix::from_row_slice(2, 3, &[2_i8, 3, 4, 5, 6, 7]))
            .unwrap()
            .snapshot()
            .unwrap();
        assert_snapshot_eq(&out, &expected);
        for position in [0, 2, 5] {
            let mut values = [-4_i8; 6];
            values[position] = i8::MIN;
            arg.replace(
                &ValueCell::from_exact(DMatrix::from_row_slice(2, 3, &values))
                    .unwrap()
                    .snapshot()
                    .unwrap(),
            )
            .unwrap();
            let version = out.published_version();
            let error = function.instance().solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "MathArithmeticOverflow");
            assert_snapshot_eq(&out, &expected);
            assert_eq!(out.published_version(), version);
        }
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
        let specialized = crate::catalog::specialize_test_operation(
            &catalog,
            "math/abs",
            vec![ValueCell::from_exact(-3.0_f32).unwrap()],
        );

        assert!(
            specialized
                .instance()
                .implementation()
                .to_string()
                .starts_with("MathAbsF32S")
        );
        specialized.instance().solve_result().unwrap();
        let output = specialized.output().snapshot().unwrap();
        let ValueData::F32(output) = output.data() else {
            panic!("expected the exact f32 absolute-value output")
        };
        assert_eq!(output.to_f32().to_bits(), 3.0_f32.to_bits());
    }
}
